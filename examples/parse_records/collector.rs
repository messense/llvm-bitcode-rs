use crate::metadata::MetadataNamedNode;
use llvm_bitcode::Cursor;
use llvm_bitcode::bitcode::RecordIter;
use llvm_bitcode::read::{BlockItem, BlockIter, Error};
use llvm_bitcode::schema::blocks::*;
use llvm_bitcode::schema::records::*;
use llvm_bitcode::schema::values::*;
use std::collections::HashMap;
use std::ops::Range;
use std::sync::Arc;

#[derive(Debug, Default)]
struct Attributes {
    function: Vec<Attribute>,
    ret: Vec<Attribute>,
    arg: Vec<Vec<Attribute>>,
}

#[derive(Debug)]
pub struct BcCollector<'a> {
    strtab: &'a [u8],
    symtab_blob: &'a [u8],
    /// software writing the bitcode
    identification: Option<String>,
    types: Types,
    /// Shared mutable base data for LLVM's value list
    global_value_list: GlobalList<Value>,
    operand_bundle_tags: Vec<Vec<String>>,
    sync_scope_names: Vec<Vec<String>>,

    attribute_groups: HashMap<ParamAttrGroupId, Attributes>,
    attributes: Vec<Vec<ParamAttrGroupId>>,

    pub global_metadata: GlobalList<Arc<MetadataRecord>>,
    pub metadata_node_names: Vec<MetadataNamedNode>,
    /// Mapping of metadata kind IDs to their names
    metadata_kinds: HashMap<MetadataKind, String>,
    /// Parsed functions
    pub functions: Vec<Function>,
    /// Temporary - functions defined in the module (not just declarations)
    module_defined_functions: Vec<ModuleFunctionRecord>,
    pub function_prototypes: Vec<ModuleFunctionRecord>,
    /// Cached i32 type ID to avoid repeated lookups
    cached_i32_type_id: Option<TypeId>,
}

/// An instruction within a basic block
#[derive(Debug)]
pub struct BBInstruction {
    /// Instruction index for metadata attachment
    pub index: InstIndex,
    /// Optional value ID if the instruction produces a value
    pub value_id: Option<ValueId>,
    /// The actual instruction
    pub inst: Inst,
}

/// Debug instruction within a basic block
#[derive(Debug)]
pub struct BBDebugInstruction {
    /// Instruction index for metadata attachment
    pub index: InstIndex,
    pub di: DebugInstruction,
}

/// A basic block within a function
#[derive(Debug)]
pub struct BasicBlock {
    /// Optional name of the basic block
    pub name: Option<String>,
    pub instructions: Vec<BBInstruction>,
}

#[derive(Debug)]
pub struct LocalList<T> {
    pub first: usize,
    pub data: Vec<T>,
}

impl<T> LocalList<T> {
    fn len(&self) -> usize {
        self.first + self.data.len()
    }
}

#[derive(Debug)]
pub struct GlobalList<T> {
    pub data: Vec<T>,
}

impl<T> GlobalList<T> {
    #[must_use]
    pub fn get<'a>(&'a self, local: &'a LocalList<T>, id: usize) -> Option<&'a T> {
        if let Some(local_id) = id.checked_sub(local.first) {
            local.data.get(local_id)
        } else {
            self.data.get(id)
        }
    }

    #[must_use]
    pub fn fork(&self) -> LocalList<T> {
        LocalList {
            first: self.data.len(),
            data: vec![],
        }
    }

    fn new() -> Self {
        Self { data: vec![] }
    }
}

/// Function representation from the bitcode
#[derive(Debug)]
pub struct Function {
    /// Function record from the module block
    pub record: ModuleFunctionRecord,
    /// `ValIDs` lower than function's start are looked up in the global value list,
    /// and higher ones are in the local value list.
    local_value_list: LocalList<Value>,
    /// Same as `value_list` but for metadata
    ///
    /// LLVM resets these in incorporateFunctionMetadata
    pub local_metadata: LocalList<Arc<MetadataRecord>>,

    pub basic_blocks: Vec<BasicBlock>,

    // inlined from all BBs, sorted by instruction index
    pub debug_metadata: Vec<BBDebugInstruction>,

    /// This is incremented for all instructions, even `Void` ones.
    /// Debug metadata does not increment this.
    instruction_counter: InstIndex,
    /// Metadata attachments to instructions
    pub inst_metadata_attachment: Vec<(InstIndex, Vec<(MetadataKind, Arc<MetadataRecord>)>)>,
    /// Metadata attachments to the function
    pub fn_metadata_attachment: Vec<(MetadataKind, Arc<MetadataRecord>)>,
}

impl Function {
    /// Add a value to the function's local value list and return its ID
    pub fn push_value_list(&mut self, value: Value) -> ValueId {
        let inserted_id = self.next_value_id();
        self.local_value_list.data.push(value);
        inserted_id
    }

    /// In LLVM this is `InstID` variable, but it's not the same counter as instruction IDs for purpose of metadata attachments.
    #[must_use]
    pub fn next_value_id(&self) -> ValueId {
        ValueId(self.local_value_list.len() as u32)
    }

    /// In LLVM this is `InstID` variable, but it's not the same counter as instruction IDs for purpose of metadata attachments.
    #[must_use]
    pub fn next_metadata_node_id(&self) -> usize {
        self.local_metadata.len()
    }

    // /// Look up a value in the function's local value list
    // pub fn local_value_by_id(&self, id: ValueId) -> Option<&Value> {
    //     let local_id = id.0.checked_sub(self.local_value_list.first)?;
    //     self.local_value_list.get(local_id as usize)
    // }

    // /// Look up a metadata node in the function's local metadata list
    // pub fn local_metadata_by_id(&self, id: MetadataNodeId) -> Option<&MetadataRecord> {
    //     let local_id = id.checked_sub(self.first_local_metadata_list_id)?;
    //     self.local_metadata.get(local_id as usize)
    // }
}

#[derive(Debug)]
pub struct Value {
    /// It really only matters if it's void
    pub type_id: TypeId,
    pub numeric_value: Option<i64>,
    pub name: Option<Range<usize>>,
}

impl<'input> BcCollector<'input> {
    /// Create a new parser
    #[must_use]
    pub fn new() -> Self {
        Self {
            functions: Vec::new(),
            identification: None,
            module_defined_functions: Vec::new(),
            function_prototypes: Vec::new(),
            global_value_list: GlobalList::new(),
            strtab: &[],
            symtab_blob: &[],
            types: Types { types: Vec::new() },
            global_metadata: GlobalList::new(),
            metadata_node_names: Vec::new(),
            attribute_groups: HashMap::new(),
            attributes: Vec::new(),
            operand_bundle_tags: Vec::new(),
            sync_scope_names: Vec::new(),
            metadata_kinds: HashMap::new(),
            cached_i32_type_id: None,
        }
    }

    /// Find and cache the i32 type ID
    fn find_i32_type_id(&mut self) -> Result<TypeId, Error> {
        if let Some(id) = self.cached_i32_type_id {
            return Ok(id);
        }

        for (i, t) in self.types.types.iter().enumerate() {
            if matches!(t, Type::Integer { width } if width.get() == 32) {
                let id = i as TypeId;
                self.cached_i32_type_id = Some(id);
                return Ok(id);
            }
        }

        Err(Error::Other("i32 type not found in type table"))
    }

    /// Iterate through a block and its nested blocks
    pub fn iter_outer_block(
        &mut self,
        mut outer_block: BlockIter<'_, 'input>,
    ) -> Result<(), Error> {
        while let Some(item) = outer_block.next()? {
            match item {
                BlockItem::Block(mut block) => {
                    let block_id = block.id;
                    match BlockId::try_from(block.id as u8)
                        .map_err(|_| Error::UnexpectedBlock(block_id))?
                    {
                        BlockId::Module => {
                            self.parse_module_block(block)?;
                        }
                        BlockId::Identification => {
                            while let Some(mut record) = block.next_record()? {
                                match IdentificationCode::try_from(record.id as u8).map_err(
                                    |_| Error::UnexpectedRecord {
                                        block_id,
                                        record_id: record.id,
                                    },
                                )? {
                                    IdentificationCode::String => {
                                        self.identification = record.string_utf8().ok();
                                    }
                                    IdentificationCode::Epoch => {
                                        if record.u32()? != 0 {
                                            return Err(Error::Other("expected epoch 0"));
                                        }
                                    }
                                    _ => {
                                        return Err(Error::UnexpectedRecord {
                                            block_id,
                                            record_id: record.id,
                                        });
                                    }
                                }
                            }
                        }
                        BlockId::Symtab => {
                            while let Some(mut record) = block.next_record()? {
                                self.symtab_blob = record.blob()?;
                            }
                        }
                        BlockId::Strtab => {
                            while let Some(mut record) = block.next_record()? {
                                self.strtab = record.blob()?;
                            }
                        }
                        other => unimplemented!("{other:?}"),
                    }
                }
                BlockItem::Record(_) => {
                    return Err(Error::Other("unexpected top-level record"));
                }
            }
        }
        Ok(())
    }

