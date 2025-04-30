use llvm_bitcode::BitStreamReader;
use llvm_bitcode::bitcode::Signature;
use llvm_bitcode::schema::records::*;
use llvm_bitcode::schema::values::*;
use std::collections::{HashMap, HashSet};

pub mod parse_records {
    pub mod collector;
}

use parse_records::collector::*;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
struct Pos {
    function_idx: u32,
    bb_idx: BbId,
}

struct IntraCfg<'b> {
    module: &'b BcCollector<'b>,
    cache: HashMap<Pos, bool>, // TODO: true should point to the noreturn fn or unreachable being called (multiple possible due to select)
    function_index: HashMap<&'b str, u32>,
    noreturn_prototypes: HashSet<&'b str>,
}

impl<'b> IntraCfg<'b> {
    fn new(module: &'b BcCollector<'b>) -> Self {
        let mut function_index = HashMap::new();
        for (idx, f) in module.functions.iter().enumerate() {
            let name = module.strtab(&f.record.symbol_strtab_range).unwrap();
            function_index.insert(name, idx as u32);
        }

        let mut noreturn_prototypes = HashSet::new();
        for f in &module.function_prototypes {
            let name = module.strtab(&f.symbol_strtab_range).unwrap();
            if module
                .function_attributes(f)
                .any(|at| matches!(at, Attribute::AttrKind(AttrKind::NoReturn)))
            {
                noreturn_prototypes.insert(name);
            }
        }

        Self {
            module,
            cache: HashMap::new(),
            function_index,
            noreturn_prototypes,
        }
    }

    fn dbg(&self, di: &DebugInstruction, func: &Function, recursion: &mut Vec<*const MetadataRecord>) {
        match di {
            DebugInstruction::Loc(loc) => {
                eprintln!("loc = {loc:?}");
                if let Some(s) = &loc.inlined_at { self.dbg_node_id("inlined_at", s, func, recursion); }
                if let Some(s) = &loc.scope { self.dbg_node_id("scope", s, func, recursion); }
            }
            // DebugInstruction::RecordValue(debug_record_value) => todo!(),
            // DebugInstruction::RecordDeclare(debug_record_declare) => todo!(),
            // DebugInstruction::RecordAssign(debug_record_assign) => todo!(),
            // DebugInstruction::RecordValueSimple(debug_record_value_simple) => todo!(),
            DebugInstruction::RecordLabel(label) => {
                self.dbg_node_id("di_location", &label.di_location, func, recursion);
                self.dbg_node_id("di_label", &label.di_label, func, recursion);
            }
            _ => {}
        }
    }

