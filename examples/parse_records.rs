#[derive(Debug)]
pub struct BBInstruction {
    pub index: InstIndex, // instruction Id
    pub value_id: Option<ValueId>,
    pub inst: Inst,
}

#[derive(Debug)]
pub struct BasicBlock {
    pub name: Option<String>,
    pub inst: Vec<BBInstruction>,
}

#[derive(Debug)]
pub struct Function {
    record: ModuleFunctionRecord,
    /// `ValIDs` lower than this are looked up in the global value list,
    /// and higher ones are in the local value list.
    first_local_value_list_id: ValueId,
    local_value_list: Vec<Value>,
    basic_blocks: Vec<BasicBlock>,

    /// This is incremented for all instructions, even `Void` ones.
    /// Debug metadata does not increment this.
    instruction_counter: InstIndex,
    /// function-local ones
    metadata: Vec<MetadataRecord>,
    inst_metadata_attachment: HashMap<InstIndex, Vec<(MetadataKindId, MetadataNodeId)>>,
    fn_metadata_attachment: Vec<(MetadataKindId, MetadataNodeId)>,
}

impl Function {
    pub fn push_value_list(&mut self, value: Value) {
        self.local_value_list.push(value);
    }
    /// In LLVM this is `InstID` variable, but it's not the same counter as instruction IDs for purpose of metadata attachments.
    pub fn next_value_id(&self) -> ValueId {
        self.first_local_value_list_id + self.local_value_list.len() as u32
    }

    pub fn local_value_by_id(&self, id: ValueId) -> Option<&Value> {
        let local_id = id.checked_sub(self.first_local_value_list_id)?;
        self.local_value_list.get(local_id as usize)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ValueKind {
    Function,
    GlobalVariable,
    BasicBlock,
    Instruction,
    Constant,
    Argument,
    Metadata,
    Other,
}

#[derive(Debug)]
pub struct Value {
    pub type_id: TypeId, // Type identifier
    pub kind: ValueKind, // What kind of value this is
}

impl WIP {
    fn parse_constants_block(
        &mut self,
        mut block: BlockIter<'_, 'input>,
        mut func: Option<&mut Function>,
    ) -> Result<(), Error> {
        while let Some(record) = block.next_record()? {
            let Some(con) = self.parse_constants_record(record)? else {
                continue;
            };
            let type_id = con.get_type_id().unwrap_or_else(|| {
                self.types
                    .types
                    .iter()
                    .position(|t| matches!(t, Type::Void))
                    .unwrap() as TypeId
            });
            if let Some(func) = &mut func {
                func.push_value_list(Value {
                    type_id,
                    kind: ValueKind::Constant,
                });
            } else {
                self.global_value_list.push(Value {
                    type_id,
                    kind: ValueKind::Constant,
                });
            }
        }
        Ok(())
    }

    fn parse_function_block(&mut self, mut block: BlockIter<'_, 'input>) -> Result<(), Error> {
        let mut local_value_list = Vec::new();
        let func = self.module_defined_functions.remove(0);

        let fty = self.types.get_fn(func.ty).ok_or(Error::Other("bad fn"))?;

        for (i, arg) in fty.param_types.clone().into_iter().enumerate() {
            local_value_list.push(Value {
                type_id: arg,
                kind: ValueKind::Argument,
            });
        }
        let mut func = Function {
            record: func,
            first_local_value_list_id: self.global_value_list.len() as _,
            local_value_list,
            instruction_counter: 0,
            fn_metadata_attachment: Vec::new(),
            inst_metadata_attachment: HashMap::new(),
            metadata: Vec::new(),
            basic_blocks: Vec::new(),
        };
        while let Some(b) = block.next()? {
            match b {
                BlockItem::Block(mut block) => {
                    match BlockId::try_from(block.id as u8).unwrap() {
                        BlockId::Constants => {
                            self.parse_constants_block(block, Some(&mut func))?;
                        }
                        BlockId::Metadata => {
                            self.parse_metadata_block(block, Some(&mut func))?;
                        }
                        BlockId::MetadataAttachment => {
                            self.parse_metadata_attachment(block, &mut func)?;
                        }
                        BlockId::ValueSymtab => {
                            while let Some(record) = block.next_record()? {
                                vst.push(self.parse_value_symtab_record(record)?);
                            }
                        }
                        block_id => {}
                    };
                }
                BlockItem::Record(record) => {
                    let record_id = record.id;
                    if let Some(r) = self.parse_function_record(record, &mut func)? {
                        let mut terminator = false;
                        match r {
                            TypedRecord::FunctionInst(inst) => {
                                terminator = inst.is_terminator();
                                let void_value = inst.is_void_type(&self.types);
                                let inst_type_id = inst.ret_type_id(&self.types);
                                let mut value_id = None;

                                let index = func.instruction_counter;
                                func.instruction_counter += 1;
                                let mut restypeid = inst_type_id.map(|v| v as i64).unwrap_or(-1);
                                if matches!(inst, Inst::FunctionInstCmp(_)) {
                                    restypeid = -2;
                                }
                                let llvm_basic_type_id = inst_type_id
                                    .and_then(|v| self.types.llvm_basic_type_id(v))
                                    .map(|v| v as i64)
                                    .unwrap_or(7);
                                if !void_value {
                                    value_id = Some(func.next_value_id());
                                    func.push_value_list(Value {
                                        kind: ValueKind::Instruction,
                                        type_id: inst_type_id.expect("nonvoid"),
                                    });
                                }
                                func.basic_blocks
                                    .last_mut()
                                    .unwrap()
                                    .inst
                                    .push(BBInstruction {
                                        index,
                                        value_id,
                                        inst,
                                    });
                            }
                            TypedRecord::FunctionDI(
                                _meta @ (FunctionDI::DebugLoc(_)
                                | FunctionDI::DebugRecordValue(_)
                                | FunctionDI::DebugRecordDeclare(_)
                                | FunctionDI::DebugLocAgain
                                | FunctionDI::DebugRecordValueSimple(_)),
                            ) => {}
                            TypedRecord::FunctionDI(meta) => {
                                func.push_value_list(Value {
                                    kind: ValueKind::Metadata,
                                    type_id: self
                                        .types
                                        .types
                                        .iter()
                                        .position(|t| matches!(t, Type::Metadata))
                                        .unwrap()
                                        as u32,
                                });
                            }
                            _ => {}
                        };

                        if terminator {
                            let bb = &mut func.basic_blocks;
                            bb.push(BasicBlock { inst: Vec::new() });
                        }
                    }
                }
            }
        }
        self.functions.push(func);

        Ok(())
    }

    /// pushValueAndType/getValueTypePair equivalent
    #[track_caller]
    fn value_and_type<'cursor>(
        &mut self,
        record: &mut RecordIter<'cursor, 'input>,
        func: &mut Function,
    ) -> Result<(ValueId, TypeId), Error> {
        self.value_and_type_from_iter(record, func)
    }

    /// pushValueAndType/getValueTypePair equivalent
    #[track_caller]
    fn value_and_type_from_iter<'cursor>(
        &mut self,
        iter: &mut impl Iterator<Item = Result<u64, Error>>,
        func: &mut Function,
    ) -> Result<(ValueId, TypeId), Error> {
        let relative_val_id = iter
            .next()
            .ok_or(Error::EndOfRecord)??
            .try_into()
            .map_err(|_| Error::ValueOverflow)?;
        let next_value_id = func.next_value_id();

        let val_id = next_value_id.checked_sub(relative_val_id).expect("nope");

        // Forward references to values that haven't been processed yet need a type
        let ty = if val_id >= next_value_id {
            iter.next()
                .ok_or(Error::EndOfRecord)??
                .try_into()
                .map_err(|_| Error::ValueOverflow)?
        } else if let Some(v) = func
            .local_value_by_id(val_id)
            .or_else(|| self.global_value_list.get(val_id as usize))
        {
            // For backward references to global values, use the stored type if available
            v.type_id
        };

        Ok((val_id, ty))
    }