    fn parse_constants_block(
        &mut self,
        mut block: BlockIter<'_, 'input>,
        mut func: Option<&mut Function>,
    ) -> Result<(), Error> {
        // Defaults to i32 in LLVM
        let mut constants_current_type = self.find_i32_type_id()?;
        let block_id = block.id;

        while let Some(mut record) = block.next_record()? {
            let id = ConstantsCodes::try_from(record.id as u8)
                .map_err(|_| Error::UnexpectedRecord { block_id, record_id: record.id })?;

            let mut numeric_value = None;

            let con = match id {
                ConstantsCodes::Settype => {
                    let tyid = record.u32()?;
                    constants_current_type = tyid;
                    continue;
                }
                ConstantsCodes::Null => ConstantRecord::ConstantNull(constants_current_type),
                ConstantsCodes::Undef => ConstantRecord::ConstantUndef,
                ConstantsCodes::Integer => {
                    let value = record.i64()?;
                    numeric_value = Some(value);
                    ConstantRecord::ConstantInteger(ConstantInteger { ty: constants_current_type, value })
                },
                ConstantsCodes::WideInteger => ConstantRecord::ConstantWideInteger(ConstantWideInteger {
                    ty: constants_current_type,
                    values: record.collect::<Result<Vec<_>, _>>()?,
                }),
                ConstantsCodes::Float => ConstantRecord::ConstantFloat(ConstantFloat {
                    ty: constants_current_type,
                    value: f64::from_bits(record.u64()?),
                }),
                ConstantsCodes::Aggregate => ConstantRecord::ConstantAggregate(ConstantAggregate {
                    ty: constants_current_type,
                    values: record.collect::<Result<Vec<_>, _>>()?,
                }),
                ConstantsCodes::String | ConstantsCodes::Data => ConstantRecord::ConstantString(ConstantString {
                    ty: constants_current_type,
                    value: record.string()?,
                }),
                ConstantsCodes::CString => ConstantRecord::ConstantCString(ConstantCString {
                    ty: constants_current_type,
                    value: record.string()?,
                }),
                ConstantsCodes::BinOp => ConstantRecord::ConstantBinaryOp(ConstantBinaryOp {
                    ty: constants_current_type,
                    opcode: record.try_from::<u8, _>()?,
                    lhs: ValueId(record.u32()?),
                    rhs: ValueId(record.u32()?),
                    flags: record.next()?.unwrap_or(0) as u8,
                }),
                ConstantsCodes::Cast => ConstantRecord::ConstantCast(ConstantCast {
                    opcode: record.try_from::<u8, _>()?,
                    ty: record.u32()?,
                    operand: ValueId(record.u32()?),
                }),
                id @ (ConstantsCodes::Gep | ConstantsCodes::GepWithInrange | ConstantsCodes::InboundsGep) => {
                    let base_type = record.u32()?;
                    let flags = if matches!(id, ConstantsCodes::InboundsGep) {
                        1 /*GEP_INBOUNDS*/
                    } else {
                        record.u8()?
                    };
                    let inrange = if id == ConstantsCodes::GepWithInrange {
                        let bitwidth = record.u32()?;
                        if bitwidth > 64 {
                            unimplemented!();
                        }
                        Some(record.i64()?..record.i64()?)
                    } else {
                        None
                    };
                    let mut operands = Vec::with_capacity(record.len() / 2);
                    while record.len() >= 2 {
                        operands.push((record.u32()?, ValueId(record.u32()?)));
                    }
                    ConstantRecord::ConstantGEP(ConstantGEP {
                        ty: constants_current_type,
                        base_type,
                        flags,
                        inrange,
                        operands,
                    })
                }
                ConstantsCodes::Select => ConstantRecord::ConstantSelect(ConstantSelect {
                    ty: constants_current_type,
                    condition: record.u64()?,
                    true_value: record.u64()?,
                    false_value: record.u64()?,
                }),
                ConstantsCodes::ExtractElt => ConstantRecord::ConstantExtractElement(ConstantExtractElement {
                    operand_ty: record.u32()?,
                    operand_val: ValueId(record.u32()?),
                    index_ty: record.u32()?,
                    index_val: ValueId(record.u32()?),
                }),
                ConstantsCodes::InsertElt => ConstantRecord::ConstantInsertElement(ConstantInsertElement {
                    ty: constants_current_type,
                    operand_type: record.u32()?,
                    vector: record.u64()?,
                    element: record.u64()?,
                    index: record.u64()?,
                }),
                ConstantsCodes::ShuffleVec => ConstantRecord::ConstantShuffleVector(ConstantShuffleVector {
                    ty: constants_current_type,
                    vector1: record.u64()?,
                    vector2: record.u64()?,
                    mask: record.u64()?,
                }),
                ConstantsCodes::Cmp => ConstantRecord::ConstantCompare(ConstantCompare {
                    ty: constants_current_type,
                    operand_type: record.u32()?,
                    lhs: record.u64()?,
                    rhs: record.u64()?,
                    predicate: record.u8()?,
                }),
                ConstantsCodes::BlockAddress => {
                    ConstantRecord::ConstantBlockAddress(ConstantBlockAddress {
                        ty: constants_current_type,
                        function: ValueId(record.u32()?),
                        block: record.u64()?, // getGlobalBasicBlockID
                    })
                }
                ConstantsCodes::InlineAsm => ConstantRecord::ConstantInlineASM(ConstantInlineASM {
                    ty: constants_current_type,
                    function_type: record.u32()?,
                    flags: record.u8()?,
                    asm: record.string_utf8()?,
                    constraints: record.string_utf8()?,
                }),
                ConstantsCodes::Poison => ConstantRecord::ConstantPoison,
                ConstantsCodes::DsoLocalEquivalent => {
                    ConstantRecord::ConstantDSOLocalEquivalent(ConstantDSOLocalEquivalent {
                        ty: constants_current_type,
                        gv_type: record.u32()?,
                        gv: record.u64()?,
                    })
                }
                ConstantsCodes::NoCfiValue => ConstantRecord::ConstantNoCFI(ConstantNoCFI {
                    ty: constants_current_type,
                    function_type: record.u32()?,
                    function: record.u64()?,
                }),
                ConstantsCodes::PtrAuth => ConstantRecord::ConstantPtrAuth(ConstantPtrAuth {
                    ty: constants_current_type,
                    pointer: record.u64()?,
                    key: record.u64()?,
                    discriminator: record.u64()?,
                    address_discriminator: record.u64()?,
                }),
                ConstantsCodes::ShufVecEx => todo!(),
                ConstantsCodes::UnOp => todo!(),
                other => unimplemented!("{other:?} constant"),
            };

            let type_id = con.get_type_id().unwrap_or_else(|| {
                // Find void type if not specified
                for (i, t) in self.types.types.iter().enumerate() {
                    if matches!(t, Type::Void) {
                        return i as TypeId;
                    }
                }
                // This should not happen if the module is well-formed
                panic!("Void type not found in type table");
            });

            if let Some(func) = &mut func {
                func.push_value_list(Value { numeric_value, type_id, name: None });
            } else {
                self.global_value_list.data.push(Value { numeric_value, type_id, name: None });
            }
        }
        Ok(())
    }

    fn parse_function_block(&mut self, mut block: BlockIter<'_, 'input>) -> Result<(), Error> {
        if self.module_defined_functions.is_empty() {
            return Err(Error::Other("No defined functions to parse"));
        }

        let func = self.module_defined_functions.remove(0);

        let mut func = Function {
            record: func,
            local_value_list: self.global_value_list.fork(),
            local_metadata: self.global_metadata.fork(),
            instruction_counter: 0,
            debug_metadata: Vec::new(),
            fn_metadata_attachment: Vec::new(),

            inst_metadata_attachment: Vec::new(),
            basic_blocks: vec![],
        };

        self.incorporate_function(&mut func)?;

        // If true, stop appending new instructions to the last BB, but may append metadata
        let mut last_basic_block_terminated = true;
        let mut last_debug_loc = DebugLoc {
            line: 0,
            column: 0,
            scope: None,
            inlined_at: None,
            implicit_code: false,
        };
        let mut vst = Vec::new();
        'block_or_record: while let Some(b) = block.next()? {
            match b {
                BlockItem::Block(mut block) => match BlockId::try_from(block.id as u8) {
                    Ok(BlockId::Constants) => {
                        self.parse_constants_block(block, Some(&mut func))?;
                    }
                    Ok(BlockId::Metadata) => {
                        self.parse_metadata_block(block, Some(&mut func))?;
                    }
                    Ok(BlockId::MetadataAttachment) => {
                        self.parse_metadata_attachment(block, &mut func)?;
                    }
                    Ok(BlockId::ValueSymtab) => {
                        while let Some(record) = block.next_record()? {
                            vst.push(self.parse_value_symtab_record(record)?);
                        }
                    }
                    Ok(BlockId::Uselist) => {
                        // unimplemented, uninteresting
                    }
                    _ => {
                        debug_assert!(false, "function block id={}", block.id);
                        return Err(Error::UnexpectedBlock(block.id));
                    }
                },
                BlockItem::Record(record) => {
                    let Some(r) = self.parse_function_record(record, &mut func, &mut last_debug_loc)? else {
                        continue 'block_or_record;
                    };
                    match r {
                        FunctionRecord::FunctionInst(inst) => {
                            // Avoids creating empty BB after last terminator
                            // (and in LLVM, debug info meta is in BBs, *after* their instructions)
                            if last_basic_block_terminated {
                                func.basic_blocks.push(BasicBlock { name: None, instructions: vec![] });
                            }
                            last_basic_block_terminated = inst.is_terminator();

                            let index = func.instruction_counter;
                            func.instruction_counter += 1;

                            let value_id = if !inst.is_void_type(&self.types) {
                                let inst_type_id = inst
                                    .ret_type_id(&self.types)
                                    .ok_or(Error::Other("Invalid instruction return type"))?;
                                Some(func.push_value_list(Value {
                                    numeric_value: None,
                                    type_id: inst_type_id,
                                    name: None,
                                }))
                            } else {
                                None
                            };

                            let Some(bb) = func.basic_blocks.last_mut() else {
                                debug_assert!(false);
                                return Ok(());
                            };

                            bb.instructions.push(BBInstruction {
                                index,
                                value_id,
                                inst,
                            });
                        }
                        FunctionRecord::FunctionDI(meta) => {
                            if !matches!(
                                meta,
                                DebugInstruction::Loc(_)
                                    | DebugInstruction::RecordValue(_)
                                    | DebugInstruction::RecordDeclare(_)
                                    | DebugInstruction::RecordValueSimple(_)
                            ) {
                                // TODO: cache this?
                                let metadata_type_id = self
                                    .types
                                    .types
                                    .iter()
                                    .position(|t| matches!(t, Type::Metadata))
                                    .ok_or(Error::Other("Metadata type not found"))?
                                    as TypeId;

                                func.push_value_list(Value {
                                    numeric_value: None, // what about metadata as value?
                                    type_id: metadata_type_id,
                                    name: None,
                                });
                            }
                            // debug applies to the previous instruction, and isn't an instruction itself
                            let index = func.instruction_counter - 1;
                            func.debug_metadata.push(BBDebugInstruction { index, di: meta });
                        },
                        _ => {},
                    }
                }
            }
        }

        func.inst_metadata_attachment.sort_unstable_by_key(|a| a.0);
        self.process_vst(vst, Some(&mut func))?;
        self.functions.push(func);