    fn dbg_node_id(&self, label: &str, di: &MetadataRecord, func: &Function, recursion: &mut Vec<*const MetadataRecord>) {
        let ptr: *const MetadataRecord = di;
        if recursion.contains(&ptr) {
            return;
        }
        recursion.push(ptr);

        eprint!(
            "{:>indent$} {label} => ",
            "•",
            indent = (2 * recursion.len())
        );
        match di {
            MetadataRecord::String(s) => {
                eprintln!("{s:?}");
            }
            MetadataRecord::Node(node) => {
                eprintln!("node");
                for op in node.operands.iter().flatten() {
                    self.dbg_node_id("op", op, func, recursion);
                }
            }
            MetadataRecord::DILocation(dilocation) => {
                eprintln!("{}:{}", dilocation.loc.line, dilocation.loc.column);
                if let Some(s) = &dilocation.loc.inlined_at { self.dbg_node_id("inlined_at", s, func, recursion); }
                if let Some(s) = &dilocation.loc.scope { self.dbg_node_id("scope", s, func, recursion); }
            }
            // MetadataRecord::DISubrange(disubrange) => todo!(),
            // MetadataRecord::DIGenericSubrange(digeneric_subrange) => todo!(),
            // MetadataRecord::DIEnumerator(dienumerator) => todo!(),
            // MetadataRecord::DIBasicType(dibasic_type) => todo!(),
            // MetadataRecord::DIStringType(distring_type) => todo!(),
            MetadataRecord::DIDerivedType(block) => {
                eprintln!("type line #{}", block.line);
                if let Some(id) = &block.file { self.dbg_node_id("file", id, func, recursion); }
                if let Some(id) = &block.scope { self.dbg_node_id("type scope", id, func, recursion); }
            }
            // MetadataRecord::DICompositeType(dicomposite_type) => todo!(),
            MetadataRecord::DISubroutineType(s) => {
                eprintln!("subroutine");
                if let Some(id) = &s.type_array { self.dbg_node_id("type_array", id, func, recursion); }
            }
            MetadataRecord::DIFile(f) => {
                eprintln!("file");
                if let Some(id) = &f.filename { self.dbg_node_id("filename", id, func, recursion); }
                if let Some(id) = &f.directory { self.dbg_node_id("directory", id, func, recursion); }
            }
            // MetadataRecord::DICompileUnit(dicompile_unit) => todo!(),
            MetadataRecord::DISubprogram(s) => {
                eprintln!("subprog line {}; scope line {}", s.line, s.scope_line);
                if let Some(s) = &s.name { self.dbg_node_id("name", s, func, recursion); }
                if let Some(s) = &s.linkage_name { self.dbg_node_id("linkage_name", s, func, recursion); }
                if let Some(s) = &s.file { self.dbg_node_id("file", s, func, recursion); }
                if let Some(s) = &s.scope { self.dbg_node_id("sub scope", s, func, recursion); }
            }
            MetadataRecord::DILexicalBlock(block) => {
                eprintln!("block line {}:{}", block.line, block.column);
                if let Some(id) = &block.file { self.dbg_node_id("file", id, func, recursion); }
                if let Some(id) = &block.scope { self.dbg_node_id("bl scope", id, func, recursion); }
            }
            // MetadataRecord::DILexicalBlockFile(dilexical_block_file) => todo!(),
            // MetadataRecord::DICommonBlock(dicommon_block) => todo!(),
            MetadataRecord::DINamespace(dinamespace) => {
                eprintln!("ns");
                if let Some(id) = &dinamespace.name { self.dbg_node_id("name", id, func, recursion); }
                if let Some(id) = &dinamespace.scope { self.dbg_node_id("ns scope", id, func, recursion); }
            }
            // MetadataRecord::DIMacro(dimacro) => todo!(),
            // MetadataRecord::DIMacroFile(dimacro_file) => todo!(),
            // MetadataRecord::DIArgList(diarg_list) => todo!(),
            // MetadataRecord::DIModule(dimodule) => todo!(),
            // MetadataRecord::DIAssignID(diassign_id) => todo!(),
            MetadataRecord::DITemplateTypeParameter(t) => {
                eprintln!("<>");
                if let Some(id) = &t.name { self.dbg_node_id("name", id, func, recursion) }
                if let Some(id) = &t.type_id { self.dbg_node_id("type_id", id, func, recursion) }
            }
            // MetadataRecord::DITemplateValueParameter(ditemplate_value_parameter) => todo!(),
            // MetadataRecord::DIGlobalVariable(diglobal_variable) => todo!(),
            MetadataRecord::DILocalVariable(v) => {
                eprintln!("var line {}", v.line);
                if let Some(id) = &v.name { self.dbg_node_id("name", id, func, recursion) }
                if let Some(id) = &v.file { self.dbg_node_id("file", id, func, recursion) }
                if let Some(id) = &v.scope { self.dbg_node_id("var scope", id, func, recursion) }
            }
            MetadataRecord::DILabel(v) => {
                eprintln!("label line {}", v.line);
                if let Some(id) = &v.name { self.dbg_node_id("name", id, func, recursion) }
                if let Some(id) = &v.file { self.dbg_node_id("file", id, func, recursion) }
                if let Some(id) = &v.scope { self.dbg_node_id("label scope", id, func, recursion) }
            }
            // MetadataRecord::DIExpression(diexpression) => todo!(),
            // MetadataRecord::DIGlobalVariableExpression(diglobal_variable_expression) => todo!(),
            // MetadataRecord::DIObjCProperty(diobj_cproperty) => todo!(),
            // MetadataRecord::DIImportedEntity(diimported_entity) => todo!(),
            other => {
                eprintln!("[dbg]: {other:#?}");
            }
        }
        recursion.pop();
    }