    // pushValue/popValue equivalent
    #[track_caller]
    fn value_without_type<'cursor>(
        &mut self,
        record: &mut RecordIter<'cursor, 'input>,
        func: &mut Function,
    ) -> Result<ValueId, Error> {
        let relative_val_id = record.u32()?;
        let next_value_id = func.next_value_id();
        let val_id = next_value_id.checked_sub(relative_val_id).expect("nope");
        Ok(val_id)
    }

    #[track_caller]
    fn value_signed<'cursor>(
        &mut self,
        record: &mut RecordIter<'cursor, 'input>,
        func: &mut Function,
    ) -> Result<ValueId, Error> {
        let relative_val_id = record.i64()?;
        let next_value_id = func.next_value_id();
        let val_id = (next_value_id as i64 - relative_val_id).try_into().unwrap();
        return Ok(val_id);
    }

    fn parse_module_record<'cursor>(
        &mut self,
        mut record: RecordIter<'cursor, 'input>,
    ) -> Result<Option<TypedRecord<'input>>, Error> {
        Ok(Some(match ModuleCode::try_from(record.id as u8).unwrap() {
            ModuleCode::Version => {
                let version = record.u64()?;
                assert!(version >= 2);
                TypedRecord::ModuleVersion(ModuleVersionRecord { version })
            }
            ModuleCode::Triple => TypedRecord::ModuleTriple(ModuleTripleRecord {
                triple: record.string()?.try_into().unwrap(),
            }),
            ModuleCode::Datalayout => TypedRecord::ModuleDataLayout(ModuleDataLayoutRecord {
                datalayout: record.string()?.try_into().unwrap(),
            }),
            ModuleCode::Asm => TypedRecord::ModuleAsm(ModuleAsmRecord {
                asm: record.string()?.try_into().unwrap(),
            }),
            ModuleCode::SectionName => TypedRecord::ModuleSectionName(ModuleSectionNameRecord {
                section_name: record.string()?.try_into().unwrap(),
            }),
            ModuleCode::Deplib => TypedRecord::ModuleDepLib(ModuleDepLibRecord {
                deplib_name: record.string()?.try_into().unwrap(),
            }),
            ModuleCode::GlobalVar => {
                let name = record.range()?;
                let type_id = record.u32()?;
                let flags = record.u32()?;
                let init_id = record.nzu64()?;
                let linkage = Linkage::try_from(record.u8()?).unwrap();
                let alignment = record.nzu32()?;
                let section = record.nzu32()?;
                let var = if !record.is_empty() {
                    ModuleGlobalVarRecord {
                        name,
                        type_id,
                        flags,
                        init_id,
                        linkage,
                        alignment,
                        section,
                        visibility: record.u8()?,
                        thread_local: record.u8()?,
                        unnamed_addr: record.nzu8()?,
                        dll_storage_class: DllStorageClass::try_from(record.u8()?).unwrap(),
                        comdat: record.nzu64()?,
                        attributes: record.nzu32()?,
                        dso_local: record.bool()?,
                        global_sanitizer: record.nzu32()?,
                        partition: record.range()?,
                        code_model: record.u32()?,
                    }
                } else {
                    ModuleGlobalVarRecord {
                        name,
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
                };
                assert!(self.functions.is_empty());
                self.global_value_list.push(Value {
                    type_id: type_id,
                    kind: ValueKind::GlobalVariable,
                });
                TypedRecord::ModuleGlobalVar(var)
            }
            ModuleCode::Function => {
                assert!(record.len() >= 18);
                let fun = ModuleFunctionRecord {
                    name: record.range()?,
                    ty: record.u32()?,
                    calling_conv: CallConv::try_from(record.u8()?).unwrap(),
                    is_proto: record.bool()?,
                    linkage: Linkage::try_from(record.u8()?).unwrap(),
                    attributes: record.nzu32()?,
                    alignment: record.nzu32()?,
                    section: record.nzu32()?,
                    visibility: record.u8()?,
                    gc: record.nzu64()?,
                    unnamed_addr: record.nzu8()?,
                    prologue_data: record.nzu64()?,
                    dll_storage_class: DllStorageClass::try_from(record.u8()?).unwrap(),
                    comdat: record.nzu64()?,
                    prefix_data: record.nzu64()?,
                    personality_fn: record.nzu64()?,
                    dso_local: record.bool()?,
                    address_space: record.u64()?,
                    partition_name: record.range()?,
                };
                assert!(self.functions.is_empty());
                self.global_value_list.push(Value {
                    type_id: fun.ty,
                    kind: ValueKind::Function,
                });
                if !fun.is_proto {
                    self.module_defined_functions.push(fun);
                    return Ok(None);
                }
                TypedRecord::ModuleFunction(fun)
            }
            ModuleCode::AliasOld => TypedRecord::ModuleAlias(ModuleAliasRecord {
                name: record.range()?,
                alias_type: record.u32()?,
                aliasee_val: record.u32()?,
                linkage: record.u64()?,
                visibility: record.u8()?,
                dll_storage_class: DllStorageClass::try_from(record.u8()?).unwrap(),
                threadlocal: record.u8()?,
                unnamed_addr: record.nzu8()?,
                preemption_specifier: record.u64()?,
            }),
            ModuleCode::GCName => TypedRecord::ModuleGCName(ModuleGCNameRecord {
                gc_name: record.string()?.try_into().unwrap(),
            }),
            ModuleCode::VstOffset => {
                return Ok(None);
            }
            ModuleCode::SourceFilename => {
                TypedRecord::ModuleSourceFilename(record.string()?.try_into().unwrap())
            }
            ModuleCode::Hash => {
                TypedRecord::ModuleHash(ModuleHashRecord(std::array::from_fn(|_| {
                    record.u32().unwrap_or_default()
                })))
            }
            _ => self.parse_generic_record(record)?,
        }))
    }

    fn parse_attributes_record<'cursor>(
        &mut self,
        record: RecordIter<'cursor, 'input>,
    ) -> Result<TypedRecord<'input>, Error> {
        Ok(TypedRecord::Attributes(
            record.collect::<Result<Vec<_>, _>>().unwrap(),
        ))
    }

    fn parse_constants_record<'cursor>(
        &mut self,
        mut record: RecordIter<'cursor, 'input>,
    ) -> Result<Option<ConstantRecord>, Error> {
        let id = ConstantsCodes::try_from(record.id as u8).unwrap();
        let ty = self.constants_current_type.unwrap_or_else(|| panic!());
        let con = match id {
            ConstantsCodes::Settype => {
                self.constants_current_type = Some(record.u32()?);
                return Ok(None);
            }
            ConstantsCodes::Null => ConstantRecord::ConstantNull,
            ConstantsCodes::Undef => ConstantRecord::ConstantUndef,
            ConstantsCodes::Integer => ConstantRecord::ConstantInteger(ConstantInteger {
                ty,
                value: record.i64()?,
            }),
            ConstantsCodes::WideInteger => {
                ConstantRecord::ConstantWideInteger(ConstantWideInteger {
                    ty,
                    values: record.collect::<Result<Vec<_>, _>>()?,
                })
            }
            ConstantsCodes::Float => ConstantRecord::ConstantFloat(ConstantFloat {
                ty,
                value: f64::from_bits(record.u64()?),
            }),
            ConstantsCodes::Aggregate => ConstantRecord::ConstantAggregate(ConstantAggregate {
                ty,
                values: record.collect::<Result<Vec<_>, _>>()?,
            }),
            ConstantsCodes::String | ConstantsCodes::Data => {
                ConstantRecord::ConstantString(ConstantString {
                    ty,
                    value: record.string()?,
                })
            }
            ConstantsCodes::CString => ConstantRecord::ConstantCString(ConstantCString {
                ty,
                value: record.string()?.try_into().unwrap(),
            }),
            ConstantsCodes::BinOp => ConstantRecord::ConstantBinaryOp(ConstantBinaryOp {
                ty,
                opcode: record.try_from::<u8, _>()?,
                lhs: record.u32()?,
                rhs: record.u32()?,
                flags: record.next()?.unwrap_or(0) as u8,
            }),
            ConstantsCodes::Cast => ConstantRecord::ConstantCast(ConstantCast {
                opcode: record.try_from::<u8, _>()?,
                ty: record.u32()?,
                operand: record.u32()?,
            }),
            id @ (ConstantsCodes::Gep | ConstantsCodes::GepWithInrange) => {
                let base_type = record.u32()?;
                let flags = record.u8()?;
                let inrange = if id == ConstantsCodes::GepWithInrange {
                    record.nzu64()?
                } else {
                    None
                };
                let mut operands = Vec::with_capacity(record.len() / 2);
                while record.len() >= 2 {
                    operands.push((record.u32()?, record.u32()?));
                }
                ConstantRecord::ConstantGEP(ConstantGEP {
                    ty,
                    base_type,
                    flags,
                    inrange,
                    operands,
                })
            }
            ConstantsCodes::Select => ConstantRecord::ConstantSelect(ConstantSelect {
                ty,
                condition: record.u64()?,
                true_value: record.u64()?,
                false_value: record.u64()?,
            }),
            ConstantsCodes::ExtractElt => {
                ConstantRecord::ConstantExtractElement(ConstantExtractElement {
                    operand_ty: record.u32()?,
                    operand_val: record.u32()?,
                    index_ty: record.u32()?,
                    index_val: record.u32()?,
                })
            }
            ConstantsCodes::InsertElt => {
                ConstantRecord::ConstantInsertElement(ConstantInsertElement {
                    ty,
                    operand_type: record.u32()?,
                    vector: record.u64()?,
                    element: record.u64()?,
                    index: record.u64()?,
                })
            }
            ConstantsCodes::ShuffleVec => {
                ConstantRecord::ConstantShuffleVector(ConstantShuffleVector {
                    ty,
                    vector1: record.u64()?,
                    vector2: record.u64()?,
                    mask: record.u64()?,
                })
            }
            ConstantsCodes::Cmp => ConstantRecord::ConstantCompare(ConstantCompare {
                ty,
                operand_type: record.u32()?,
                lhs: record.u64()?,
                rhs: record.u64()?,
                predicate: record.u8()?,
            }),
            ConstantsCodes::BlockAddress => {
                ConstantRecord::ConstantBlockAddress(ConstantBlockAddress {
                    ty,
                    function: record.u64()?,
                    block: record.u64()?,
                })
            }
            ConstantsCodes::InlineAsm => ConstantRecord::ConstantInlineASM(ConstantInlineASM {
                ty,
                function_type: record.u32()?,
                flags: record.u8()?,
                asm: record.string()?.try_into().unwrap(),
                constraints: record.string()?.try_into().unwrap(),
            }),
            ConstantsCodes::Poison => ConstantRecord::ConstantPoison,
            ConstantsCodes::DsoLocalEquivalent => {
                ConstantRecord::ConstantDSOLocalEquivalent(ConstantDSOLocalEquivalent {
                    ty,
                    gv_type: record.u32()?,
                    gv: record.u64()?,
                })
            }
            ConstantsCodes::NoCfiValue => ConstantRecord::ConstantNoCFI(ConstantNoCFI {
                ty,
                function_type: record.u32()?,
                function: record.u64()?,
            }),
            ConstantsCodes::PtrAuth => ConstantRecord::ConstantPtrAuth(ConstantPtrAuth {
                ty,
                pointer: record.u64()?,
                key: record.u64()?,
                discriminator: record.u64()?,
                address_discriminator: record.u64()?,
            }),
            ConstantsCodes::ShufVecEx => todo!(),
            ConstantsCodes::InboundsGep => todo!(),
            ConstantsCodes::UnOp => todo!(),
            other => unimplemented!("{other:?} constant"),
        };
        Ok(Some(con))
    }

    fn parse_function_record<'cursor>(
        &mut self,
        mut record: RecordIter<'cursor, 'input>,
        func: &mut Function,
    ) -> Result<Option<TypedRecord<'input>>, Error> {
        Ok(Some(
            match FunctionCode::try_from(record.id as u8).unwrap() {
                FunctionCode::DeclareBlocks => {
                    let num = record.u64()?;
                    let bb = &mut func.basic_blocks;
                    bb.reserve_exact(num as _);
                    if bb.is_empty() {
                        bb.push(BasicBlock { inst: Vec::new() });
                    }
                    return Ok(None);
                }
                // this is still relevant, in addition to metadata
                FunctionCode::DebugLocAgain => TypedRecord::FunctionDI(FunctionDI::DebugLocAgain),
                // this is still relevant, in addition to metadata
                FunctionCode::DebugLoc => {
                    TypedRecord::FunctionDI(FunctionDI::DebugLoc(FunctionDebugLoc {
                        line: record.u64()?,
                        column: record.u64()?,
                        scope_id: record.u64()?,
                        ia_val: record.u32()?,
                        implicit_code: record.u64()?,
                    }))
                }
                FunctionCode::OperandBundle => {
                    TypedRecord::FunctionOperandBundle(FunctionOperandBundle {
                        tag_id: record.u64()?,
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
                    TypedRecord::FunctionBlockAddrUsers(FunctionBlockAddrUsers(
                        record
                            .map(|r| r.map(|u| u as u32))
                            .collect::<Result<Vec<_>, _>>()?,
                    ))
                }
                FunctionCode::DebugRecordValue => TypedRecord::FunctionDI(
                    FunctionDI::DebugRecordValue(FunctionDebugRecordValue {
                        di_location: record.u64()?,
                        di_local_variable: record.u64()?,
                        di_expression: record.u64()?,
                        value_as_metadata: record.u64()?,
                    }),
                ),
                FunctionCode::DebugRecordDeclare => TypedRecord::FunctionDI(
                    FunctionDI::DebugRecordDeclare(FunctionDebugRecordDeclare {
                        di_location: record.u64()?,
                        di_local_variable: record.u64()?,
                        di_expression: record.u64()?,
                        value_as_metadata: record.u64()?,
                    }),
                ),
                FunctionCode::DebugRecordAssign => TypedRecord::FunctionDI(
                    FunctionDI::DebugRecordAssign(FunctionDebugRecordAssign {
                        di_location: record.u64()?,
                        di_local_variable: record.u64()?,
                        di_expression: record.u64()?,
                        value_as_metadata: record.u64()?,
                        di_assign_id: record.u64()?,
                        di_expression_addr: record.u64()?,
                        value_as_metadata_addr: record.u64()?,
                    }),
                ),
                FunctionCode::DebugRecordValueSimple => {
                    return Ok(Some(TypedRecord::FunctionDI(
                        FunctionDI::DebugRecordValueSimple(FunctionDebugRecordValueSimple {
                            di_location: record.u64()?,
                            di_local_variable: record.u64()?,
                            di_expression: record.u64()?,
                            value: record.u64()?,
                        }),
                    )));
                }
                FunctionCode::DebugRecordLabel => TypedRecord::FunctionDI(
                    FunctionDI::DebugRecordLabel(FunctionDebugRecordLabel {
                        di_location: record.u64()?,
                        di_label: record.u64()?,
                    }),
                ),
                _ => {
                    let inst = match FunctionCode::try_from(record.id as u8).unwrap() {
                        FunctionCode::BinOp => {
                            let (operand_val, operand_ty) =
                                self.value_and_type(&mut record, func)?;
                            let operand2_val = self.value_without_type(&mut record, func)?;

                            let opcode = BinOpcode::try_from(record.u8()?)
                                .map_err(|_| Error::Other("bad binop"))?;
                            let op_vals = [operand_val, operand2_val];
                            let flags = record.next()?.unwrap_or(0) as u8;
                            if record.len() >= 4 {
                                // uint8_t Flag
                            }
                            let inst = Inst::FunctionInstBinOp(FunctionInstBinOp {
                                opcode,
                                op_vals,
                                operand_ty,
                                flags,
                            });
                            inst
                        }
                        FunctionCode::Cast => {
                            let (operand_val, operand_ty) =
                                self.value_and_type(&mut record, func)?;
                            let inst = Inst::FunctionInstCast(FunctionInstCast {
                                operand_ty,
                                operand_val,
                                result_ty: record.u32()?,
                                opcode: CastOpcode::try_from(record.u8()?)
                                    .map_err(|_| Error::Other("bad cast"))?,
                            });
                            inst
                        }
                        FunctionCode::ExtractElt => {
                            Inst::FunctionInstExtractElt(FunctionInstExtractElt {
                                vector_ty: record.u32()?,
                                vector_val: record.u32()?,
                                index_val: record.u32()?,
                            })
                        }
                        FunctionCode::InsertElt => {
                            Inst::FunctionInstInsertElt(FunctionInstInsertElt {
                                vector_ty: record.u32()?,
                                vector_val: record.u32()?,
                                element_val: record.u32()?,
                                index_val: record.u32()?,
                            })
                        }
                        FunctionCode::ShuffleVec => {
                            Inst::FunctionInstShuffleVec(FunctionInstShuffleVec {
                                vector_ty: record.u32()?,
                                lhs_val: record.u32()?,
                                rhs_val: record.u32()?,
                                mask_val: record.u32()?,
                            })
                        }
                        FunctionCode::Ret => {
                            let (return_val, return_ty) = if !record.is_empty() {
                                self.value_and_type(&mut record, func)
                                    .map(|(a, b)| (Some(a), Some(b)))?
                            } else {
                                let fty = self
                                    .types
                                    .get_fn(func.record.ty)
                                    .ok_or(Error::Other("bad fn"))?;
                                (None, fty.ret_ty)
                            };
                            Inst::FunctionInstRet(FunctionInstRet {
                                return_ty,
                                return_val,
                            })
                        }
                        FunctionCode::Br => {
                            // this is bb index, not val_id
                            let bb1 = record.u32()?;
                            Inst::FunctionInstBr(if !record.is_empty() {
                                FunctionInstBr::Cond {
                                    true_bb: bb1,
                                    false_bb: record.u32()?,
                                    condition_val: self.value_without_type(&mut record, func)?,
                                }
                            } else {
                                FunctionInstBr::Uncond { dest_bb: bb1 }
                            })
                        }
                        FunctionCode::Switch => {
                            let condition_ty = record.u32()?;
                            let condition_val = self.value_without_type(&mut record, func)?;
                            let default_bb = record.u32()?;
                            let num_cases = record.len() / 2;
                            let mut cases = Vec::with_capacity(num_cases);
                            for _ in 0..num_cases {
                                let value = record.u32()?;
                                let target_bb = record.u32()?;
                                cases.push((value, target_bb));
                            }
                            Inst::FunctionInstSwitch(FunctionInstSwitch {
                                condition_ty,
                                condition_val,
                                default_bb,
                                cases,
                            })
                        }
                        // can be void
                        FunctionCode::Invoke => Inst::FunctionInstInvoke(FunctionInstInvoke {
                            attr: record.u64()?,
                            callee_val: record.u32()?, // val id
                            normal_bb: record.u32()?,  // not val id
                            unwind_bb: record.u32()?,
                            function_ty: record.u32()?,
                            args: record.collect::<Result<Vec<_>, _>>()?, // val id
                        }),
                        FunctionCode::Unreachable => {
                            assert!(record.is_empty());
                            Inst::FunctionInstUnreachable
                        }
                        FunctionCode::Phi => {
                            let ty = record.u32()?;
                            let mut incoming = Vec::new();
                            while record.len() >= 2 {
                                let incoming_val = self.value_signed(&mut record, func)?;
                                let incoming_bb = record.u32()?;
                                incoming.push((incoming_val, incoming_bb));
                            }
                            let flags = record.next()?.unwrap_or(0) as u8;
                            Inst::FunctionInstPhi(FunctionInstPhi {
                                ty,
                                incoming,
                                flags,
                            })
                        }
                        FunctionCode::Alloca => Inst::FunctionInstAlloca(FunctionInstAlloca {
                            result_ty: record.u32()?,
                            array_size_ty: record.u32()?,
                            array_size_val: record.u32()?,
                            alignment: record.u64()?,
                        }),
                        id @ (FunctionCode::Load | FunctionCode::LoadAtomic) => {
                            let is_atomic = matches!(id, FunctionCode::LoadAtomic);
                            let (ptr_val, ptr_ty) = self.value_and_type(&mut record, func)?;
                            Inst::FunctionInstLoad(FunctionInstLoad {
                                ptr_ty,
                                ptr_val,
                                ret_ty: record.u32()?,
                                alignment: record.u64()?,
                                is_volatile: record.bool()?,
                                atomic: if is_atomic {
                                    Some((record.try_from::<u8, _>()?, record.u64()?))
                                } else {
                                    None
                                },
                            })
                        }
                        FunctionCode::VaArg => Inst::FunctionInstVAArg(FunctionInstVAArg {
                            valist_ty: record.u32()?,
                            valist_val: record.u32()?,
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

                            Inst::FunctionInstStore(FunctionInstStore {
                                ptr_ty,
                                ptr_val,
                                stored_val,
                                stored_ty,
                                alignment,
                                is_volatile,
                                atomic: if is_atomic {
                                    Some((record.try_from::<u8, _>()?, record.u64()?))
                                } else {
                                    None
                                },
                            })
                        }
                        FunctionCode::ExtractVal => {
                            let (val, ty) = self.value_and_type(&mut record, func)?;
                            Inst::FunctionInstExtractVal(FunctionInstExtractVal {
                                ty,
                                val,
                                operands: record.collect::<Result<Vec<_>, _>>()?,
                            })
                        }
                        FunctionCode::InsertVal => {
                            let (aggregate_val, aggregate_ty) =
                                self.value_and_type(&mut record, func)?;
                            let (element_val, element_ty) =
                                self.value_and_type(&mut record, func)?;
                            let indices = record.array()?;
                            Inst::FunctionInstInsertVal(FunctionInstInsertVal {
                                aggregate_ty,
                                aggregate_val,
                                element_ty,
                                element_val,
                                indices,
                            })
                        }
                        FunctionCode::Cmp2 | FunctionCode::Cmp => {
                            let (val_id, ty) = self.value_and_type(&mut record, func)?;
                            Inst::FunctionInstCmp(FunctionInstCmp {
                                operand_ty: ty,
                                lhs_val: val_id,
                                rhs_val: record.u32()?,
                                predicate: record.u64()?,
                                flags: record.next()?.unwrap_or(0),
                            })
                        }
                        FunctionCode::SelectOld => {
                            // obsolete opcode
                            let mut condition_ty = None;
                            for (i, t) in self.types.types.iter().enumerate() {
                                if matches!(t, Type::Integer { width: n } if n.get() == 1) {
                                    condition_ty = Some(i as u32);
                                    break;
                                }
                            }
                            let (true_val, result_ty) = self.value_and_type(&mut record, func)?;
                            let false_val = self.value_without_type(&mut record, func)?;
                            let condition_val = self.value_without_type(&mut record, func)?;
                            Inst::FunctionInstSelect(FunctionInstSelect {
                                result_ty,
                                condition_ty: condition_ty.unwrap(),
                                condition_val,
                                true_val,
                                false_val,
                                flags: 0,
                            })
                        }
                        FunctionCode::Vselect => {
                            let (true_val, result_ty) = self.value_and_type(&mut record, func)?;
                            let false_val = self.value_without_type(&mut record, func)?;
                            let (condition_val, condition_ty) =
                                self.value_and_type(&mut record, func)?;
                            let flags = record.next()?.unwrap_or(0) as u8;
                            let inst = Inst::FunctionInstSelect(FunctionInstSelect {
                                result_ty,
                                true_val,
                                false_val,
                                condition_ty,
                                condition_val,
                                flags,
                            });
                            dbg!(inst)
                        }
                        FunctionCode::IndirectBr => {
                            Inst::FunctionInstIndirectBr(FunctionInstIndirectBr {
                                ptr_ty: record.u32()?,
                                address_val: self.value_without_type(&mut record, func)?,
                                destinations: record
                                    .map(|v| v.map(|v| v as u32))
                                    .collect::<Result<Vec<_>, _>>()?,
                            })
                        }
                        // can be void
                        FunctionCode::Call => {
                            let attr = record.nzu32()?;
                            let calling_conv_flags = record.u64()?;
                            let math_flags = if (calling_conv_flags >> 17) & 1 != 0 {
                                record.u8()?
                            } else {
                                0
                            };
                            let explicit_type = (calling_conv_flags >> 15) & 1 != 0;

                            let mut function_ty = if explicit_type { record.u32()? } else { 0 };
                            let (callee_val, callee_ty) = self.value_and_type(&mut record, func)?;
                            if !explicit_type {
                                function_ty = callee_ty;
                            }

                            let _fty = self
                                .types
                                .get_fn(function_ty)
                                .ok_or(Error::Other("bad fn"))?;

                            Inst::FunctionInstCall(FunctionInstCall {
                                attr,
                                calling_conv: CallConv::from_flags(calling_conv_flags).unwrap(),
                                math_flags,
                                function_ty,
                                callee_val,
                                callee_ty,
                                args: self.function_args(function_ty, &mut record, func)?,
                            })
                        }
                        FunctionCode::Fence => Inst::FunctionInstFence(FunctionInstFence {
                            ordering: record.try_from::<u8, _>()?,
                            synch_scope: record.u64()?,
                        }),
                        FunctionCode::Resume => {
                            let (exception_val, exception_ty) =
                                self.value_and_type(&mut record, func)?;
                            Inst::FunctionInstResume(FunctionInstResume {
                                exception_val,
                                exception_ty,
                            })
                        }
                        FunctionCode::GepOld => unimplemented!(),
                        FunctionCode::Gep => {
                            let flags = record.u8()?;
                            let source_type = record.u32()?;

                            // this is weird format with vbr6 array not vbr6 fields
                            let record_payload = record.array()?;
                            let mut record_payload = record_payload.into_iter().map(Ok);

                            let (base_ptr, base_ty) =
                                self.value_and_type_from_iter(&mut record_payload, func)?;

                            let mut operands = Vec::with_capacity(record.len() / 2);
                            while record_payload.len() > 0 {
                                operands.push(
                                    self.value_and_type_from_iter(&mut record_payload, func)?,
                                );
                            }
                            let inst = Inst::FunctionInstGep(FunctionInstGep {
                                base_ptr,
                                base_ty,
                                flags,
                                source_type,
                                operands,
                            });
                            inst
                        }

                        FunctionCode::Cmpxchg | FunctionCode::CmpxchgOld => {
                            let (ptr_val, ptr_ty) = self.value_and_type(&mut record, func)?;
                            let (cmp_val, cmp_ty) = self.value_and_type(&mut record, func)?;
                            let new_val = self.value_without_type(&mut record, func)?;
                            Inst::FunctionInstCmpXchg(FunctionInstCmpXchg {
                                ptr_ty,
                                ptr_val,
                                cmp_val,
                                cmp_ty,
                                new_val,
                                is_volatile: record.bool()?,
                                success_ordering: record.try_from::<u8, _>()?,
                                synch_scope: record.u64()?,
                                failure_ordering: record.try_from::<u8, _>()?,
                                is_weak: record.bool()?,
                                alignment: record.u64()?,
                            })
                        }
                        FunctionCode::LandingPad | FunctionCode::LandingPadOld => {
                            let result_ty = record.u32()?;
                            let is_cleanup = record.u32()?;
                            let num_clauses = record.u64()? as usize;
                            let mut clauses = Vec::with_capacity(num_clauses);
                            for _ in 0..num_clauses {
                                // catch or filter
                                clauses
                                    .push((record.u32()?, self.value_and_type(&mut record, func)?));
                            }
                            Inst::FunctionInstLandingPad(FunctionInstLandingPad {
                                result_ty,
                                is_cleanup,
                                clauses,
                            })
                        }
                        FunctionCode::CatchRet => {
                            Inst::FunctionInstCatchRet(FunctionInstCatchRet {
                                catch_pad: record.u32()?,
                                successor: record.u32()?,
                            })
                        }
                        id @ (FunctionCode::CatchPad | FunctionCode::CleanupPad) => {
                            let parent_pad = self.value_without_type(&mut record, func)?;
                            let num_args = record.u64()? as usize;
                            let mut args = Vec::with_capacity(num_args);
                            for _ in 0..num_args {
                                args.push(self.value_and_type(&mut record, func)?);
                            }
                            if matches!(id, FunctionCode::CatchPad) {
                                Inst::FunctionInstCatchPad(FunctionInstCatchPad {
                                    parent_pad,
                                    args,
                                })
                            } else {
                                Inst::FunctionInstCleanupPad(FunctionInstCleanupPad {
                                    parent_pad,
                                    args,
                                })
                            }
                        }
                        FunctionCode::CatchSwitch => {
                            let parent_pad = self.value_without_type(&mut record, func)?;
                            let num_args = record.u64()? as usize;
                            let mut args = Vec::with_capacity(num_args);
                            for _ in 0..num_args {
                                args.push(record.u32()?);
                            }
                            Inst::FunctionInstCatchSwitch(FunctionInstCatchSwitch {
                                parent_pad,
                                args,
                                unwind_dest: record.next()?.map(|u| u as ValueId),
                            })
                        }
                        FunctionCode::UnOp => {
                            let (operand_val, operand_ty) =
                                self.value_and_type(&mut record, func)?;
                            Inst::FunctionInstUnOp(FunctionInstUnOp {
                                operand_ty,
                                operand_val,
                                opcode: record.u8()?,
                                flags: record.next()?.unwrap_or(0) as u8,
                            })
                        }
                        FunctionCode::CallBr => {
                            let attr = record.u64()?;
                            let calling_conv_flags = record.u64()?;
                            let explicit_type = (calling_conv_flags >> 15) & 1 != 0;
                            let mut function_ty = if explicit_type { record.u32()? } else { 0 };
                            let (callee_val, callee_ty) = self.value_and_type(&mut record, func)?;
                            if !explicit_type {
                                function_ty = callee_ty;
                            }
                            let _ty = self
                                .types
                                .get_fn(function_ty)
                                .ok_or(Error::Other("bad fn"))?;
                            Inst::FunctionInstCallBr(FunctionInstCallBr {
                                attr,
                                calling_conv: CallConv::from_flags(calling_conv_flags).unwrap(),
                                normal_bb: record.u32()?,
                                indirect_bb: (0..record.u64()?)
                                    .map(|_| record.u32())
                                    .collect::<Result<Vec<_>, _>>()?,
                                function_ty,
                                callee_val,
                                callee_ty,
                                args: self.function_args(function_ty, &mut record, func)?,
                            })
                        }
                        FunctionCode::Freeze => {
                            let (operand_val, operand_ty) =
                                self.value_and_type(&mut record, func)?;
                            Inst::FunctionInstFreeze(FunctionInstFreeze {
                                operand_ty,
                                operand_val,
                            })
                        }
                        FunctionCode::AtomicRmw | FunctionCode::AtomicRmwOld => {
                            let (ptr_val, ptr_ty) = self.value_and_type(&mut record, func)?;
                            let (stored_val, val_ty) = self.value_and_type(&mut record, func)?;
                            Inst::FunctionInstAtomicRmw(FunctionInstAtomicRmw {
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
                    TypedRecord::FunctionInst(inst)
                }
            },
        ))
    }

    fn parse_identification_record<'cursor>(
        &mut self,
        mut record: RecordIter<'cursor, 'input>,
    ) -> Result<TypedRecord<'input>, Error> {
        match record.id {
            1 => Ok(TypedRecord::IdentificationString(
                record.string()?.try_into().unwrap(),
            )),
            2 => Ok(TypedRecord::IdentificationEpoch(
                record.next().unwrap().unwrap(),
            )),
            _ => self.parse_generic_record(record),
        }
    }

    fn parse_global_value_summary<'cursor>(
        &mut self,
        mut record: RecordIter<'cursor, 'input>,
    ) -> Result<TypedRecord<'input>, Error> {
        match GlobalValueSummaryCode::try_from(record.id as u8).unwrap() {
            GlobalValueSummaryCode::PerModuleGlobalvarInitRefs => Ok(
                TypedRecord::PerModuleGlobalVarInitRefs(PerModuleGlobalVarInitRefsRecord {
                    value_id: record.u32()?,
                    flags: record.u64()?,
                    init_refs: record.array()?.into_iter().map(|u| u as u32).collect(),
                }),
            ),
            _ => Ok(self.parse_generic_record(record)?),
        }
    }

    fn parse_operand_bundle_tag<'cursor>(
        &mut self,
        mut record: RecordIter<'cursor, 'input>,
    ) -> Result<TypedRecord<'input>, Error> {
        Ok(TypedRecord::OperandBundleTag(
            record.string()?.try_into().unwrap(),
        ))
    }

    fn parse_symtab_blob<'cursor>(
        &mut self,
        mut record: RecordIter<'cursor, 'input>,
    ) -> Result<TypedRecord<'input>, Error> {
        Ok(TypedRecord::SymtabBlob(SymtabBlobRecord {
            blob: record.blob()?,
        }))
    }

    fn parse_value_symtab_record<'cursor>(
        &mut self,
        mut record: RecordIter<'cursor, 'input>,
    ) -> Result<ValueSymtab, Error> {
        Ok(match ValueSymtabCode::try_from(record.id as u8).unwrap() {
            ValueSymtabCode::Entry => ValueSymtab::Entry(ValueSymtabEntryRecord {
                value_id: record.u32()?,
                name: record.string()?.try_into().unwrap(),
            }),
            ValueSymtabCode::BbEntry => ValueSymtab::Bbentry(ValueSymtabBbentryRecord {
                id: record.u32()?,
                name: record.string()?.try_into().unwrap(),
            }),
            ValueSymtabCode::FnEntry => {
                // unused
                ValueSymtab::Fnentry(ValueSymtabFnentryRecord {
                    linkage_value_id: record.u32()?,
                    function_offset: record.u64()?,
                    name: record.string().map(|e| String::try_from(e).unwrap()).ok(),
                })
            }
            // Obsolete
            ValueSymtabCode::CombinedEntry => {
                ValueSymtab::CombinedEntry(ValueSymtabCombinedEntryRecord {
                    linkage_value_id: record.u32()?,
                    refguid: record.u64()?,
                })
            }
            _ => unimplemented!(),
        })
    }

    fn parse_sync_scope_name<'cursor>(
        &mut self,
        mut record: RecordIter<'cursor, 'input>,
    ) -> Result<TypedRecord<'input>, Error> {
        Ok(TypedRecord::SyncScopeName(
            record.string()?.try_into().unwrap(),
        ))
    }

    fn parse_generic_record<'cursor>(
        &mut self,
        record: RecordIter<'cursor, 'input>,
    ) -> Result<TypedRecord<'input>, Error> {
        Ok(TypedRecord::Generic(
            record.id,
            record.collect::<Result<Vec<_>, _>>()?,
        ))
    }

    //  [attr_index, fnty, callee, arg0... argN, flags].
    //  this only gets arg0..argN
    fn function_args<'cursor>(
        &mut self,
        function_ty: TypeId,
        record: &mut RecordIter<'cursor, 'input>,
        func: &mut Function,
    ) -> Result<Vec<CallArg>, Error> {
        let ty = self
            .types
            .get_fn(function_ty)
            .ok_or(Error::Other("bad function type"))?;

        let arg_types = ty.param_types.as_slice();
        let mut args = Vec::with_capacity(arg_types.len());
        for &arg_ty in arg_types {
            let ty = self.types.get(arg_ty).ok_or(Error::Other("bad arg"))?;
            let id = record.u32()?;
            args.push(if matches!(ty, Type::Label) {
                CallArg::Label(id)
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

    pub fn parse_param_attr_grp_record(
        &mut self,
        mut record: RecordIter<'_, 'input>,
    ) -> Result<AttributeGroupEntry, Error> {
        let group_id = record.u64()?;
        let index = record.u64()?;
        let mut attributes = Vec::new();

        while let Some(id) = record.next()? {
            attributes.push(match ParamAttrGrpCodes::try_from(id as u8).unwrap() {
                ParamAttrGrpCodes::EnumAttr => {
                    // Enum attribute (e.g., AlwaysInline, NoInline)
                    Attribute::AttrKind(AttrKind::try_from(record.u8()?).unwrap())
                }
                ParamAttrGrpCodes::IntAttr => {
                    // Integer attribute (e.g., Alignment, StackAlignment)
                    let kind = AttrKind::try_from(record.u8()?).unwrap();
                    let value = record.u64()?;
                    Attribute::Int { kind, value }
                }
                id @ (ParamAttrGrpCodes::StringAttr | ParamAttrGrpCodes::StringAttrWithValue) => {
                    // String attribute
                    let key = record.zstring()?;
                    let mut value = None;
                    if matches!(id, ParamAttrGrpCodes::StringAttrWithValue) {
                        value = Some(record.zstring()?);
                    }
                    Attribute::String { key, value }
                }
                id @ (ParamAttrGrpCodes::TypeAttr | ParamAttrGrpCodes::TypeAttrTypeId) => {
                    // Type attribute (e.g., ByVal)
                    let kind = AttrKind::try_from(record.u8()?).unwrap();
                    let type_id = if matches!(id, ParamAttrGrpCodes::TypeAttrTypeId) {
                        Some(record.u64()?)
                    } else {
                        None
                    };
                    Attribute::Type { kind, type_id }
                }
                // 7
                ParamAttrGrpCodes::ConstantRange => {
                    // Constant range attribute
                    let kind = AttrKind::try_from(record.u8()?).unwrap();
                    let bit_width = record.u32()?;
                    let lower = record.i64()?;
                    let upper = record.i64()?;
                    Attribute::ConstantRange {
                        bit_width,
                        kind,
                        range: lower..upper,
                    }
                }
                // 8
                ParamAttrGrpCodes::ConstantRangeList => {
                    // Constant range list attribute
                    let kind = AttrKind::try_from(record.u8()?).unwrap();
                    let num_ranges = record.u64()?;
                    let bit_width = record.u32()?;
                    let mut ranges = Vec::new();
                    for _ in 0..num_ranges {
                        let lower = record.i64()?;
                        let upper = record.i64()?;
                        ranges.push(lower..upper);
                    }
                    Attribute::ConstantRangeList {
                        kind,
                        bit_width,
                        ranges,
                    }
                }
            });
        }

        Ok(AttributeGroupEntry {
            group_id,
            index,
            attributes,
        })
    }

    pub fn process_vst(&mut self, vst: Vec<ValueSymtab>, mut func: Option<&mut Function>) {
        for v in vst {
            match v {
                ValueSymtab::Entry(r) => {
                    let val_id = r.value_id as usize;
                    let name = r.name;
                    let global_end = func
                        .as_ref()
                        .map(|f| f.first_local_value_list_id as usize)
                        .unwrap_or(self.global_value_list.len());
                    if val_id < global_end {
                        let tmp = self.global_value_list.get_mut(val_id).expect(&name);
                        tmp.name = Some(name);
                    } else {
                        let func = func.as_mut().unwrap();
                        let local_id = val_id - global_end;
                        let local_list = &mut func.local_value_list;
                        let Some(tmp) = local_list.get_mut(local_id) else {
                            continue;
                        };
                        tmp.name = Some(name);
                    }
                }
                ValueSymtab::Bbentry(b) => {
                    func.as_mut().unwrap().basic_blocks[b.id as usize].name = Some(b.name);
                }
                ValueSymtab::Fnentry(r) => {
                    if r.name.is_some() {
                        panic!("strtab replaced fn vst {r:?}");
                    }
                }
                ValueSymtab::CombinedEntry(_) => unimplemented!(),
            }
        }
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
                TypeCode::Integer => Type::Integer {
                    width: record.nzu8()?.unwrap(),
                },
                TypeCode::Pointer => return Err(Error::Other("obsolete")),
                TypeCode::FunctionOld => unimplemented!(),
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
                    is_packed: record.next().unwrap().unwrap() != 0,
                    element_types: record
                        .array()?
                        .into_iter()
                        .map(|t| t as u32)
                        .collect::<Vec<_>>(),
                }),
                TypeCode::StructName => {
                    name = Some(record.string()?.try_into().unwrap());
                    continue;
                }
                TypeCode::StructNamed => Type::Struct(TypeStructRecord {
                    name: name.take(),
                    is_packed: record.next().unwrap().unwrap() != 0,
                    element_types: record
                        .array()?
                        .into_iter()
                        .map(|t| t as u32)
                        .collect::<Vec<_>>(),
                }),
                TypeCode::Function => {
                    let vararg = record.bool()?;
                    let mut array = record
                        .array()?
                        .into_iter()
                        .enumerate()
                        .map(|(i, v)| (i, v as u32));
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
                }
                TypeCode::OpaquePointer => Type::OpaquePointer(TypeOpaquePointerRecord {
                    address_space: record.u64()?,
                }),
                TypeCode::Token => Type::Token,
                c => unimplemented!("type {c:?}"),
            };
            self.types.types.push(ty);
        }
        Ok(())
    }

    pub fn parse_metadata_attachment(
        &mut self,
        mut block: BlockIter<'_, 'input>,
        func: &mut Function,
    ) -> Result<(), Error> {
        while let Some(mut record) = block.next_record()? {
            assert_eq!(record.id, MetadataCode::Attachment as _);
            let num_items = record.len() / 2; // round down

            // instruction if present, function otherwise
            let dest = if record.len() % 2 != 0 {
                let inst_index = record.u64()? as InstIndex;

                func.inst_metadata_attachment
                    .entry(inst_index)
                    .and_modify(|f| f.reserve(num_items))
                    .or_insert_with(|| Vec::with_capacity(num_items))
            } else {
                func.fn_metadata_attachment.reserve(num_items);
                &mut func.fn_metadata_attachment
            };

            for _ in 0..num_items {
                let md_kind: MetadataKindId = record.u32()?;
                // these may be forward refs
                // MDStringRef + GlobalMetadataBitPosIndex
                let md_node: MetadataNodeId = record.u32()?;
                dest.push((md_kind, md_node));
            }
        }
        Ok(())
    }

    pub fn parse_metadata_block(
        &mut self,
        mut block: BlockIter<'_, 'input>,
        mut func: Option<&mut Function>,
    ) -> Result<(), Error> {
        while let Some(mut record) = block.next_record()? {
            use llvm_bitcode::dumper::metadata::*;

            let m = match MetadataCode::try_from(record.id as u8).unwrap() {
                MetadataCode::StringOld => {
                    MetadataRecord::String(record.string()?.try_into().unwrap())
                }
                MetadataCode::Value => MetadataRecord::Value(MetadataValue {
                    type_id: record.u32()?,
                    value_id: record.u32()?,
                }),
                id @ (MetadataCode::Node | MetadataCode::DistinctNode) => {
                    // NODE: [n x md num] – non-distinct MDNode.
                    MetadataRecord::Node(MetadataNode {
                        distinct: matches!(id, MetadataCode::DistinctNode),
                        operands: record.collect::<Result<Vec<_>, _>>()?,
                    })
                }
                MetadataCode::Name => MetadataRecord::Name(record.string()?.try_into().unwrap()),
                MetadataCode::Location => {
                    // LOCATION: [distinct, line, column, scope, inlined_at?, is_implicit_code]
                    let distinct = record.bool()?;
                    let line = record.u32()?; // assume u32 fields
                    let column = record.u32()?;
                    let scope = record.u64()?;
                    let inlined_at = {
                        let v = record.u64()?;
                        if v == 0 { None } else { Some(v) }
                    };
                    let implicit_code = record.bool()?;
                    MetadataRecord::DILocation(DILocation {
                        distinct,
                        line,
                        column,
                        scope,
                        inlined_at,
                        implicit_code,
                    })
                }
                MetadataCode::NamedNode => MetadataRecord::NamedNode(MetadataNamedNode {
                    mdnodes: record.collect::<Result<Vec<_>, _>>()?,
                }),
                MetadataCode::Attachment => unreachable!("is in a separate block"),
                MetadataCode::GenericDebug => MetadataRecord::DIGenericNode(DIGenericNode {
                    distinct: record.bool()?,
                    tag: record.u32()?,
                    version: record.u32()?,
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
                        raw_name: record.next()?,
                        value: record.array()?.into_iter().map(|v| v as i64).collect(),
                    })
                }
                MetadataCode::BasicType => MetadataRecord::DIBasicType(DIBasicType {
                    distinct: record.bool()?,
                    tag: record.u32()?,
                    raw_name: record.next()?,
                    size_in_bits: record.u64()?,
                    align_in_bits: record.u64()?,
                    encoding: record.u64()?,
                    flags: record.u64()?,
                }),
                MetadataCode::File => MetadataRecord::DIFile(DIFile {
                    distinct: record.bool()?,
                    raw_filename: record.next()?,
                    raw_directory: record.next()?,
                    checksum_kind: record.nzu64()?,
                    raw_checksum: record.next()?,
                    raw_source: record.next()?,
                }),
                MetadataCode::DerivedType => MetadataRecord::DIDerivedType(DIDerivedType {
                    distinct: record.bool()?,
                    tag: record.u32()?,
                    raw_name: record.next()?,
                    file: record.next()?,
                    line: record.u32()?,
                    scope: record.next()?,
                    base_type: record.next()?,
                    size_in_bits: record.u64()?,
                    align_in_bits: record.u64()?,
                    offset_in_bits: record.u64()?,
                    flags: record.u64()?,
                    extra_data: record.next()?,
                    dwarf_address_space: record.next()?,
                    annotations: record.next()?,
                    ptr_auth_data: record.u64()?,
                }),
                MetadataCode::CompositeType => MetadataRecord::DICompositeType(DICompositeType {
                    distinct: (record.u8()? & 1) != 0,
                    tag: record.u32()?,
                    raw_name: record.next()?,
                    file: record.next()?,
                    line: record.u32()?,
                    scope: record.next()?,
                    base_type: record.next()?,
                    size_in_bits: record.u64()?,
                    align_in_bits: record.u64()?,
                    offset_in_bits: record.u64()?,
                    flags: record.u64()?,
                    elements: record.next()?,
                    runtime_lang: record.u64()?,
                    vtable_holder: record.next()?,
                    template_params: record.next()?,
                    raw_identifier: record.next()?,
                    discriminator: record.next()?,
                    raw_data_location: record.next()?,
                    raw_associated: record.next()?,
                    raw_allocated: record.next()?,
                    raw_rank: record.next()?,
                    annotations: record.next()?,
                }),
                MetadataCode::SubroutineType => {
                    MetadataRecord::DISubroutineType(DISubroutineType {
                        distinct: (record.u8()? & 1) != 0,
                        flags: record.u64()?,
                        type_array: record.u64()?,
                        cc: record.u64()?,
                    })
                }
                MetadataCode::CompileUnit => MetadataRecord::DICompileUnit(DICompileUnit {
                    distinct: record.bool()?,
                    source_language: record.u32()?,
                    file: record.next()?,
                    raw_producer: record.next()?,
                    is_optimized: record.bool()?,
                    raw_flags: record.next()?,
                    runtime_version: record.u32()?,
                    raw_split_debug_filename: record.next()?,
                    emission_kind: record.u32()?,
                    enum_types: record.next()?,
                    retained_types: record.next()?,
                    subprograms: record.u64()?,
                    global_variables: record.next()?,
                    imported_entities: record.next()?,
                    dwo_id: record.u64()?,
                    macros: record.next()?,
                    split_debug_inlining: record.u32()?,
                    debug_info_for_profiling: record.u32()?,
                    name_table_kind: record.u32()?,
                    ranges_base_address: record.u64()?,
                    raw_sysroot: record.next()?,
                    raw_sdk: record.next()?,
                }),
                MetadataCode::Subprogram => {
                    let composite = record.u64()?; // contains several packed flags
                    MetadataRecord::DISubprogram(DISubprogram {
                        distinct: (composite & 1) != 0,
                        scope: record.next()?,
                        raw_name: record.next()?,
                        raw_linkage_name: record.next()?,
                        file: record.next()?,
                        line: record.u32()?,
                        type_id: record.next()?,
                        scope_line: record.u32()?,
                        containing_type: record.next()?,
                        sp_flags: record.u64()?,
                        virtual_index: record.u64()?,
                        flags: record.u64()?,
                        raw_unit: record.next()?,
                        template_params: record.next()?,
                        declaration: record.next()?,
                        retained_nodes: record.next()?,
                        this_adjustment: record.u64()?,
                        thrown_types: record.next()?,
                        annotations: record.next()?,
                        raw_target_func_name: record.next()?,
                    })
                }
                MetadataCode::LexicalBlock => {
                    // LEXICAL_BLOCK: [distinct, scope, file, line, column]
                    MetadataRecord::DILexicalBlock(DILexicalBlock {
                        distinct: record.bool()?,
                        scope: record.next()?,
                        file: record.next()?,
                        line: record.u32()?,
                        column: record.u32()?,
                    })
                }
                MetadataCode::LexicalBlockFile => {
                    // LEXICAL_BLOCK_FILE: [distinct, scope, file, discriminator]
                    MetadataRecord::DILexicalBlockFile(DILexicalBlockFile {
                        distinct: record.bool()?,
                        scope: record.next()?,
                        file: record.next()?,
                        discriminator: record.u64()?,
                    })
                }
                MetadataCode::Namespace => {
                    // NAMESPACE: [composite (distinct|export_symbols), scope, raw_name]
                    let composite = record.u64()?;
                    MetadataRecord::DINamespace(DINamespace {
                        distinct: (composite & 1) != 0,
                        export_symbols: (composite & 2) != 0,
                        scope: record.next()?,
                        raw_name: record.next()?,
                    })
                }
                MetadataCode::TemplateType => {
                    // TEMPLATE_TYPE: [distinct, raw_name, type, is_default]
                    MetadataRecord::DITemplateTypeParameter(DITemplateTypeParameter {
                        distinct: record.bool()?,
                        raw_name: record.next()?,
                        type_id: record.next()?,
                        is_default: record.bool()?,
                    })
                }
                MetadataCode::TemplateValue => {
                    // TEMPLATE_VALUE: [distinct, tag, raw_name, type, is_default, raw_value]
                    MetadataRecord::DITemplateValueParameter(DITemplateValueParameter {
                        distinct: record.bool()?,
                        tag: record.u32()?,
                        raw_name: record.next()?,
                        type_id: record.next()?,
                        is_default: record.bool()?,
                        raw_value: record.next()?,
                    })
                }
                MetadataCode::GlobalVar => MetadataRecord::DIGlobalVariable(DIGlobalVariable {
                    distinct: (record.u64()? & 1) != 0,
                    scope: record.next()?,
                    raw_name: record.next()?,
                    raw_linkage_name: record.next()?,
                    file: record.next()?,
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
                    scope: record.next()?,
                    raw_name: record.next()?,
                    file: record.next()?,
                    line: record.u32()?,
                    type_id: record.next()?,
                    arg: record.u64()?,
                    flags: record.u64()?,
                    align_in_bits: record.u64()?,
                    annotations: record.next()?,
                }),
                MetadataCode::Label => MetadataRecord::DILabel(DILabel {
                    distinct: record.bool()?,
                    scope: record.next()?,
                    raw_name: record.next()?,
                    file: record.next()?,
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
                    raw_name: record.next()?,
                    file: record.next()?,
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
                        scope: record.next()?,
                        entity: record.next()?,
                        line: record.u32()?,
                        raw_name: record.next()?,
                        raw_file: record.next()?,
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
                    raw_name: record.next()?,
                    raw_value: record.next()?,
                }),
                MetadataCode::MacroFile => MetadataRecord::DIMacroFile(DIMacroFile {
                    distinct: record.bool()?,
                    macinfo_type: record.u32()?,
                    line: record.u32()?,
                    file: record.next()?,
                    elements: record.next()?,
                }),
                MetadataCode::ArgList => MetadataRecord::DIArgList(DIArgList {
                    args: record.array()?,
                }),
                MetadataCode::AssignId => MetadataRecord::DIAssignID(DIAssignID {
                    distinct: record.bool()?,
                }),
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

                    MetadataRecord::Strings(MetadataStringsRecord {
                        strings: strings.to_vec(),
                        ranges,
                    })
                }
                MetadataCode::IndexOffset | MetadataCode::Index => {
                    // we have no use for offsets
                    return Ok(());
                }
                MetadataCode::GlobalDeclAttachment => {
                    // Implementation for METADATA_GLOBAL_DECL_ATTACHMENT
                    // Structure: [valueid, n x [id, mdnode]]
                    let value_id = record.u32()?;
                    let mut attachments = Vec::new();
                    while let Some(id) = record.next()? {
                        let mdnode = record.u64()?;
                        attachments.push((id, mdnode));
                    }
                    MetadataRecord::GlobalDeclAttachment(MetadataAttachment {
                        value_id,
                        attachments,
                    })
                }
                id => {
                    unimplemented!("metadata {id:?}")
                }
            };
            if let Some(f) = func.as_mut() {
                f.metadata.push(m);
            } else {
                self.metadata.push(m);
            }
        }
        Ok(())
    }
}