        Ok(())
    }

    fn incorporate_function(&mut self, func: &mut Function) -> Result<(), Error> {
        let fty = self.types.get_fn(func.record.ty).ok_or(Error::Other("Invalid function type"))?;

        func.local_value_list.data.extend(fty.param_types.iter().map(|&type_id| Value {
            numeric_value: None,
            type_id,
            name: None,
        }));

        // constants with operands are enumerated recursively here?
        // basic block IDs are added here?
        Ok(())
    }

    /// pushValueAndType/getValueTypePair equivalent
    fn value_and_type<'cursor>(
        &mut self,
        record: &mut RecordIter<'cursor, 'input>,
        func: &mut Function,
    ) -> Result<(ValueId, TypeId), Error> {
        self.value_and_type_from_iter(record, func)
    }

    /// pushValueAndType/getValueTypePair equivalent
    fn value_and_type_from_iter(
        &mut self,
        iter: &mut impl Iterator<Item = Result<u64, Error>>,
        func: &mut Function,
    ) -> Result<(ValueId, TypeId), Error> {
        let relative_val_id = iter.next().ok_or(Error::EndOfRecord)??;

        if relative_val_id == 0x80000000 {
            // OB_METADATA
            unimplemented!()
        }

        // IDs are i32 and can be negative for forward refs
        let relative_val_id = u32::try_from(relative_val_id).map_err(|_| Error::ValueOverflow)? as i32;

        let next_value_id = func.next_value_id().0;

        // Calculate the absolute value ID from the relative ID
        let val_id = (next_value_id as i32 - relative_val_id) as u32;
        debug_assert!(val_id < (1 << 30), "bad id {relative_val_id} > expected {next_value_id}");

        // Forward references to values that haven't been processed yet need a type
        let ty = if val_id >= next_value_id {
            iter.next().ok_or(Error::EndOfRecord)??.try_into().map_err(|_| Error::ValueOverflow)?
        } else {
            let v = self.global_value_list.get(&func.local_value_list, val_id as usize)
                .ok_or_else(|| {
                    debug_assert!(false, "bad id {val_id}; globals have {}; func starts {}", self.global_value_list.data.len(), func.local_value_list.first);
                    Error::Other("bug: relative IDs got out of sync")
                })?;
            // For backward references to global values, use the stored type if available
            v.type_id
        };

        Ok((ValueId(val_id), ty))
    }

    // pushValue/popValue equivalent
    #[track_caller]
    fn value_without_type<'cursor>(
        record: &mut RecordIter<'cursor, 'input>,
        func: &mut Function,
    ) -> Result<ValueId, Error> {
        // IDs are i32 and can be negative for forward refs
        let relative_val_id = record.u32()? as i32;
        let next_value_id = func.next_value_id().0;
        let val_id = (next_value_id as i32 - relative_val_id) as u32;
        debug_assert!(val_id < (1 << 30));
        Ok(ValueId(val_id))
    }

    /// Get a signed value
    fn value_signed<'cursor>(
        &mut self,
        record: &mut RecordIter<'cursor, 'input>,
        func: &mut Function,
    ) -> Result<ValueId, Error> {
        let relative_val_id = record.i64()?;
        let next_value_id = func.next_value_id().0;
        let val_id = u32::try_from(i64::from(next_value_id) - relative_val_id)
            .map_err(|_| Error::Other("bad IDs"))?;
        Ok(ValueId(val_id))
    }

    // `None` if strtab hasn't been loaded yet, the range is invalid, or non-UTF-8
    #[must_use]
    pub fn strtab(&self, range: &Range<usize>) -> Option<&str> {
        std::str::from_utf8(self.strtab.get(range.clone())?).ok()
    }

    fn parse_module_block(&mut self, mut outer_block: BlockIter<'_, 'input>) -> Result<(), Error> {
        let mut module = Module::default();

        while let Some(item) = outer_block.next()? {
            match item {
                BlockItem::Block(mut block) => {
                    let block_id = BlockId::try_from(block.id as u8)
                        .map_err(|_| Error::UnexpectedBlock(block.id))?;
                    match block_id {
                        BlockId::Constants => {
                            self.parse_constants_block(block, None)?;
                        }
                        BlockId::Function => {
                            self.parse_function_block(block)?;
                        }
                        BlockId::Type => self.parse_type_block(block)?,
                        BlockId::Metadata => self.parse_metadata_block(block, None)?,
                        BlockId::MetadataAttachment => unreachable!("no function"),
                        BlockId::MetadataKind => {
                            while let Some(mut record) = block.next_record()? {
                                if record.id == 6 {
                                    let md_kind: MetadataKind =
                                        MetadataKind::try_from(record.u8()?).unwrap_or(MetadataKind::Unknown);
                                    self.metadata_kinds.insert(md_kind, record.string_utf8()?);
                                }
                            }
                        }
                        BlockId::ParamAttr => {
                            while let Some(record) = block.next_record()? {
                                self.parse_attributes_record(record)?;
                            }
                        }
                        BlockId::ParamAttrGroup => {
                            while let Some(record) = block.next_record()? {
                                self.parse_param_attr_group_record(record)?;
                            }
                        }
                        BlockId::OperandBundleTags => {
                            let mut tags = vec![];
                            while let Some(mut record) = block.next_record()? {
                                tags.push(record.string_utf8()?);
                            }
                            self.operand_bundle_tags.push(tags);
                        }
                        BlockId::SyncScopeNames => {
                            let mut names = vec![];
                            while let Some(mut record) = block.next_record()? {
                                names.push(record.string_utf8()?);
                            }
                            self.sync_scope_names.push(names);
                        }
                        BlockId::ValueSymtab => {
                            // LLVM says:
                            // Specialized value symbol table parser used when reading module index
                            // blocks where we don't actually create global values. The parsed information
                            // is saved in the bitcode reader for use when later parsing summaries.
                            // With a strtab the VST is not required to parse the summary.

                            let mut vst = Vec::new();
                            while let Some(record) = block.next_record()? {
                                vst.push(self.parse_value_symtab_record(record)?);
                            }
                            self.process_vst(vst, None)?;
                        }
                        BlockId::Uselist => unimplemented!(),
                        BlockId::ModuleStrtab => unimplemented!(),
                        BlockId::GlobalvalSummary => {
                            // self.parse_global_value_summary(block)?;
                        }
                        BlockId::FullLtoGlobalvalSummary => unimplemented!(),
                        other => unimplemented!("{other:?}"),
                    }
                }
                BlockItem::Record(mut record) => {
                    let record_id = ModuleCode::try_from(record.id as u8)
                        .map_err(|_| Error::Other("Invalid module record code"))?;

                    match record_id {
                        ModuleCode::Version => {
                            let version = record.u64()?;
                            if version < 2 {
                                return Err(Error::Other("Bitcode format is too old. Must be at least v2"));
                            }
                            module.version = Some(ModuleVersionRecord { version });
                        },
                        ModuleCode::Triple => {
                            module.triple = Some(ModuleTripleRecord { triple: record.string_utf8()? });
                        },
                        ModuleCode::Datalayout => {
                            module.data_layout = Some(ModuleDataLayoutRecord { datalayout: record.string_utf8()? });
                        },
                        ModuleCode::Asm => {
                            module.asm = Some(ModuleAsmRecord { asm: record.string_utf8()? });
                        },
                        ModuleCode::SectionName => {
                            module.section_name = Some(ModuleSectionNameRecord { section_name: record.string_utf8()? });
                        },
                        ModuleCode::Deplib => {
                            module.dep_lib = Some(ModuleDepLibRecord { deplib_name: record.string_utf8()? });
                        },
                        ModuleCode::GlobalVar => {
                            let name = record.range()?;
                            let type_id = record.u32()?;
                            let flags = record.u32()?;
                            let init_id = record.nzu64()?;
                            let linkage = Linkage::try_from(record.u8()?)
                                .map_err(|_| Error::Other("Invalid linkage code"))?;
                            let alignment = record.nzu32()?;
                            let section = record.nzu32()?;

                            let var = if record.is_empty() {
                                ModuleGlobalVarRecord {
                                    name: name.clone(),
                                    type_id,
                                    flags,
                                    init_id,
                                    linkage,
                                    alignment,
                                    section,
                                    visibility: 0,
                                    thread_local: 0,
                                    unnamed_addr: None,
                                    dll_storage_class: DllStorageClass::Default,
                                    comdat: None,
                                    attributes: None,
                                    dso_local: false,
                                    global_sanitizer: None,
                                    partition: 0..0,
                                    code_model: 0,
                                }
                            } else {
                                ModuleGlobalVarRecord {
                                    name: name.clone(),
                                    type_id,
                                    flags,
                                    init_id,
                                    linkage,
                                    alignment,
                                    section,
                                    visibility: record.u8()?,
                                    thread_local: record.u8()?,
                                    unnamed_addr: record.nzu8()?,
                                    dll_storage_class: DllStorageClass::try_from(record.u8()?)
                                        .map_err(|_| Error::Other("Invalid DLL storage class"))?,
                                    comdat: record.nzu64()?,
                                    attributes: record.nzu32()?,
                                    dso_local: record.bool()?,
                                    global_sanitizer: record.nzu32()?,
                                    partition: record.range()?,
                                    code_model: record.u32()?,
                                }
                            };

                            // Verify that we're not in the middle of parsing functions
                            if !self.functions.is_empty() {
                                return Err(Error::Other("Global variable in middle of function parsing"));
                            }

                            // See also METADATA_GLOBAL_DECL_ATTACHMENT
                            self.global_value_list.data.push(Value {
                                numeric_value: None,
                                type_id,
                                name: Some(name),
                            });
                            module.global_var.push(var);
                        }
                        ModuleCode::Function => {
                            let fun = ModuleFunctionRecord {
                                symbol_strtab_range: record.range()?,
                                ty: record.u32()?,
                                calling_conv: CallConv::try_from(record.u8()?)
                                    .map_err(|_| Error::Other("Invalid calling convention"))?,
                                is_proto: record.bool()?,
                                linkage: Linkage::try_from(record.u8()?)
                                    .map_err(|_| Error::Other("Invalid linkage"))?,
                                attributes_index: record.nzu32()?.map(|id| id.get() - 1),
                                alignment: record.nzu32()?,
                                section: record.nzu32()?,
                                visibility: record.u8()?,
                                gc: record.nzu64()?,
                                unnamed_addr: record.nzu8()?,
                                prologue_data: record.nzu64()?,
                                dll_storage_class: DllStorageClass::try_from(record.u8()?)
                                    .map_err(|_| Error::Other("Invalid DLL storage class"))?,
                                comdat: record.nzu64()?,
                                prefix_data: record.nzu64()?,
                                personality_fn: record.nzu64()?,
                                dso_local: record.bool()?,
                                address_space: record.u64()?,
                                partition_name: if !record.is_empty() {
                                    record.range()?
                                } else {
                                    0..0 // back compat
                                },
                            };

                            // Verify that we're not in the middle of parsing functions
                            if !self.functions.is_empty() {
                                return Err(Error::Other("Function declaration in middle of function parsing"));
                            }

                            self.global_value_list.data.push(Value {
                                numeric_value: None,
                                type_id: fun.ty,
                                name: Some(fun.symbol_strtab_range.clone()),
                            });

                            // Function definitions are handled separately
                            let dest = if !fun.is_proto {
                                &mut self.module_defined_functions
                            } else {
                                &mut self.function_prototypes
                            };
                            dest.push(fun);
                        }
                        ModuleCode::AliasOld => {
                            module.alias = Some(ModuleAliasRecord {
                                name: record.range()?,
                                alias_type: record.u32()?,
                                aliasee_val: ValueId(record.u32()?),
                                linkage: record.u64()?,
                                visibility: record.u8()?,
                                dll_storage_class: DllStorageClass::try_from(record.u8()?)
                                    .map_err(|_| Error::Other("Invalid DLL storage class"))?,
                                threadlocal: record.u8()?,
                                unnamed_addr: record.nzu8()?,
                                preemption_specifier: record.u64()?,
                            });
                        }
                        ModuleCode::GCName => {
                            module.gc_name = Some(ModuleGCNameRecord { gc_name: record.string_utf8()? });
                        },
                        ModuleCode::VstOffset => {
                            // VST offset record doesn't produce a typed record
                            continue;
                        }
                        ModuleCode::SourceFilename => {
                            module.source_filename = Some(record.string_utf8()?);
                        }
                        ModuleCode::Comdat => {
                            // getEncodedComdatSelectionKind
                            module.comdats.push((record.range()?, record.u64()?));
                        }
                        ModuleCode::Hash => {
                            module.hash = Some(ModuleHashRecord(std::array::from_fn(|_| {
                                record.u32().unwrap_or_default()
                            })));
                        }
                        _ => {
                            debug_assert!(false, "unexpected record in module: {record_id:?}");
                            return Err(Error::Other("unexpected module record"));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// No forward refs
    fn read_metadata_node_reference(&self, local_list: &LocalList<Arc<MetadataRecord>>, record: &mut RecordIter<'_, '_>) -> Result<Option<Arc<MetadataRecord>>, Error> {
        self.read_metadata_node_reference_global(Some(local_list), record)
    }

    fn read_metadata_node_reference_global(&self, local_list: Option<&LocalList<Arc<MetadataRecord>>>, record: &mut RecordIter<'_, '_>) -> Result<Option<Arc<MetadataRecord>>, Error> {
        Ok(record.nzu32()?.map(|v| {
            let mdnode_id = v.get() - 1;
            if let Some(local_list) = local_list {
                self.global_metadata.get(local_list, mdnode_id as usize)
            } else {
                self.global_metadata.data.get(mdnode_id as usize)
            }.cloned().unwrap_or_else(|| {
                eprintln!("md fwd ref {mdnode_id}; globals.len = {} + locals.len() = {:?}", self.global_metadata.data.len(), local_list.map(|l| l.data.len()));
                Arc::new(MetadataRecord::UnresolvedReference(mdnode_id))
            })
        }))
    }

    fn parse_function_record<'cursor>(
        &mut self,
        mut record: RecordIter<'cursor, 'input>,
        func: &mut Function,
        last_debug_loc: &mut DebugLoc,
    ) -> Result<Option<FunctionRecord>, Error> {
        Ok(Some(
            match FunctionCode::try_from(record.id as u8).unwrap() {
                FunctionCode::DeclareBlocks => {
                    let num = record.u64()?;
                    let bb = &mut func.basic_blocks;
                    bb.reserve_exact(num as _);
                    if bb.is_empty() {
                        bb.push(BasicBlock {
                            name: None,
                            instructions: Vec::new(),
                        });
                    }
                    return Ok(None);
                }
                // this is still relevant, in addition to metadata
                FunctionCode::DebugLocAgain => {
                    FunctionRecord::FunctionDI(DebugInstruction::Loc(last_debug_loc.clone()))
                }
                // this is still relevant, in addition to metadata
                FunctionCode::DebugLoc => {
                    *last_debug_loc = DebugLoc {
                        line: record.u32()?,
                        column: record.u32()?,
                        scope: self.read_metadata_node_reference(&func.local_metadata, &mut record).expect("not fwd"),
                        inlined_at: self.read_metadata_node_reference(&func.local_metadata, &mut record).expect("not fwd2"),
                        implicit_code: record.bool()?,
                    };
                    FunctionRecord::FunctionDI(DebugInstruction::Loc(last_debug_loc.clone()))
                }
                FunctionCode::OperandBundle => {
                    FunctionRecord::FunctionOperandBundle(FunctionOperandBundle {
                        tag_id: record.u64()?, // getOperandBundleTagID
                        values_types: {
                            let mut tmp = Vec::new();
                            while !record.is_empty() {
                                tmp.push(self.value_and_type(&mut record, func)?);
                            }
                            tmp
                        },
                    })
                }
                FunctionCode::BlockaddrUsers => {
                    FunctionRecord::FunctionBlockAddrUsers(FunctionBlockAddrUsers(
                        record
                            .map(|r| r.map(|u| ValueId(u as u32)))
                            .collect::<Result<Vec<_>, _>>()?,
                    ))
                }
                FunctionCode::DebugRecordValue => {
                    FunctionRecord::FunctionDI(DebugInstruction::RecordValue(DebugRecordValue {
                        di_location: self.read_metadata_node_reference(&func.local_metadata, &mut record)?.expect("md missing"),
                        di_local_variable: self.read_metadata_node_reference(&func.local_metadata, &mut record)?.expect("md missing"),
                        di_expression: self.read_metadata_node_reference(&func.local_metadata, &mut record)?.expect("md missing"),
                        value_as_metadata: self.read_metadata_node_reference(&func.local_metadata, &mut record)?.expect("md missing"),
                    }))
                }
                FunctionCode::DebugRecordDeclare => FunctionRecord::FunctionDI(
                    DebugInstruction::RecordDeclare(DebugRecordDeclare {
                        di_location: self.read_metadata_node_reference(&func.local_metadata, &mut record)?.expect("md missing"),
                        di_local_variable: self.read_metadata_node_reference(&func.local_metadata, &mut record)?.expect("md missing"),
                        di_expression: self.read_metadata_node_reference(&func.local_metadata, &mut record)?.expect("md missing"),
                        value_as_metadata: self.read_metadata_node_reference(&func.local_metadata, &mut record)?.expect("md missing"),
                    }),
                ),
                FunctionCode::DebugRecordAssign => {
                    FunctionRecord::FunctionDI(DebugInstruction::RecordAssign(DebugRecordAssign {
                        di_location: self.read_metadata_node_reference(&func.local_metadata, &mut record)?.expect("md missing"),
                        di_local_variable: self.read_metadata_node_reference(&func.local_metadata, &mut record)?.expect("md missing"),
                        di_expression: self.read_metadata_node_reference(&func.local_metadata, &mut record)?.expect("md missing"),
                        value_as_metadata: self.read_metadata_node_reference(&func.local_metadata, &mut record)?.expect("md missing"),
                        di_assign_id: self.read_metadata_node_reference(&func.local_metadata, &mut record)?.expect("md missing"),
                        di_expression_addr: self.read_metadata_node_reference(&func.local_metadata, &mut record)?.expect("md missing"),
                        value_as_metadata_addr: self.read_metadata_node_reference(&func.local_metadata, &mut record)?.expect("md missing"),
                    }))
                }
                FunctionCode::DebugRecordValueSimple => {
                    return Ok(Some(FunctionRecord::FunctionDI(
                        DebugInstruction::RecordValueSimple(DebugRecordValueSimple {
                            di_location: self.read_metadata_node_reference(&func.local_metadata, &mut record)?.expect("md missing"),
                            di_local_variable: self.read_metadata_node_reference(&func.local_metadata, &mut record)?.expect("md missing"),
                            di_expression: self.read_metadata_node_reference(&func.local_metadata, &mut record)?.expect("md missing"),
                            value: ValueId(record.u32()?),
                        }),
                    )));
                }
                FunctionCode::DebugRecordLabel => {
                    FunctionRecord::FunctionDI(DebugInstruction::RecordLabel(DebugRecordLabel {
                        di_location: self.read_metadata_node_reference(&func.local_metadata, &mut record)?.expect("md missing"),
                        di_label: self.read_metadata_node_reference(&func.local_metadata, &mut record)?.expect("md missing"),
                    }))
                }
                _ => {
                    let inst = match FunctionCode::try_from(record.id as u8).unwrap() {
                        FunctionCode::BinOp => {
                            let (operand_val, operand_ty) =
                                self.value_and_type(&mut record, func)?;
                            let operand2_val = Self::value_without_type(&mut record, func)?;

                            let opcode = BinOpcode::try_from(record.u8()?)
                                .map_err(|_| Error::Other("bad binop"))?;
                            let op_vals = [operand_val, operand2_val];
                            let flags = record.next()?.unwrap_or(0) as u8;
                            if record.len() >= 4 {
                                // uint8_t Flags = 0;
                                // if (Record.size() >= 4) {
                                //   if (Opc == Instruction::Add ||
                                //       Opc == Instruction::Sub ||
                                //       Opc == Instruction::Mul ||
                                //       Opc == Instruction::Shl) {
                                //     if (Record[3] & (1 << bitc::OBO_NO_SIGNED_WRAP))
                                //       Flags |= OverflowingBinaryOperator::NoSignedWrap;
                                //     if (Record[3] & (1 << bitc::OBO_NO_UNSIGNED_WRAP))
                                //       Flags |= OverflowingBinaryOperator::NoUnsignedWrap;
                                //   } else if (Opc == Instruction::SDiv ||
                                //              Opc == Instruction::UDiv ||
                                //              Opc == Instruction::LShr ||
                                //              Opc == Instruction::AShr) {
                                //     if (Record[3] & (1 << bitc::PEO_EXACT))
                                //       Flags |= PossiblyExactOperator::IsExact;
                                //   }
                                // }
                            }
                            let inst = Inst::BinOp(InstBinOp {
                                opcode,
                                op_vals,
                                operand_ty,
                                flags,
                            });
                            debug_assert!(!inst.is_void_type(&self.types));
                            inst
                        }
                        FunctionCode::Cast => {
                            let (operand_val, operand_ty) = self.value_and_type(&mut record, func)?;
                            let inst = Inst::Cast(InstCast {
                                operand_ty,
                                operand_val,
                                result_ty: record.u32()?,
                                opcode: CastOpcode::try_from(record.u8()?).map_err(|_| Error::Other("bad cast"))?,
                            });
                            debug_assert!(!inst.is_void_type(&self.types), "{inst:?}");
                            inst
                        }

                        FunctionCode::ExtractElt => {
                            let (op0_val, op0_ty) = self.value_and_type(&mut record, func)?;
                            let (op1_val, op1_ty) = self.value_and_type(&mut record, func)?;
                            Inst::ExtractElt(InstExtractElt { op0_ty, op0_val, op1_ty, op1_val })
                        },
                        FunctionCode::InsertElt => {
                            let (op0_val, op0_ty) = self.value_and_type(&mut record, func)?;
                            let op1 = Self::value_without_type(&mut record, func)?;
                            let (op2_val, op2_ty) = self.value_and_type(&mut record, func)?;

                            Inst::InsertElt(InstInsertElt { op0_ty, op0_val, op1, op2_ty, op2_val })
                        },
                        FunctionCode::ShuffleVec => {
                            let (vector_val, vector_ty) = self.value_and_type(&mut record, func)?;
                            let op = Self::value_without_type(&mut record, func)?;
                            let mask = Self::value_without_type(&mut record, func)?;

                            Inst::ShuffleVec(InstShuffleVec { vector_ty, vector_val, op, mask })
                        },
                        FunctionCode::Ret => {
                            let value = if record.is_empty() {
                                debug_assert!(record.payload()?.is_none());
                                None
                            } else {
                                Some(self.value_and_type(&mut record, func)?)
                            };
                            debug_assert!(record.next()?.is_none(), "retvals");
                            let inst = Inst::Ret(InstRet { value });
                            assert!(inst.is_terminator());
                            inst
                        }
                        FunctionCode::Br => {
                            // this is bb index, not val_id
                            let true_bb = BbId(record.u32()?);
                            let inst = Inst::Br(if record.is_empty() {
                                InstBr::Uncond { dest_bb: true_bb }
                            } else {
                                InstBr::Cond {
                                    true_bb,
                                    false_bb: BbId(record.u32()?),
                                    condition_val: Self::value_without_type(&mut record, func)?,
                                }
                            });
                            assert!(inst.is_terminator());
                            inst
                        }
                        FunctionCode::Switch => {
                            let condition_ty = record.u32()?;
                            let condition_val = Self::value_without_type(&mut record, func)?;
                            let default_bb = BbId(record.u32()?);
                            let num_cases = record.len() / 2;
                            let mut cases = Vec::with_capacity(num_cases);
                            for _ in 0..num_cases {
                                let value = ValueId(record.u32()?);
                                let target_bb = BbId(record.u32()?);
                                cases.push((value, target_bb));
                            }
                            Inst::Switch(InstSwitch {
                                condition_ty,
                                condition_val,
                                default_bb,
                                cases,
                            })
                        }
                        // can be void
                        FunctionCode::Invoke => {
                            // writes operand bundles
                            let attr = record.u64()?;

                            // Calling convention + flags
                            let calling_conv_flags = record.u64()?;
                            let calling_conv = CallConv::from_flags(calling_conv_flags).unwrap();

                            let normal_bb = BbId(record.u32()?);
                            let unwind_bb = BbId(record.u32()?);

                            // Determine if explicit func type is present
                            let explicit_type = (calling_conv_flags >> 13) & 1 != 0;
                            let function_ty = if explicit_type { Some(record.u32()?) } else { None };
                            let (callee_val, callee_ty) = self.value_and_type(&mut record, func)?;
                            let function_ty = function_ty.unwrap_or(callee_ty);
                            let args = self.function_args(function_ty, &mut record, func)?;

                            Inst::Invoke(InstInvoke {
                                attr,
                                calling_conv,
                                callee_val,
                                normal_bb,
                                unwind_bb,
                                function_ty,
                                args,
                            })
                        }
                        FunctionCode::Unreachable => {
                            assert!(record.is_empty());
                            Inst::Unreachable
                        }
                        FunctionCode::Phi => {
                            let ty = record.u32()?;
                            let mut incoming = Vec::new();
                            while record.len() >= 2 {
                                let incoming_val = self.value_signed(&mut record, func)?;
                                let incoming_bb = BbId(record.u32()?);
                                incoming.push((incoming_val, incoming_bb));
                            }
                            let flags = record.next()?.unwrap_or(0) as u8;
                            Inst::Phi(InstPhi { ty, incoming, flags })
                        },
                        // AllocA (INST_ALLOCA 19): Layout
                        // [result_type, array_size_type, array_size_val,
                        // alignment] Here result_type is the type of the
                        // element being allocated (the allocated pointer’s
                        // pointee type). The alloca returns a pointer of
                        // type “pointer to result_type”. In older bitcode,
                        // this was simply encoded as insttype and opty as
                        // above. The second field is the type of the array
                        // size operand (must be an integer, often i32 or i64
                        // depending on platform). The third field is the
                        // array size value ID (or 0 if no array size operand
                        // was present, meaning allocating 1 element). The
                        // fourth field is alignment (log2 encoding +1, or 0
                        // if unspecified). The parser will create an Alloca
                        // of type pointer-to-result_type. If array_size_val
                        // is 0, it implies a constant 1 (older bitcode may
                        // have omitted the operand entirely for single
                        // element; but the presence of array_size_type field
                        // suggests they even encode a constant 1 as an ID 0
                        // which might actually refer to an implicit null? In
                        // practice, LLVM may use a constant 1 from the
                        // constant pool and reference it).
                        FunctionCode::Alloca => Inst::Alloca(InstAlloca {
                            result_ty: record.u32()?,
                            array_size_ty: record.u32()?,
                            array_size_val: ValueId(record.u32()?),
                            alignment: record.u64()?,
                        }),
                        id @ (FunctionCode::Load | FunctionCode::LoadAtomic) => {
                            let is_atomic = matches!(id, FunctionCode::LoadAtomic);
                            let (ptr_val, ptr_ty) = self.value_and_type(&mut record, func)?;
                            Inst::Load(InstLoad {
                                ptr_ty,
                                ptr_val,
                                ret_ty: record.u32()?,
                                alignment: record.u64()?,
                                is_volatile: record.bool()?,
                                atomic: if is_atomic { Some((record.try_from::<u8, _>()?, record.u64()?)) } else { None },
                            })
                        }
                        FunctionCode::VaArg => Inst::VAArg(InstVAArg {
                            valist_ty: record.u32()?,
                            valist_val: ValueId(record.u32()?),
                            result_ty: record.u32()?,
                        }),
                        id @ (FunctionCode::Store
                        | FunctionCode::StoreOld
                        | FunctionCode::StoreAtomic
                        | FunctionCode::StoreAtomicOld) => {
                            let is_atomic = matches!(
                                id,
                                FunctionCode::StoreAtomic | FunctionCode::StoreAtomicOld
                            );
                            let (ptr_val, ptr_ty) = self.value_and_type(&mut record, func)?;
                            let (stored_val, stored_ty) = self.value_and_type(&mut record, func)?;
                            let alignment = record.u64()?;
                            let is_volatile = record.bool()?;

                            Inst::Store(InstStore {
                                ptr_ty,
                                ptr_val,
                                stored_val,
                                stored_ty,
                                alignment,
                                is_volatile,
                                atomic: if is_atomic { Some((record.try_from::<u8, _>()?, record.u64()?)) } else { None },
                            })
                        }
                        FunctionCode::ExtractValue => {
                            //ok
                            let (val, ty) = self.value_and_type(&mut record, func)?;
                            Inst::ExtractVal(InstExtractVal {
                                ty,
                                val,
                                operands: record.collect::<Result<Vec<_>, _>>()?,
                            })
                        }
                        FunctionCode::InsertValue => {
                            // ok
                            let (aggregate_val, aggregate_ty) = self.value_and_type(&mut record, func)?;
                            let (element_val, element_ty) = self.value_and_type(&mut record, func)?;
                            let indices = record.array()?;
                            Inst::InsertVal(InstInsertVal {
                                aggregate_ty,
                                aggregate_val,
                                element_ty,
                                element_val,
                                indices,
                            })
                        }
                        FunctionCode::Cmp2 | FunctionCode::Cmp => {
                            // ok
                            let (lhs_val, operand_ty) = self.value_and_type(&mut record, func)?;
                            Inst::Cmp(InstCmp {
                                operand_ty,
                                lhs_val,
                                rhs_val: Self::value_without_type(&mut record, func)?,
                                predicate: record.u64()?,
                                flags: record.next()?.unwrap_or(0),
                            })
                        }
                        FunctionCode::SelectOld => {
                            debug_assert!(false);

                            // obsolete opcode
                            let mut condition_ty = None;
                            for (i, t) in self.types.types.iter().enumerate() {
                                if matches!(t, Type::Integer { width: n } if n.get() == 1) {
                                    condition_ty = Some(i as u32);
                                    break;
                                }
                            }
                            let (true_val, result_ty) = self.value_and_type(&mut record, func)?;
                            let false_val = Self::value_without_type(&mut record, func)?;
                            let condition_val = Self::value_without_type(&mut record, func)?;
                            Inst::Select(InstSelect {
                                result_ty,
                                condition_ty: condition_ty.unwrap(),
                                condition_val,
                                true_val,
                                false_val,
                                flags: 0,
                            })
                        }
                        FunctionCode::Vselect => {
                            // ok
                            let (true_val, result_ty) = self.value_and_type(&mut record, func)?;
                            let false_val = Self::value_without_type(&mut record, func)?;
                            let (condition_val, condition_ty) =
                                self.value_and_type(&mut record, func)?;
                            let flags = record.next()?.unwrap_or(0) as u8;
                            let inst = Inst::Select(InstSelect {
                                result_ty,
                                true_val,
                                false_val,
                                condition_ty,
                                condition_val,
                                flags,
                            });
                            debug_assert!(!inst.is_void_type(&self.types), "{inst:?}");
                            inst
                        }
                        // ok
                        FunctionCode::IndirectBr => Inst::IndirectBr(InstIndirectBr {
                            ptr_ty: record.u32()?,
                            address_val: Self::value_without_type(&mut record, func)?,
                            destinations: record.map(|v| v.map(|v| BbId(v as u32))).collect::<Result<Vec<_>, _>>()?,
                        }),
                        // ok; may be void
                        FunctionCode::Call => {
                            // writes operand bundles
                            let attributes_index = record.nzu32()?.map(|v| v.get() - 1);

                            let calling_conv_flags = record.u64()?;
                            // CALL_FMF
                            let math_flags = if (calling_conv_flags >> 17) & 1 != 0 {
                                record.u8()?
                            } else {
                                0
                            };
                            // CALL_EXPLICIT_TYPE
                            let explicit_type = (calling_conv_flags >> 15) & 1 != 0;
                            debug_assert!(explicit_type, "seems to be written unconditionally?");

                            // This is the type ID of the function type (i.e., a FunctionType*) that specifies the return type and parameter types.
                            // It is emitted explicitly in the bitcode only if the CALL_EXPLICIT_TYPE flag is set.
                            let mut function_ty = if explicit_type {
                                record.u32()?
                            } else {
                                debug_assert!(false, "obsolete?");
                                0
                            };

                            let (callee_val, callee_ty) = self.value_and_type(&mut record, func)?;
                            if !explicit_type {
                                function_ty = callee_ty;
                            }

                            Inst::Call(InstCall {
                                attributes_index,
                                calling_conv: CallConv::from_flags(calling_conv_flags).unwrap(),
                                math_flags,
                                function_ty,
                                callee_val,
                                callee_ty,
                                args: self.function_args(function_ty, &mut record, func)?,
                            })
                        }
                        FunctionCode::Fence => Inst::Fence(InstFence {
                            // getEncodedOrdering
                            ordering: record.try_from::<u8, _>()?,
                            // getEncodedSyncScopeID
                            synch_scope: record.u64()?,
                        }),
                        // ok
                        FunctionCode::Resume => {
                            let (exception_val, exception_ty) = self.value_and_type(&mut record, func)?;
                            Inst::Resume(InstResume { exception_val, exception_ty })
                        },
                        FunctionCode::GepOld => unimplemented!(),
                        FunctionCode::Gep => {
                            let flags = record.u8()?;
                            let source_type = record.u32()? as TypeId;
                            debug_assert_eq!(0, record.len(), "gep should be array?");

                            // this is weird format with vbr6 array not vbr6 fields
                            let record_payload = record.array()?;
                            debug_assert!(!record_payload.is_empty(), "must be 1+ operand");
                            let mut record_payload = record_payload.into_iter().map(Ok);

                            let (base_ptr, base_ty) = self.value_and_type_from_iter(&mut record_payload, func)?;

                            let mut operands = Vec::with_capacity(record.len() / 2);
                            while record_payload.len() > 0 {
                                operands.push(self.value_and_type_from_iter(&mut record_payload, func)?);
                            }
                            let inst = Inst::Gep(InstGep {
                                base_ptr,
                                base_ty,
                                flags,
                                source_type,
                                operands,
                            });
                            debug_assert!(!inst.is_void_type(&self.types), "{inst:?}");
                            inst
                        }
                        // ok
                        FunctionCode::AtomicCmpXchg | FunctionCode::CmpXchgOld => {
                            let (ptr_val, ptr_ty) = self.value_and_type(&mut record, func)?;
                            let (cmp_val, cmp_ty) = self.value_and_type(&mut record, func)?;
                            let new_val = Self::value_without_type(&mut record, func)?;
                            Inst::CmpXchg(InstCmpXchg {
                                ptr_ty,
                                ptr_val,
                                cmp_val,
                                cmp_ty,
                                new_val,
                                is_volatile: record.bool()?,
                                success_ordering: record.try_from::<u8, _>()?,
                                // getEncodedSyncScopeID
                                synch_scope: record.u64()?,
                                failure_ordering: record.try_from::<u8, _>()?,
                                is_weak: record.bool()?,
                                // getEncodedAlign
                                alignment: record.u64()?,
                            })
                        }
                        // ok
                        FunctionCode::LandingPad | FunctionCode::LandingPadOld => {
                            let result_ty = record.u32()?;
                            let is_cleanup = record.bool()?;
                            let num_clauses = record.u64()? as usize;
                            let mut clauses = Vec::with_capacity(num_clauses);
                            for _ in 0..num_clauses {
                                // catch or filter
                                let is_filter = record.bool()?;
                                clauses.push((is_filter, self.value_and_type(&mut record, func)?));
                            }
                            Inst::LandingPad(InstLandingPad { result_ty, is_cleanup, clauses })
                        },
                        // ok
                        FunctionCode::CleanupRet => Inst::CleanupRet(InstCleanupRet {
                            cleanup_pad: Self::value_without_type(&mut record, func)?,
                            unwind_dest: record.next()?.map(|v| BbId(v as u32)),
                        }),
                        // ok
                        FunctionCode::CatchRet => Inst::CatchRet(InstCatchRet {
                            catch_pad: Self::value_without_type(&mut record, func)?,
                            successor: BbId(record.u32()?),
                        }),
                        // ok
                        id @ (FunctionCode::CatchPad | FunctionCode::CleanupPad) => {
                            let parent_pad = Self::value_without_type(&mut record, func)?;
                            let num_operands = record.u64()? as usize;
                            let mut args = Vec::with_capacity(num_operands);
                            for _ in 0..num_operands {
                                args.push(self.value_and_type(&mut record, func)?);
                            }
                            if matches!(id, FunctionCode::CatchPad) {
                                Inst::CatchPad(InstCatchPad { parent_pad, args })
                            } else {
                                Inst::CleanupPad(InstCleanupPad { parent_pad, args })
                            }
                        }
                        // ok
                        FunctionCode::CatchSwitch => {
                            let parent_pad = Self::value_without_type(&mut record, func)?;
                            let num_args = record.u64()? as usize;
                            let mut args = Vec::with_capacity(num_args);
                            for _ in 0..num_args {
                                args.push(ValueId(record.u32()?));
                            }
                            Inst::CatchSwitch(InstCatchSwitch {
                                parent_pad,
                                args,
                                unwind_dest: record.next()?.filter(|&u| u != !0).map(|u| BbId(u as u32)),
                            })
                        }
                        // TODO
                        FunctionCode::UnOp => {
                            let (operand_val, operand_ty) = self.value_and_type(&mut record, func)?;
                            Inst::UnOp(InstUnOp {
                                operand_ty,
                                operand_val,
                                opcode: record.u8()?,
                                flags: record.next()?.unwrap_or(0) as u8,
                            })
                        }

                        FunctionCode::CallBr => {
                            // has operand bundles
                            let attr = record.u64()?; // getAttributeListID
                            let calling_conv_flags = record.u64()?;
                            let normal_bb = BbId(record.u32()?);

                            let num_indirect_dests = record.u64()? as usize;
                            let indirect_bb = record
                                .by_ref()
                                .take(num_indirect_dests)
                                .map(|r| r.map(|v| BbId(v as u32)))
                                .collect::<Result<Vec<_>, _>>()?;

                            // CALL_EXPLICIT_TYPE
                            let explicit_type = (calling_conv_flags >> 15) & 1 != 0;
                            let mut function_ty = if explicit_type {
                                record.u32()?
                            } else {
                                debug_assert!(false, "obsolete?");
                                0
                            };
                            let (callee_val, callee_ty) = self.value_and_type(&mut record, func)?;
                            if !explicit_type {
                                function_ty = callee_ty;
                            }
                            Inst::CallBr(InstCallBr {
                                attr,
                                calling_conv: CallConv::from_flags(calling_conv_flags).unwrap(),
                                normal_bb,
                                indirect_bb,
                                function_ty,
                                callee_val,
                                callee_ty,
                                args: self.function_args(function_ty, &mut record, func)?,
                            })
                        }
                        FunctionCode::Freeze => {
                            let (operand_val, operand_ty) = self.value_and_type(&mut record, func)?;
                            Inst::Freeze(InstFreeze { operand_ty, operand_val })
                        },
                        FunctionCode::AtomicRmw | FunctionCode::AtomicRmwOld => {
                            let (ptr_val, ptr_ty) = self.value_and_type(&mut record, func)?;
                            let (stored_val, val_ty) = self.value_and_type(&mut record, func)?;
                            Inst::AtomicRmw(InstAtomicRmw {
                                ptr_ty,
                                ptr_val,
                                val_ty,
                                stored_val,
                                operation: record.u64()?,
                                is_volatile: record.bool()?,
                                ordering: record.try_from::<u8, _>()?,
                                synch_scope: record.u64()?,
                                alignment: record.u64()?,
                            })
                        }
                        _ => unimplemented!(),
                    };
                    FunctionRecord::FunctionInst(inst)
                }
            },
        ))
    }

    // fn parse_global_value_summary<'cursor>(
    //     &mut self,
    //     mut block: BlockIter<'cursor, 'input>,
    // ) -> Result<(), Error> {
    //     while let Some(mut record) = block.next_record()? {
    //         match GlobalValueSummaryCode::try_from(record.id as u8).unwrap() {
    //             GlobalValueSummaryCode::PerModuleGlobalvarInitRefs => {
    //                 let _ = PerModuleGlobalVarInitRefsRecord {
    //                     value_id: ValueId(record.u32()?),
    //                     flags: record.u64()?,
    //                     init_refs: record.array()?.into_iter().map(|u| u as u32).collect(),
    //                 };
    //             },
    //             _ => unimplemented!(),
    //         };
    //     }
    //     Ok(())
    // }

    fn parse_value_symtab_record<'cursor>(
        &mut self, mut record: RecordIter<'cursor, 'input>,
    ) -> Result<ValueSymtab, Error> {
        Ok(match ValueSymtabCode::try_from(record.id as u8).unwrap() {
            ValueSymtabCode::Entry => ValueSymtab::Entry(ValueSymtabEntryRecord {
                value_id: ValueId(record.u32()?),
                name: record.string_utf8()?,
            }),
            ValueSymtabCode::BbEntry => ValueSymtab::Bbentry(ValueSymtabBbentryRecord {
                id: BbId(record.u32()?),
                name: record.string_utf8()?,
            }),
            ValueSymtabCode::FnEntry => {
                // unused
                ValueSymtab::FnEntry(ValueSymtabFnentryRecord {
                    linkage_value_id: ValueId(record.u32()?),
                    function_offset: record.u64()?,
                    name: record.string_utf8().ok(),
                })
            }
            // Obsolete
            ValueSymtabCode::CombinedEntry => {
                ValueSymtab::CombinedEntry(ValueSymtabCombinedEntryRecord {
                    linkage_value_id: ValueId(record.u32()?),
                    refguid: record.u64()?,
                })
            }
            _ => unimplemented!(),
        })
    }

    //  [attr_index, fnty, callee, arg0... argN, flags].
    //  this only gets arg0..argN
    fn function_args<'cursor>(
        &mut self,
        function_ty: TypeId,
        record: &mut RecordIter<'cursor, 'input>,
        func: &mut Function,
    ) -> Result<Vec<CallArg>, Error> {
        let ty = self.types.get_fn(function_ty).ok_or(Error::Other("bad function type"))?;

        let arg_types = ty.param_types.as_slice();
        let mut args = Vec::with_capacity(arg_types.len());

        for &arg_ty in arg_types {
            let ty = self.types.get(arg_ty).ok_or(Error::Other("bad arg"))?;
            let id = Self::value_without_type(record, func)?;
            args.push(if matches!(ty, Type::Label) {
                CallArg::Label(BbId(id.0.try_into().unwrap()))
            } else {
                CallArg::Val(id)
            });
        }
        if ty.vararg {
            for _ in 0..record.len() {
                let (v_op, v_ty) = self.value_and_type(record, func)?;
                args.push(CallArg::Var(v_op, v_ty));
            }
        }
        Ok(args)
    }

    pub fn parse_param_attr_group_record(&mut self, mut record: RecordIter<'_, 'input>) -> Result<(), Error> {
        let group_id = record.u32()?;
        let index = record.u32()?;

        let group = self.attribute_groups.entry(group_id).or_default();

        let attributes = if index == !0 {
            &mut group.function
        } else if index == 0 {
            &mut group.ret
        } else {
            let index = index as usize;
            group.arg.resize_with(index, Vec::new);
            &mut group.arg[index - 1]
        };

        attributes.reserve(record.len());

        while let Some(id) = record.next()? {
            attributes.push(match ParamAttrGroupCodes::try_from(id as u8).unwrap() {
                ParamAttrGroupCodes::EnumAttr => Attribute::AttrKind(AttrKind::try_from(record.u8()?).unwrap()),
                ParamAttrGroupCodes::IntAttr => {
                    let kind = AttrKind::try_from(record.u8()?).unwrap();
                    let value = record.u64()?;
                    Attribute::Int { kind, value }
                }
                id @ (ParamAttrGroupCodes::StringAttr
                | ParamAttrGroupCodes::StringAttrWithValue) => {
                    let key = record.zstring()?;
                    let mut value = None;
                    if matches!(id, ParamAttrGroupCodes::StringAttrWithValue) {
                        value = Some(record.zstring()?);
                    }
                    Attribute::String { key, value }
                }
                id @ (ParamAttrGroupCodes::TypeAttr | ParamAttrGroupCodes::TypeAttrTypeId) => {
                    let kind = AttrKind::try_from(record.u8()?).unwrap();
                    let type_id = if matches!(id, ParamAttrGroupCodes::TypeAttrTypeId) {
                        Some(record.u64()?)
                    } else {
                        None
                    };
                    Attribute::Type { kind, type_id }
                }
                ParamAttrGroupCodes::ConstantRange => {
                    let kind = AttrKind::try_from(record.u8()?).unwrap();
                    let bit_width = record.u32()?;
                    let lower = record.i64()?;
                    let upper = record.i64()?;
                    Attribute::ConstantRange { bit_width, kind, range: lower..upper }
                },
                ParamAttrGroupCodes::ConstantRangeList => {
                    let kind = AttrKind::try_from(record.u8()?).unwrap();
                    let num_ranges = record.u64()?;
                    let bit_width = record.u32()?;
                    let mut ranges = Vec::new();
                    for _ in 0..num_ranges {
                        let lower = record.i64()?;
                        let upper = record.i64()?;
                        ranges.push(lower..upper);
                    }
                    Attribute::ConstantRangeList { kind, bit_width, ranges }
                },
            });
        }
        Ok(())
    }

    pub fn process_vst(&mut self, vst: Vec<ValueSymtab>, mut func: Option<&mut Function>) -> Result<(), Error> {
        for v in vst {
            match v {
                ValueSymtab::Entry(r) => {
                    let val_id = r.value_id.0 as usize;
                    let global_end =
                        func.as_ref().map_or(self.global_value_list.data.len(), |f| f.local_value_list.first);
                    if val_id < global_end && func.is_some() {
                        return Err(Error::Other("global vst inside function"));
                    }
                },
                ValueSymtab::Bbentry(b) => {
                    let func = func.as_mut().ok_or(Error::Other("bb outside function"))?;
                    func.basic_blocks[b.id.0 as usize].name = Some(b.name);
                }
                ValueSymtab::FnEntry(r) => {
                    assert!(r.name.is_none(), "strtab replaced fn vst {r:?}");
                }
                ValueSymtab::CombinedEntry(_) => unimplemented!(),
            }
        }
        Ok(())
    }

    fn parse_type_block(&mut self, mut b: BlockIter<'_, 'input>) -> Result<(), Error> {
        let mut name = None;
        while let Some(mut record) = b.next_record()? {
            let ty = match TypeCode::try_from(record.id as u8).unwrap() {
                TypeCode::NumEntry => {
                    let n = record.u64()?;
                    self.types.types.reserve(n as usize);
                    continue;
                }
                TypeCode::Void => Type::Void,
                TypeCode::Half => Type::Half,
                TypeCode::BFloat => Type::BFloat,
                TypeCode::Float => Type::Float,
                TypeCode::Double => Type::Double,
                TypeCode::Label => Type::Label,
                TypeCode::Opaque => Type::Opaque,
                TypeCode::Integer => Type::Integer { width: record.nzu8()?.unwrap() },
                TypeCode::Pointer => Type::Opaque, // obsolete
                TypeCode::FunctionOld => return Err(Error::Other("obsolete")),
                TypeCode::Array => Type::Array(TypeArrayRecord {
                    num_elements: record.u64()?,
                    elements_type: record.u32()?,
                }),
                TypeCode::Vector => Type::Vector(TypeVectorRecord {
                    num_elements: record.u64()?,
                    elements_type: record.u32()?,
                }),
                TypeCode::X86Fp80 => Type::X86Fp80,
                TypeCode::Fp128 => Type::Fp128,
                TypeCode::PpcFp128 => Type::PpcFp128,
                TypeCode::Metadata => Type::Metadata,
                TypeCode::X86Mmx => Type::X86Mmx,
                TypeCode::StructAnon => Type::Struct(TypeStructRecord {
                    name: None,
                    is_packed: record.next().unwrap().unwrap() != 0,
                    element_types: record.array()?.into_iter().map(|t| t as u32).collect::<Vec<_>>(),
                }),
                TypeCode::StructName => {
                    name = Some(record.string_utf8()?);
                    continue;
                }
                TypeCode::StructNamed => Type::Struct(TypeStructRecord {
                    name: name.take(),
                    is_packed: record.next().unwrap().unwrap() != 0,
                    element_types: record.array()?.into_iter().map(|t| t as u32).collect::<Vec<_>>(),
                }),
                TypeCode::Function => {
                    let vararg = record.bool()?;
                    let mut array = record.array()?.into_iter().enumerate().map(|(i, v)| (i, v as u32));
                    let ret_ty = array.next().map(|(_, i)| i);
                    Type::Function(TypeFunctionRecord {
                        vararg,
                        ret_ty,
                        param_types: array.map(|(_i, t)| t).collect::<Vec<_>>(),
                    })
                }
                TypeCode::X86Amx => Type::X86Amx,
                TypeCode::TargetType => {
                    let num_tys = record.u64()? as usize;
                    let mut ty_params = Vec::with_capacity(num_tys);
                    for _ in 0..num_tys {
                        ty_params.push(record.u32()?);
                    }
                    Type::TargetType(TypeTargetTypeRecord {
                        ty_params,
                        int_params: record.collect::<Result<Vec<_>, _>>()?,
                    })
                },
                TypeCode::OpaquePointer => {
                    Type::OpaquePointer(TypeOpaquePointerRecord { address_space: record.u64()? })
                },
                TypeCode::Token => Type::Token,
                c => unimplemented!("type {c:?}"),
            };
            self.types.types.push(ty);
        }
        debug_assert!(name.is_none());
        Ok(())
    }

    pub fn parse_metadata_attachment(
        &mut self, mut block: BlockIter<'_, 'input>, func: &mut Function,
    ) -> Result<(), Error> {
        while let Some(mut record) = block.next_record()? {
            assert_eq!(record.id, MetadataCode::Attachment as _);
            let num_items = record.len() / 2; // round down

            // instruction if present, function otherwise
            let dest = if record.len() % 2 != 0 {
                let inst_index = record.u64()? as InstIndex;

                if let Some((_, old)) = func.inst_metadata_attachment.last_mut().filter(|(old, _)| *old == inst_index) {
                    old
                } else {
                    func.inst_metadata_attachment.push((inst_index, Vec::new()));
                    &mut func.inst_metadata_attachment.last_mut().unwrap().1
                }
            } else {
                &mut func.fn_metadata_attachment
            };

            dest.reserve(num_items);
            for _ in 0..num_items {
                let md_kind: MetadataKind = MetadataKind::try_from(record.u8()?).unwrap_or(MetadataKind::Unknown);
                // these may be forward refs
                // MDStringRef + GlobalMetadataBitPosIndex

                let md = self.read_metadata_node_reference(&func.local_metadata, &mut record)?.expect("md");
                dest.push((md_kind, md));
            }
        }
        Ok(())
    }

    pub fn parse_metadata_block(
        &mut self, mut block: BlockIter<'_, 'input>, mut func: Option<&mut Function>,
    ) -> Result<(), Error> {
        let mut next_name = None;
        eprintln!(
            "metadata list starts at {}/{:?}",
            self.global_metadata.data.len(),
            func.as_deref().map(|f| f.local_metadata.len())
        );

        while let Some(mut record) = block.next_record()? {
            use llvm_bitcode::schema::records::metadata::*;

            let code = MetadataCode::try_from(record.id as u8).unwrap();
            let local_list = func.as_deref().map(|f| &f.local_metadata);

            let m = match code {
                MetadataCode::Value => MetadataRecord::Value(MetadataValue {
                    type_id: record.u32()?,
                    value_id: ValueId(record.u32()?),
                }),
                id @ (MetadataCode::Node | MetadataCode::DistinctNode) => {
                    // NODE: [n x md num] – non-distinct MDNode.

                    let len = record.len();
                    let mut operands = Vec::with_capacity(len);
                    for _ in 0..len {
                        operands.push(self.read_metadata_node_reference_global(local_list, &mut record)?);
                    }
                    MetadataRecord::Node(MetadataNode {
                        distinct: matches!(id, MetadataCode::DistinctNode),
                        operands,
                    })
                }
                MetadataCode::Name => {
                    next_name = Some(record.string_utf8()?);
                    continue;
                }
                MetadataCode::Location => MetadataRecord::DILocation(DILocation {
                    distinct: record.bool()?,
                    loc: DebugLoc {
                        line: record.u32()?,
                        column: record.u32()?,
                        scope: self.read_metadata_node_reference_global(local_list, &mut record)?,
                        inlined_at: self.read_metadata_node_reference_global(local_list, &mut record)?,
                        implicit_code: record.bool()?,
                    },
                }),
                MetadataCode::NamedNode => {
                    self.metadata_node_names.push(MetadataNamedNode {
                        name: next_name.take().ok_or(Error::Other("missing name"))?,
                        mdnodes: record.collect::<Result<Vec<_>, _>>()?,
                    });
                    continue;
                }
                MetadataCode::Attachment => unreachable!("is in a separate block"),
                MetadataCode::GenericDebug => MetadataRecord::DIGenericNode(DIGenericNode {
                    distinct: record.bool()?,
                    tag: record.u32()?,
                    version: record.u8()?,
                    operands: record.array()?,
                }),
                MetadataCode::Subrange => MetadataRecord::DISubrange(DISubrange {
                    distinct: (record.u64()? & 1) != 0,
                    count: record.next()?,
                    lower_bound: record.next()?,
                    upper_bound: record.next()?,
                    stride: record.next()?,
                }),
                MetadataCode::Enumerator => {
                    // ENUMERATOR: [flags, bit_width, raw_name, wide_value...]
                    let composite = record.u64()?;
                    MetadataRecord::DIEnumerator(DIEnumerator {
                        distinct: (composite & 1) != 0,
                        is_unsigned: (composite & 2) != 0,
                        is_big_int: (composite & 4) != 0,
                        bit_width: record.u32()?,
                        name: mdstring_opt(self.read_metadata_node_reference_global(local_list, &mut record)?),
                        value: record.array()?.into_iter().map(|v| v as i64).collect(),
                    })
                }
                MetadataCode::BasicType => MetadataRecord::DIBasicType(DIBasicType {
                    distinct: record.bool()?,
                    tag: record.u32()?,
                    name: mdstring_opt(self.read_metadata_node_reference_global(local_list, &mut record)?),
                    size_in_bits: record.u64()?,
                    align_in_bits: record.u64()?,
                    encoding: record.u64()?,
                    flags: record.u64()?,
                }),
                MetadataCode::File => MetadataRecord::DIFile(DIFile {
                    distinct: record.bool()?,
                    filename: mdstring_opt(self.read_metadata_node_reference_global(local_list, &mut record)?),
                    directory: self.read_metadata_node_reference_global(local_list, &mut record)?,
                    checksum_kind: record.nzu64()?,
                    raw_checksum: self.read_metadata_node_reference_global(local_list, &mut record)?,
                    raw_source: if !record.is_empty() { self.read_metadata_node_reference_global(local_list, &mut record)? } else { None },
                }),
                MetadataCode::DerivedType => MetadataRecord::DIDerivedType(DIDerivedType {
                    distinct: record.bool()?,
                    tag: record.u32()?,
                    name: mdstring_opt(self.read_metadata_node_reference_global(local_list, &mut record)?),
                    file: self.read_metadata_node_reference_global(local_list, &mut record)?,
                    line: record.u32()?,
                    scope: self.read_metadata_node_reference_global(local_list, &mut record)?,
                    base_type: self.read_metadata_node_reference_global(local_list, &mut record)?,
                    size_in_bits: record.u64()?,
                    align_in_bits: record.u64()?,
                    offset_in_bits: record.u64()?,
                    flags: record.u64()?,
                    extra_data: self.read_metadata_node_reference_global(local_list, &mut record)?,
                    dwarf_address_space: record.nzu64()?.map(|v| v.get() - 1),
                    annotations: self.read_metadata_node_reference_global(local_list, &mut record)?,
                    ptr_auth_data: record.nzu64()?,
                }),
                MetadataCode::CompositeType => MetadataRecord::DICompositeType(DICompositeType {
                    distinct: (record.u8()? & 1) != 0,
                    tag: record.u32()?,
                    name: mdstring_opt(self.read_metadata_node_reference_global(local_list, &mut record)?),
                    file: self.read_metadata_node_reference_global(local_list, &mut record)?,
                    line: record.u32()?,
                    scope: self.read_metadata_node_reference_global(local_list, &mut record)?,
                    base_type: self.read_metadata_node_reference_global(local_list, &mut record)?,
                    size_in_bits: record.u64()?,
                    align_in_bits: record.u64()?,
                    offset_in_bits: record.u64()?,
                    flags: record.u8()?,
                    elements: self.read_metadata_node_reference_global(local_list, &mut record)?,
                    runtime_lang: record.u64()?,
                    vtable_holder: self.read_metadata_node_reference_global(local_list, &mut record)?,
                    template_params: self.read_metadata_node_reference_global(local_list, &mut record)?,
                    raw_identifier: self.read_metadata_node_reference_global(local_list, &mut record)?,
                    discriminator: self.read_metadata_node_reference_global(local_list, &mut record)?,
                    raw_data_location: self.read_metadata_node_reference_global(local_list, &mut record)?,
                    raw_associated: self.read_metadata_node_reference_global(local_list, &mut record)?,
                    raw_allocated: self.read_metadata_node_reference_global(local_list, &mut record)?,
                    raw_rank: self.read_metadata_node_reference_global(local_list, &mut record)?,
                    annotations: self.read_metadata_node_reference_global(local_list, &mut record)?,
                    num_extra_inhabitants: record.next()?.unwrap_or(0),
                    raw_specification: if !record.is_empty() { self.read_metadata_node_reference_global(local_list, &mut record)? } else { None },
                }),
                MetadataCode::SubroutineType => {
                    MetadataRecord::DISubroutineType(DISubroutineType {
                        distinct: (record.u8()? & 1) != 0,
                        flags: record.u64()?,
                        type_array: self.read_metadata_node_reference_global(local_list, &mut record)?,
                        cc: record.u64()?,
                    })
                }
                MetadataCode::CompileUnit => {
                    let record_size = record.len();
                    if record_size < 14 || record_size > 22 {
                        return Err(Error::Other("Invalid record"));
                    };

                    MetadataRecord::DICompileUnit(DICompileUnit {
                        distinct: record.bool()?,
                        source_language: record.u32()?,
                        file: self.read_metadata_node_reference_global(local_list, &mut record)?,
                        producer: mdstring_opt(self.read_metadata_node_reference_global(local_list, &mut record)?),
                        is_optimized: record.bool()?,
                        raw_flags: self.read_metadata_node_reference_global(local_list, &mut record)?,
                        runtime_version: record.u32()?,
                        split_debug_filename: mdstring_opt(
                            self.read_metadata_node_reference_global(local_list, &mut record)?,
                        ),
                        emission_kind: record.u32()?,
                        enum_types: self.read_metadata_node_reference_global(local_list, &mut record)?,
                        retained_types: self.read_metadata_node_reference_global(local_list, &mut record)?,
                        subprograms: record.u64()?,
                        global_variables: self.read_metadata_node_reference_global(local_list, &mut record)?,
                        imported_entities: self.read_metadata_node_reference_global(local_list, &mut record)?,
                        dwo_id: record.u64()?,
                        macros: self.read_metadata_node_reference_global(local_list, &mut record)?,
                        split_debug_inlining: record.u32()?,
                        debug_info_for_profiling: record.u32()?,
                        name_table_kind: record.u32()?,
                        ranges_base_address: record.u64()?,
                        raw_sysroot: if !record.is_empty() {
                            mdstring_opt(self.read_metadata_node_reference_global(local_list, &mut record)?)
                        } else {
                            None
                        },
                        raw_sdk: if !record.is_empty() {
                            mdstring_opt(self.read_metadata_node_reference_global(local_list, &mut record)?)
                        } else {
                            None
                        },
                    })
                },
                MetadataCode::Subprogram => {
                    let record_size = record.len();

                    let composite = record.u64()?; // contains several packed flags
                    let has_sp_flags = (composite & 4) != 0;
                    let has_unit = (composite & 2) != 0;

                    if record_size < 18 || record_size > 21 || !has_sp_flags || !has_unit {
                        return Err(Error::Other("Invalid DISubprogram"));
                    }

                    MetadataRecord::DISubprogram(DISubprogram {
                        distinct: (composite & 1) != 0, // TODO: || sp_flags & SPFlagDefinition
                        scope: self.read_metadata_node_reference_global(local_list, &mut record)?,
                        name: mdstring_opt(self.read_metadata_node_reference_global(local_list, &mut record)?),
                        linkage_name: mdstring_opt(self.read_metadata_node_reference_global(local_list, &mut record)?),
                        file: self.read_metadata_node_reference_global(local_list, &mut record)?,
                        line: record.u32()?,
                        type_id: self.read_metadata_node_reference_global(local_list, &mut record)?,
                        scope_line: record.u32()?,
                        containing_type: self.read_metadata_node_reference_global(local_list, &mut record)?,
                        sp_flags: record.u64()?,
                        virtual_index: record.u64()?,
                        flags: record.u64()?,
                        raw_unit: self.read_metadata_node_reference_global(local_list, &mut record)?,
                        template_params: self.read_metadata_node_reference_global(local_list, &mut record)?,
                        declaration: self.read_metadata_node_reference_global(local_list, &mut record)?,
                        retained_nodes: self.read_metadata_node_reference_global(local_list, &mut record)?,
                        this_adjustment: record.u64()?,
                        thrown_types: self.read_metadata_node_reference_global(local_list, &mut record)?,
                        annotations: if !record.is_empty() { self.read_metadata_node_reference_global(local_list, &mut record)? } else { None },
                        raw_target_func_name: if !record.is_empty() { mdstring_opt(self.read_metadata_node_reference_global(local_list, &mut record)?) } else { None },
                    })
                }
                MetadataCode::LexicalBlock => {
                    // LEXICAL_BLOCK: [distinct, scope, file, line, column]
                    MetadataRecord::DILexicalBlock(DILexicalBlock {
                        distinct: record.bool()?,
                        scope: self.read_metadata_node_reference_global(local_list, &mut record)?,
                        file: self.read_metadata_node_reference_global(local_list, &mut record)?,
                        line: record.u32()?,
                        column: record.u32()?,
                    })
                }
                MetadataCode::LexicalBlockFile => {
                    // LEXICAL_BLOCK_FILE: [distinct, scope, file, discriminator]
                    MetadataRecord::DILexicalBlockFile(DILexicalBlockFile {
                        distinct: record.bool()?,
                        scope: self.read_metadata_node_reference_global(local_list, &mut record)?,
                        file: self.read_metadata_node_reference_global(local_list, &mut record)?,
                        discriminator: record.u64()?,
                    })
                }
                MetadataCode::Namespace => {
                    let record_size = record.len();
                    // NAMESPACE: [composite (distinct|export_symbols), scope, raw_name]
                    let composite = record.u64()?;
                    if record_size != 3 && record_size != 5 {
                        debug_assert_eq!(3, record_size);
                        return Err(Error::Other("wrong DINamespace"));
                    }
                    let scope = self.read_metadata_node_reference_global(local_list, &mut record)?;
                    if record_size == 5 {
                        let _ = record.next();
                    }
                    MetadataRecord::DINamespace(DINamespace {
                        distinct: (composite & 1) != 0,
                        export_symbols: (composite & 2) != 0,
                        scope,
                        name: mdstring_opt(self.read_metadata_node_reference_global(local_list, &mut record)?),
                    })
                }
                MetadataCode::TemplateType => {
                    // TEMPLATE_TYPE: [distinct, raw_name, type, is_default]
                    MetadataRecord::DITemplateTypeParameter(DITemplateTypeParameter {
                        distinct: record.bool()?,
                        name: mdstring_opt(self.read_metadata_node_reference_global(local_list, &mut record)?),
                        type_id: self.read_metadata_node_reference_global(local_list, &mut record)?,
                        is_default: record.bool()?,
                    })
                }
                MetadataCode::TemplateValue => {
                    // TEMPLATE_VALUE: [distinct, tag, raw_name, type, is_default, raw_value]
                    MetadataRecord::DITemplateValueParameter(DITemplateValueParameter {
                        distinct: record.bool()?,
                        tag: record.u32()?,
                        name: mdstring_opt(self.read_metadata_node_reference_global(local_list, &mut record)?),
                        type_id: record.next()?,
                        is_default: record.bool()?,
                        raw_value: record.next()?,
                    })
                }
                MetadataCode::GlobalVar => MetadataRecord::DIGlobalVariable(DIGlobalVariable {
                    distinct: (record.u64()? & 1) != 0,
                    scope: self.read_metadata_node_reference_global(local_list, &mut record)?,
                    name: mdstring_opt(self.read_metadata_node_reference_global(local_list, &mut record)?),
                    linkage_name: mdstring_opt(self.read_metadata_node_reference_global(local_list, &mut record)?),
                    file: self.read_metadata_node_reference_global(local_list, &mut record)?,
                    line: record.u32()?,
                    type_id: record.next()?,
                    is_local_to_unit: record.bool()?,
                    is_definition: record.bool()?,
                    static_data_member_declaration: record.next()?,
                    template_params: record.next()?,
                    align_in_bits: record.u64()?,
                    annotations: record.next()?,
                }),
                MetadataCode::LocalVar => MetadataRecord::DILocalVariable(DILocalVariable {
                    distinct: (record.u64()? & 1) != 0,
                    scope: self.read_metadata_node_reference_global(local_list, &mut record)?,
                    name: mdstring_opt(self.read_metadata_node_reference_global(local_list, &mut record)?),
                    file: self.read_metadata_node_reference_global(local_list, &mut record)?,
                    line: record.u32()?,
                    type_id: record.next()?,
                    arg: record.u64()?,
                    flags: record.u64()?,
                    align_in_bits: record.u64()?,
                    annotations: record.next()?,
                }),
                MetadataCode::Label => MetadataRecord::DILabel(DILabel {
                    distinct: record.bool()?,
                    scope: self.read_metadata_node_reference_global(local_list, &mut record)?,
                    name: mdstring_opt(self.read_metadata_node_reference_global(local_list, &mut record)?),
                    file: self.read_metadata_node_reference_global(local_list, &mut record)?,
                    line: record.u32()?,
                }),
                MetadataCode::Expression => {
                    let composite = record.u64()?;
                    MetadataRecord::DIExpression(DIExpression {
                        distinct: (composite & 1) != 0,
                        elements: record
                            .map(|v| v.map(|v| v as i64))
                            .collect::<Result<Vec<_>, _>>()?,
                    })
                }
                MetadataCode::GlobalVarExpr => {
                    MetadataRecord::DIGlobalVariableExpression(DIGlobalVariableExpression {
                        distinct: record.bool()?,
                        variable: record.u64()?,
                        expression: record.u64()?,
                    })
                }
                MetadataCode::ObjcProperty => MetadataRecord::DIObjCProperty(DIObjCProperty {
                    distinct: record.bool()?,
                    name: mdstring_opt(self.read_metadata_node_reference_global(local_list, &mut record)?),
                    file: self.read_metadata_node_reference_global(local_list, &mut record)?,
                    line: record.u32()?,
                    raw_setter_name: record.next()?,
                    raw_getter_name: record.next()?,
                    attributes: record.u64()?,
                    type_id: record.next()?,
                }),
                MetadataCode::ImportedEntity => {
                    MetadataRecord::DIImportedEntity(DIImportedEntity {
                        distinct: record.bool()?,
                        tag: record.u32()?,
                        scope: self.read_metadata_node_reference_global(local_list, &mut record)?,
                        entity: record.next()?,
                        line: record.u32()?,
                        name: mdstring_opt(self.read_metadata_node_reference_global(local_list, &mut record)?),
                        file: self.read_metadata_node_reference_global(local_list, &mut record)?,
                        elements: record.next()?,
                    })
                }
                MetadataCode::Module => MetadataRecord::DIModule(DIModule {
                    distinct: record.bool()?,
                    operands: record.array()?,
                    line_no: record.u32()?,
                    is_decl: record.bool()?,
                }),
                MetadataCode::Macro => MetadataRecord::DIMacro(DIMacro {
                    distinct: record.bool()?,
                    macinfo_type: record.u32()?,
                    line: record.u32()?,
                    name: mdstring_opt(self.read_metadata_node_reference_global(local_list, &mut record)?),
                    raw_value: record.next()?,
                }),
                MetadataCode::MacroFile => MetadataRecord::DIMacroFile(DIMacroFile {
                    distinct: record.bool()?,
                    macinfo_type: record.u32()?,
                    line: record.u32()?,
                    file: self.read_metadata_node_reference_global(local_list, &mut record)?,
                    elements: record.next()?,
                }),
                MetadataCode::ArgList => MetadataRecord::DIArgList(DIArgList { args: record.array()? }),
                MetadataCode::AssignId => MetadataRecord::DIAssignID(DIAssignID { distinct: record.bool()? }),
                MetadataCode::StringOld => unimplemented!(),
                MetadataCode::Strings => {
                    let count = record.u64()? as usize;
                    let start_offset = record.u64()?;
                    let blob = record.blob()?;
                    let (offsets, strings) = blob.split_at_checked(start_offset as usize).unwrap();

                    let mut next_offset = 0;
                    let mut c = Cursor::new(offsets);
                    let ranges = (0..count)
                        .map(|_| {
                            let len = c.read_vbr(6).unwrap() as usize;
                            let start = next_offset;
                            next_offset += len;
                            start..next_offset
                        })
                        .collect();

                    let strings = MetadataStringsRecord { strings: strings.to_vec(), ranges };

                    let gl = self.global_metadata.data.len();
                    let list = if let Some(func) = func.as_mut() {
                        &mut func.local_metadata.data
                    } else {
                        &mut self.global_metadata.data
                    };
                    for s in strings.strings() {
                        eprintln!("meta[{}] = {s}", gl + list.len());
                        list.push(Arc::new(MetadataRecord::String(s.into())));
                    }
                    continue;
                }
                MetadataCode::IndexOffset | MetadataCode::Index => {
                    // we have no use for offsets
                    return Ok(());
                }
                MetadataCode::GlobalDeclAttachment => {
                    // Implementation for METADATA_GLOBAL_DECL_ATTACHMENT
                    // Structure: [valueid, n x [id, mdnode]]
                    let value_id = ValueId(record.u32()?);
                    let mut attachments = Vec::new();
                    while let Some(id) = record.next()? {
                        let mdnode = record.u64()?;
                        attachments.push((id, mdnode));
                    }
                    let _globals = MetadataAttachment {
                        value_id,
                        attachments,
                    };
                    // FIXME: todo
                    continue;
                }
                id => {
                    unimplemented!("metadata {id:?}")
                }
            };

            if let Some(func) = func.as_mut() {
                &mut func.local_metadata.data
            } else {
                &mut self.global_metadata.data
            }
            .push(Arc::new(m));
        }
        debug_assert!(next_name.is_none());
        eprintln!(
            "metadata list ends at {}/{:?}",
            self.global_metadata.data.len(),
            func.as_deref().map(|f| f.local_metadata.len())
        );
        Ok(())
    }

    fn parse_attributes_record(&mut self, record: RecordIter<'_, '_>) -> Result<(), Error> {
        let record_id =
            AttributeCode::try_from(record.id as u8).map_err(|_| Error::UnexpectedRecord {
                block_id: BlockId::ParamAttr as _,
                record_id: record.id,
            })?;
        match record_id {
            AttributeCode::Entry => {
                self.attributes.push(
                    record
                        .map(|r| r.map(|i| i as ParamAttrGroupId))
                        .collect::<Result<_, Error>>()?,
                );
            }
            AttributeCode::GroupEntry => todo!(),
            _ => unimplemented!(),
        }
        Ok(())
    }

    pub fn function_attributes(&self, f: &ModuleFunctionRecord) -> impl Iterator<Item = &Attribute> {
        f.attributes_index
            .into_iter()
            .flat_map(|idx| &self.attributes[idx as usize])
            .flat_map(|group_idx| &self.attribute_groups[group_idx].function)
    }

    #[must_use]
    pub fn get_value<'either>(&'either self, func: &'either Function, val_id: ValueId) -> Option<&'either Value> {
        self.global_value_list.get(&func.local_value_list, val_id.0 as usize)
    }

    // pub fn get_metadata<'either>(
    //     &'either self,
    //     func: &'either Function,
    //     mdnode_id: MetadataNodeId,
    // ) -> Option<&'either MetadataRecord> {
    //     Some(self.global_metadata.get(&func.local_metadata, mdnode_id as usize)?)
    // }
}

#[track_caller]
fn mdstring_opt(md: Option<Arc<MetadataRecord>>) -> Option<Arc<MetadataRecord>> {
    let m = md?;
    // mdstrings are never forward refs
    debug_assert!(matches!(*m, MetadataRecord::String(_) | MetadataRecord::UnresolvedReference(_)), "expected string, got {m:#?}");
    Some(m)
}