    // TODO: can also scan for alloc!!
    // call to extern? fn __rust_alloc_zeroed?
    // call to extern? fn __rust_alloc?
    // call to extern? fn __rust_dealloc?
    // call to extern? fn __rust_realloc?
    //
    // TODO: it's a call graph. It can build a call graph for anything.
    pub fn is_noreturn(&mut self, pos: Pos) -> bool {
        if let Some(res) = self.cache.get(&pos).copied() {
            return res;
        }

        let Pos { function_idx, bb_idx } = pos;

        let func = &self.module.functions[function_idx as usize];
        let is_noreturn = self
            .module
            .function_attributes(&func.record)
            .any(|at| matches!(at, Attribute::AttrKind(AttrKind::NoReturn)));

        // insert even if false, in case the later bb scan has recursion
        // TODO: use worklist to iterate fixpoint?
        self.cache.insert(pos, is_noreturn);

        if is_noreturn {
            return true;
        }

        let bb = &func.basic_blocks[bb_idx.0 as usize];
        for bbinst in &bb.instructions {
            if self.is_inst_noreturn(func, function_idx, &bbinst.inst) {
                return true;
            }
        }
        false
    }

    // fn bb_idx(&self, func: &Function, val_id: BbId) -> BbId {
    //     let val = self
    //         .module
    //         .get_value(func, val_id)
    //         .expect("invalid val id for bb");
    //     val.numeric_value
    //         .expect("val for bb has no constant")
    //         .try_into()
    //         .expect("bbid")
    // }

    fn is_inst_noreturn(&mut self, func: &Function, function_idx: u32, inst: &Inst) -> bool {
        match inst {
            Inst::Br(InstBr::Uncond { dest_bb }) => self.is_noreturn(Pos {
                function_idx,
                bb_idx: (*dest_bb),
            }),
            Inst::Br(InstBr::Cond {
                true_bb, false_bb, ..
            }) => {
                self.is_noreturn(Pos {
                    function_idx,
                    bb_idx: (*true_bb),
                }) && self.is_noreturn(Pos {
                    function_idx,
                    bb_idx: (*false_bb),
                })
            }
            Inst::CatchSwitch(inst) => {
                if let Some(bb_idx) = inst.unwind_dest {
                    self.is_noreturn(Pos { function_idx, bb_idx: (bb_idx) });
                }
                false
            }
            Inst::CatchRet(inst) => self.is_noreturn(Pos {
                function_idx,
                bb_idx: (inst.successor),
            }),
            Inst::Resume(_) => true,
            Inst::Switch(InstSwitch {
                default_bb, cases, ..
            }) => {
                self.is_noreturn(Pos {
                    function_idx,
                    bb_idx: (*default_bb),
                }) && cases.iter().all(|&(_, bb_idx)| {
                    self.is_noreturn(Pos {
                        function_idx,
                        bb_idx: (bb_idx),
                    })
                })
            }
            // TODO: ignore unwind? Flag it as a potential noreturn?
            Inst::Invoke(InstInvoke {
                normal_bb,
                unwind_bb: _,
                ..
            }) => self.is_noreturn(Pos {
                function_idx,
                bb_idx: (*normal_bb),
            }),
            // Inst::Phi(_inst) => continue,
            // Inst::VAArg(_inst) => {}
            // Inst::ExtractVal(_inst) => {}
            // Inst::InsertVal(_inst) => {}
            Inst::IndirectBr(InstIndirectBr { destinations, .. }) => {
                debug_assert!(!destinations.is_empty());
                destinations.iter().all(|&bb_idx| self.is_noreturn(Pos { function_idx, bb_idx: (bb_idx) }))
            },
            Inst::Call(call) => {
                let name = self
                    .module
                    .get_value(func, call.callee_val)
                    .and_then(|val| val.name.as_ref())
                    .and_then(|name_range| self.module.strtab(name_range));
                if let Some(name) = name {
                    if let Some(called_function_idx) = self.function_index.get(name).copied() {
                        // In LLVM 0th basic block is always the entry point
                        return self.is_noreturn(Pos {
                            function_idx: called_function_idx,
                            bb_idx: BbId(0),
                        });
                    } else if self.noreturn_prototypes.contains(name) {
                        return true;
                    }
                }
                false
            }
            Inst::CallBr(call) => {
                // Assuming the call part is uninteresting (needs to be naked asm, won't have noreturn info?)
                self.is_noreturn(Pos { function_idx, bb_idx: (call.normal_bb) }) &&
                    call.indirect_bb.iter().all(|&bb_idx| self.is_noreturn(Pos { function_idx, bb_idx: (bb_idx) }))
            },
            Inst::Ret(_) => false,
            Inst::Unreachable => true,
            _ => {
                debug_assert!(!inst.is_terminator());
                false
            }
        }
    }

    pub fn scan(&mut self) {
        for (function_idx, func) in self.module.functions.iter().enumerate() {
            for (i, m) in func.local_metadata.data.iter().enumerate() {
                eprintln!("meta[{}_f] = {m:#?}", func.local_metadata.first + i);
            }

            let function_idx = function_idx as u32;
            let name = self.module.strtab(&func.record.symbol_strtab_range).unwrap();

            if self.is_noreturn(Pos { function_idx, bb_idx: BbId(0) }) {
                eprintln!("Function {} never returns", demangled(name));
                // continue;
            }

            for (bb_idx, bb) in func.basic_blocks.iter().enumerate() {
                let Some(mut first_inst_idx) = bb.instructions.first().map(|i| i.index) else {
                    continue;
                };
                for bbinst in &bb.instructions {
                    if self.is_inst_noreturn(func, function_idx, &bbinst.inst) {
                        eprintln!(
                            "Function@{function_idx} {}@{bb_idx} has a noreturn inst since {first_inst_idx}..{}",
                            demangled(name),
                            bbinst.index
                        );

                        let inst_range = first_inst_idx..=bbinst.index;
                        for dbg in func.debug_metadata.iter().filter(|dbg| inst_range.contains(&dbg.index)) {
                            self.dbg(&dbg.di, func, &mut vec![]);
                        }

                        // for idx in inst_range  {
                        //     if let Some(meta) = func.inst_metadata_attachment.get(idx) {
                        //         eprintln!("meta = {meta:?}");
                        //     }
                        // }
                        // TODO: break only if the debug info is useful, otherwise keep going to find unreachable inst
                        break;
                    }
                    match &bbinst.inst {
                        Inst::Br(InstBr::Cond { .. })
                        | Inst::Switch(_)
                        | Inst::IndirectBr(_)
                        | Inst::Resume(_)
                        | Inst::Select(_)
                        | Inst::LandingPad(_)
                        | Inst::CatchRet(_)
                        | Inst::CleanupRet(_)
                        | Inst::CatchPad(_)
                        | Inst::CleanupPad(_)
                        | Inst::CatchSwitch(_)
                        | Inst::CallBr(_) => {
                            first_inst_idx = bbinst.index;
                        }
                        // noreturn search picks these as a noreturn, but for the lint scan trust they're really unreachable
                        Inst::Unreachable | Inst::Ret(_) => break,
                        _ => {}
                    }
                }
            }
        }
    }
}

fn demangled(name: &str) -> String {
    rustc_demangle::try_demangle(name).map_or_else(|_| name.into(), |s| s.to_string())
}

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("Provide file path to a .bc file");
    let file = std::fs::read(&path).unwrap();

    let mut collector = BcCollector::new();

    let mut reader = BitStreamReader::new();
    let (_, bitcode) = Signature::parse(&file).unwrap();
    let block = reader.iter_bitcode(bitcode);

    collector.iter_outer_block(block).unwrap();

    for (i, m) in collector.global_metadata.data.iter().enumerate() {
        eprintln!("meta[{i}] = {m:#?}");
    }

    let mut cfg = IntraCfg::new(&collector);

    cfg.scan();
}
