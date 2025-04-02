#![allow(non_camel_case_types)]
use llvm_bitcode::BitStreamReader;
use llvm_bitcode::bitcode::Signature;
use llvm_bitcode::read::BlockItem;
use llvm_bitcode::read::BlockIter;
use llvm_bitcode::read::Error;
use num_enum::TryFromPrimitive;

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("Provide file path to a .bc file");
    let file = std::fs::read(&path).unwrap();

    let mut reader = BitStreamReader::new();
    let (sig, bitcode) = Signature::parse(&file).unwrap();

    println!(
        "<BITCODE_WRAPPER_HEADER Magic=0x{:08x} Version=0x{:08x} Offset=0x{:08x} Size=0x{:08x} CPUType=0x{:08x}/>",
        sig.magic, sig.version, sig.offset, sig.size, sig.cpu_type
    );
    iter_block(reader.iter_bitcode(bitcode), 0).unwrap();
}

fn iter_block(mut block: BlockIter, depth: usize) -> Result<(), Error> {
    let outer_block_id = block.id;
    while let Some(b) = block.next()? {
        match b {
            BlockItem::Block(b) => {
                let tag_name = block_tag_name(b.id as _);
                println!(
                    "{:indent$}<{tag_name} NumWords={nw} BlockCodeSize={ab}>",
                    "",
                    nw = b.debug_data_len().unwrap_or(0) / 4,
                    ab = b.debug_abbrev_width(),
                    indent = depth * 2
                );
                iter_block(b, depth + 1)?;
                println!("{:indent$}</{tag_name}>", "", indent = depth * 2);
            }
            BlockItem::Record(mut r) => {
                let tag_name = record_tag_name(outer_block_id as _, r.id as _);
                print!("{:indent$}<{tag_name}", "", indent = depth * 2);
                if let Some(a) = r.debug_abbrev_id() {
                    print!(" abbrevid={a}");
                }
                let fields = r
                    .by_ref()
                    .map(|f| f.map(|f| f as i64))
                    .collect::<Result<Vec<_>, _>>()?;
                for (i, &op) in fields.iter().enumerate() {
                    print!(" op{i}={op}");
                }
                let payload: Result<_, _> = r.payload();
                match payload {
                    Ok(Some(llvm_bitcode::bitcode::Payload::Array(a))) => {
                        for (i, op) in a.iter().enumerate() {
                            print!(" op{}={op}", i + fields.len());
                        }
                        if !a.is_empty() && a.iter().all(|&c| (c as u8) >= 0x20 && (c as u8) < 0x7F)
                        {
                            // lol bug in the original
                            let s: String = a.iter().map(|&c| c as u8 as char).collect();
                            println!("/> record string = '{s}'");
                        } else {
                            println!("/>");
                        }
                    }
                    Ok(Some(llvm_bitcode::bitcode::Payload::Char6String(s))) => {
                        for (i, op) in s.chars().enumerate() {
                            print!(" op{}={}", i + fields.len(), op as u32);
                        }
                        if s.is_empty() {
                            println!("/>");
                        } else {
                            println!("/> record string = '{s}'");
                        }
                    }
                    Ok(None) => {
                        if r.debug_abbrev_id().is_some()
                            && fields.len() > 1
                            && fields.iter().skip(1).all(|&c| (0x20..0x7F).contains(&c))
                        {
                            let s: String =
                                fields.iter().skip(1).map(|&c| c as u8 as char).collect();
                            println!("/> record string = '{s}'");
                        } else {
                            println!("/>");
                        }
                    }
                    Ok(Some(llvm_bitcode::bitcode::Payload::Blob(b))) => {
                        if b.len() < 10000 && b.iter().all(|&c| (0x20..0x7F).contains(&c)) {
                            println!("/> blob data = {}", String::from_utf8_lossy(&b));
                        } else {
                            print!("/> blob data = ");
                            if b.len() > 50 {
                                print!("unprintable, {} bytes.", b.len());
                            } else {
                                print!("'");
                                for b in b {
                                    print!("{b:02x}");
                                }
                                print!("'");
                            }
                            println!();
                        }
                    }
                    Err(err) => print!("/> payload_err={err}"),
                }
            }
        }
    }
    Ok(())
}

fn block_tag_name(id: u32) -> &'static str {
    use Blocks::*;
    match Blocks::try_from(id).unwrap() {
        MODULE_BLOCK_ID => "MODULE_BLOCK",
        PARAMATTR_BLOCK_ID => "PARAMATTR_BLOCK",
        PARAMATTR_GROUP_BLOCK_ID => "PARAMATTR_GROUP_BLOCK_ID",
        CONSTANTS_BLOCK_ID => "CONSTANTS_BLOCK",
        FUNCTION_BLOCK_ID => "FUNCTION_BLOCK",
        IDENTIFICATION_BLOCK_ID => "IDENTIFICATION_BLOCK_ID",
        VALUE_SYMTAB_BLOCK_ID => "VALUE_SYMTAB",
        METADATA_BLOCK_ID => "METADATA_BLOCK",
        METADATA_ATTACHMENT_ID => "METADATA_ATTACHMENT_BLOCK",
        TYPE_BLOCK_ID_NEW => "TYPE_BLOCK_ID",
        USELIST_BLOCK_ID => "USELIST_BLOCK",
        MODULE_STRTAB_BLOCK_ID => "MODULE_STRTAB_BLOCK",
        GLOBALVAL_SUMMARY_BLOCK_ID => "GLOBALVAL_SUMMARY_BLOCK",
        OPERAND_BUNDLE_TAGS_BLOCK_ID => "OPERAND_BUNDLE_TAGS_BLOCK",
        METADATA_KIND_BLOCK_ID => "METADATA_KIND_BLOCK",
        STRTAB_BLOCK_ID => "STRTAB_BLOCK",
        FULL_LTO_GLOBALVAL_SUMMARY_BLOCK_ID => "FULL_LTO_GLOBALVAL_SUMMARY_BLOCK",
        SYMTAB_BLOCK_ID => "SYMTAB_BLOCK",
        SYNC_SCOPE_NAMES_BLOCK_ID => "UnknownBlock26", //"SYNC_SCOPE_NAMES_BLOCK",
    }
}

fn record_tag_name(block: u32, record: u32) -> &'static str {
    match Blocks::try_from(block).unwrap() {
        Blocks::MODULE_BLOCK_ID => match ModuleCodes::try_from(record).unwrap() {
            ModuleCodes::MODULE_CODE_VERSION => "VERSION",
            ModuleCodes::MODULE_CODE_TRIPLE => "TRIPLE",
            ModuleCodes::MODULE_CODE_DATALAYOUT => "DATALAYOUT",
            ModuleCodes::MODULE_CODE_ASM => "ASM",
            ModuleCodes::MODULE_CODE_SECTIONNAME => "SECTIONNAME",
            ModuleCodes::MODULE_CODE_DEPLIB => "DEPLIB",
            ModuleCodes::MODULE_CODE_GLOBALVAR => "GLOBALVAR",
            ModuleCodes::MODULE_CODE_FUNCTION => "FUNCTION",
            ModuleCodes::MODULE_CODE_ALIAS => "ALIAS",
            ModuleCodes::MODULE_CODE_GCNAME => "GCNAME",
            ModuleCodes::MODULE_CODE_COMDAT => "COMDAT",
            ModuleCodes::MODULE_CODE_VSTOFFSET => "VSTOFFSET",
            ModuleCodes::MODULE_CODE_METADATA_VALUES_UNUSED => "METADATA_VALUES_UNUSED",
            ModuleCodes::MODULE_CODE_SOURCE_FILENAME => "SOURCE_FILENAME",
            ModuleCodes::MODULE_CODE_HASH => "HASH",
            ModuleCodes::MODULE_CODE_ALIAS_OLD | ModuleCodes::MODULE_CODE_IFUNC => todo!(),
        },
        Blocks::IDENTIFICATION_BLOCK_ID => match IdentificationCodes::try_from(record).unwrap() {
            IdentificationCodes::IDENTIFICATION_CODE_STRING => "STRING",
            IdentificationCodes::IDENTIFICATION_CODE_EPOCH => "EPOCH",
        },
        crate::Blocks::PARAMATTR_GROUP_BLOCK_ID | crate::Blocks::PARAMATTR_BLOCK_ID => {
            match AttributeCodes::try_from(record).unwrap() {
                AttributeCodes::PARAMATTR_CODE_ENTRY_OLD => "ENTRY_OLD",
                AttributeCodes::PARAMATTR_CODE_ENTRY => "ENTRY",
                AttributeCodes::PARAMATTR_GRP_CODE_ENTRY => "ENTRY",
            }
        }
        Blocks::TYPE_BLOCK_ID_NEW => match TypeCodes::try_from(record).unwrap() {
            TypeCodes::TYPE_CODE_NUMENTRY => "NUMENTRY",
            TypeCodes::TYPE_CODE_VOID => "VOID",
            TypeCodes::TYPE_CODE_FLOAT => "FLOAT",
            TypeCodes::TYPE_CODE_DOUBLE => "DOUBLE",
            TypeCodes::TYPE_CODE_LABEL => "LABEL",
            TypeCodes::TYPE_CODE_OPAQUE => "OPAQUE",
            TypeCodes::TYPE_CODE_INTEGER => "INTEGER",
            TypeCodes::TYPE_CODE_POINTER => "POINTER",
            TypeCodes::TYPE_CODE_HALF => "HALF",
            TypeCodes::TYPE_CODE_ARRAY => "ARRAY",
            TypeCodes::TYPE_CODE_VECTOR => "VECTOR",
            TypeCodes::TYPE_CODE_X86_FP80 => "X86_FP80",
            TypeCodes::TYPE_CODE_FP128 => "FP128",
            TypeCodes::TYPE_CODE_PPC_FP128 => "PPC_FP128",
            TypeCodes::TYPE_CODE_METADATA => "METADATA",
            TypeCodes::TYPE_CODE_X86_MMX => "X86_MMX",
            TypeCodes::TYPE_CODE_STRUCT_ANON => "STRUCT_ANON",
            TypeCodes::TYPE_CODE_STRUCT_NAME => "STRUCT_NAME",
            TypeCodes::TYPE_CODE_STRUCT_NAMED => "STRUCT_NAMED",
            TypeCodes::TYPE_CODE_FUNCTION => "FUNCTION",
            TypeCodes::TYPE_CODE_TOKEN => "TOKEN",
            TypeCodes::TYPE_CODE_BFLOAT => "BFLOAT",
            TypeCodes::TYPE_CODE_FUNCTION_OLD => "FUNCTION_OLD",
            TypeCodes::TYPE_CODE_X86_AMX => "X86_AMX",
            TypeCodes::TYPE_CODE_OPAQUE_POINTER => "UnknownCode25", //"OPAQUE_POINTER",
            TypeCodes::TYPE_CODE_TARGET_TYPE => "TARGET_TYPE",
        },
        Blocks::CONSTANTS_BLOCK_ID => match ConstantsCodes::try_from(record).unwrap() {
            ConstantsCodes::CST_CODE_SETTYPE => "SETTYPE",
            ConstantsCodes::CST_CODE_NULL => "NULL",
            ConstantsCodes::CST_CODE_UNDEF => "UNDEF",
            ConstantsCodes::CST_CODE_INTEGER => "INTEGER",
            ConstantsCodes::CST_CODE_WIDE_INTEGER => "WIDE_INTEGER",
            ConstantsCodes::CST_CODE_FLOAT => "FLOAT",
            ConstantsCodes::CST_CODE_AGGREGATE => "AGGREGATE",
            ConstantsCodes::CST_CODE_STRING => "STRING",
            ConstantsCodes::CST_CODE_CSTRING => "CSTRING",
            ConstantsCodes::CST_CODE_CE_BINOP => "CE_BINOP",
            ConstantsCodes::CST_CODE_CE_CAST => "CE_CAST",
            ConstantsCodes::CST_CODE_CE_GEP => "CE_GEP",
            ConstantsCodes::CST_CODE_CE_INBOUNDS_GEP => "CE_INBOUNDS_GEP",
            ConstantsCodes::CST_CODE_CE_SELECT => "CE_SELECT",
            ConstantsCodes::CST_CODE_CE_EXTRACTELT => "CE_EXTRACTELT",
            ConstantsCodes::CST_CODE_CE_INSERTELT => "CE_INSERTELT",
            ConstantsCodes::CST_CODE_CE_SHUFFLEVEC => "CE_SHUFFLEVEC",
            ConstantsCodes::CST_CODE_CE_CMP => "CE_CMP",
            ConstantsCodes::CST_CODE_INLINEASM => "INLINEASM",
            ConstantsCodes::CST_CODE_CE_SHUFVEC_EX => "CE_SHUFVEC_EX",
            ConstantsCodes::CST_CODE_CE_UNOP => "CE_UNOP",
            ConstantsCodes::CST_CODE_DSO_LOCAL_EQUIVALENT => "DSO_LOCAL_EQUIVALENT",
            ConstantsCodes::CST_CODE_NO_CFI_VALUE => "NO_CFI_VALUE",
            ConstantsCodes::CST_CODE_PTRAUTH => "PTRAUTH",
            ConstantsCodes::CST_CODE_BLOCKADDRESS => "BLOCKADDRESS",
            ConstantsCodes::CST_CODE_DATA => "DATA",
            ConstantsCodes::CST_CODE_CE_GEP_OLD => "CE_GEP_OLD",
            ConstantsCodes::CST_CODE_INLINEASM_OLD => "INLINEASM_OLD",
            ConstantsCodes::CST_CODE_INLINEASM_OLD2 => "INLINEASM_OLD2",
            ConstantsCodes::CST_CODE_CE_GEP_WITH_INRANGE_INDEX_OLD => {
                "CE_GEP_WITH_INRANGE_INDEX_OLD"
            }
            ConstantsCodes::CST_CODE_POISON => "UnknownCode26", //"POISON",
            ConstantsCodes::CST_CODE_INLINEASM_OLD3 => "INLINEASM_OLD3",
            ConstantsCodes::CST_CODE_CE_GEP_WITH_INRANGE => "CE_GEP_WITH_INRANGE",
        },
        Blocks::FUNCTION_BLOCK_ID => match FunctionCodes::try_from(record).unwrap() {
            FunctionCodes::FUNC_CODE_DECLAREBLOCKS => "DECLAREBLOCKS",
            FunctionCodes::FUNC_CODE_INST_BINOP => "INST_BINOP",
            FunctionCodes::FUNC_CODE_INST_CAST => "INST_CAST",
            FunctionCodes::FUNC_CODE_INST_GEP_OLD => "INST_GEP_OLD",
            FunctionCodes::FUNC_CODE_INST_INBOUNDS_GEP_OLD => "INST_INBOUNDS_GEP_OLD",
            FunctionCodes::FUNC_CODE_INST_SELECT => "INST_SELECT",
            FunctionCodes::FUNC_CODE_INST_EXTRACTELT => "INST_EXTRACTELT",
            FunctionCodes::FUNC_CODE_INST_INSERTELT => "INST_INSERTELT",
            FunctionCodes::FUNC_CODE_INST_SHUFFLEVEC => "INST_SHUFFLEVEC",
            FunctionCodes::FUNC_CODE_INST_CMP => "INST_CMP",
            FunctionCodes::FUNC_CODE_INST_RET => "INST_RET",
            FunctionCodes::FUNC_CODE_INST_BR => "INST_BR",
            FunctionCodes::FUNC_CODE_INST_SWITCH => "INST_SWITCH",
            FunctionCodes::FUNC_CODE_INST_INVOKE => "INST_INVOKE",
            FunctionCodes::FUNC_CODE_INST_UNOP => "INST_UNOP",
            FunctionCodes::FUNC_CODE_INST_UNREACHABLE => "INST_UNREACHABLE",
            FunctionCodes::FUNC_CODE_INST_CLEANUPRET => "INST_CLEANUPRET",
            FunctionCodes::FUNC_CODE_INST_CATCHRET => "INST_CATCHRET",
            FunctionCodes::FUNC_CODE_INST_CATCHPAD => "INST_CATCHPAD",
            FunctionCodes::FUNC_CODE_INST_PHI => "INST_PHI",
            FunctionCodes::FUNC_CODE_INST_ALLOCA => "INST_ALLOCA",
            FunctionCodes::FUNC_CODE_INST_LOAD => "INST_LOAD",
            FunctionCodes::FUNC_CODE_INST_VAARG => "INST_VAARG",
            FunctionCodes::FUNC_CODE_INST_STORE => "INST_STORE",
            FunctionCodes::FUNC_CODE_INST_EXTRACTVAL => "INST_EXTRACTVAL",
            FunctionCodes::FUNC_CODE_INST_INSERTVAL => "INST_INSERTVAL",
            FunctionCodes::FUNC_CODE_INST_CMP2 => "INST_CMP2",
            FunctionCodes::FUNC_CODE_INST_VSELECT => "INST_VSELECT",
            FunctionCodes::FUNC_CODE_DEBUG_LOC_AGAIN => "DEBUG_LOC_AGAIN",
            FunctionCodes::FUNC_CODE_INST_CALL => "INST_CALL",
            FunctionCodes::FUNC_CODE_DEBUG_LOC => "DEBUG_LOC",
            FunctionCodes::FUNC_CODE_INST_GEP => "INST_GEP",
            FunctionCodes::FUNC_CODE_OPERAND_BUNDLE => "OPERAND_BUNDLE",
            FunctionCodes::FUNC_CODE_INST_FENCE => "INST_FENCE",
            FunctionCodes::FUNC_CODE_INST_ATOMICRMW => "INST_ATOMICRMW",
            FunctionCodes::FUNC_CODE_INST_LOADATOMIC => "INST_LOADATOMIC",
            FunctionCodes::FUNC_CODE_INST_STOREATOMIC => "INST_STOREATOMIC",
            FunctionCodes::FUNC_CODE_INST_CMPXCHG => "INST_CMPXCHG",
            FunctionCodes::FUNC_CODE_INST_CALLBR => "INST_CALLBR",
            FunctionCodes::FUNC_CODE_BLOCKADDR_USERS => "BLOCKADDR_USERS",
            FunctionCodes::FUNC_CODE_DEBUG_RECORD_DECLARE => "DEBUG_RECORD_DECLARE",
            FunctionCodes::FUNC_CODE_DEBUG_RECORD_VALUE => "DEBUG_RECORD_VALUE",
            FunctionCodes::FUNC_CODE_DEBUG_RECORD_ASSIGN => "DEBUG_RECORD_ASSIGN",
            FunctionCodes::FUNC_CODE_DEBUG_RECORD_VALUE_SIMPLE => "DEBUG_RECORD_VALUE_SIMPLE",
            FunctionCodes::FUNC_CODE_DEBUG_RECORD_LABEL => "DEBUG_RECORD_LABEL",

            FunctionCodes::FUNC_CODE_INST_STORE_OLD => "INST_STORE_OLD",
            FunctionCodes::FUNC_CODE_INST_INDIRECTBR => "INST_INDIRECTBR",
            FunctionCodes::FUNC_CODE_INST_CMPXCHG_OLD => "INST_CMPXCHG_OLD",
            FunctionCodes::FUNC_CODE_INST_ATOMICRMW_OLD => "INST_ATOMICRMW_OLD",
            FunctionCodes::FUNC_CODE_INST_RESUME => "UnknownCode39", //"INST_RESUME",
            FunctionCodes::FUNC_CODE_INST_LANDINGPAD_OLD => "INST_LANDINGPAD_OLD",
            FunctionCodes::FUNC_CODE_INST_STOREATOMIC_OLD => "INST_STOREATOMIC_OLD",
            FunctionCodes::FUNC_CODE_INST_LANDINGPAD => "UnknownCode47", //"INST_LANDINGPAD",
            FunctionCodes::FUNC_CODE_INST_CLEANUPPAD => "INST_CLEANUPPAD",
            FunctionCodes::FUNC_CODE_INST_CATCHSWITCH => "INST_CATCHSWITCH",
            FunctionCodes::FUNC_CODE_INST_FREEZE => "INST_FREEZE",
        },
        Blocks::VALUE_SYMTAB_BLOCK_ID => match ValueSymtabCodes::try_from(record).unwrap() {
            ValueSymtabCodes::VST_CODE_ENTRY => "ENTRY",
            ValueSymtabCodes::VST_CODE_BBENTRY => "BBENTRY",
            ValueSymtabCodes::VST_CODE_FNENTRY => "FNENTRY",
            ValueSymtabCodes::VST_CODE_COMBINED_ENTRY => "COMBINED_ENTRY",
        },
        Blocks::MODULE_STRTAB_BLOCK_ID => match ModulePathSymtabCodes::try_from(record).unwrap() {
            ModulePathSymtabCodes::MST_CODE_ENTRY => "ENTRY",
            ModulePathSymtabCodes::MST_CODE_HASH => "HASH",
        },
        crate::Blocks::GLOBALVAL_SUMMARY_BLOCK_ID
        | crate::Blocks::FULL_LTO_GLOBALVAL_SUMMARY_BLOCK_ID => {
            match GlobalValueSummarySymtabCodes::try_from(record).unwrap() {
                GlobalValueSummarySymtabCodes::FS_PERMODULE => "PERMODULE",
                GlobalValueSummarySymtabCodes::FS_PERMODULE_PROFILE => "PERMODULE_PROFILE",
                GlobalValueSummarySymtabCodes::FS_PERMODULE_RELBF => "PERMODULE_RELBF",
                GlobalValueSummarySymtabCodes::FS_PERMODULE_GLOBALVAR_INIT_REFS => {
                    "PERMODULE_GLOBALVAR_INIT_REFS"
                }
                GlobalValueSummarySymtabCodes::FS_PERMODULE_VTABLE_GLOBALVAR_INIT_REFS => {
                    "PERMODULE_VTABLE_GLOBALVAR_INIT_REFS"
                }
                GlobalValueSummarySymtabCodes::FS_COMBINED => "COMBINED",
                GlobalValueSummarySymtabCodes::FS_COMBINED_PROFILE => "COMBINED_PROFILE",
                GlobalValueSummarySymtabCodes::FS_COMBINED_GLOBALVAR_INIT_REFS => {
                    "COMBINED_GLOBALVAR_INIT_REFS"
                }
                GlobalValueSummarySymtabCodes::FS_ALIAS => "ALIAS",
                GlobalValueSummarySymtabCodes::FS_COMBINED_ALIAS => "COMBINED_ALIAS",
                GlobalValueSummarySymtabCodes::FS_COMBINED_ORIGINAL_NAME => {
                    "COMBINED_ORIGINAL_NAME"
                }
                GlobalValueSummarySymtabCodes::FS_VERSION => "VERSION",
                GlobalValueSummarySymtabCodes::FS_FLAGS => "FLAGS",
                GlobalValueSummarySymtabCodes::FS_TYPE_TESTS => "TYPE_TESTS",
                GlobalValueSummarySymtabCodes::FS_TYPE_TEST_ASSUME_VCALLS => {
                    "TYPE_TEST_ASSUME_VCALLS"
                }
                GlobalValueSummarySymtabCodes::FS_TYPE_CHECKED_LOAD_VCALLS => {
                    "TYPE_CHECKED_LOAD_VCALLS"
                }
                GlobalValueSummarySymtabCodes::FS_TYPE_TEST_ASSUME_CONST_VCALL => {
                    "TYPE_TEST_ASSUME_CONST_VCALL"
                }
                GlobalValueSummarySymtabCodes::FS_TYPE_CHECKED_LOAD_CONST_VCALL => {
                    "TYPE_CHECKED_LOAD_CONST_VCALL"
                }
                GlobalValueSummarySymtabCodes::FS_VALUE_GUID => "VALUE_GUID",
                GlobalValueSummarySymtabCodes::FS_CFI_FUNCTION_DEFS => "CFI_FUNCTION_DEFS",
                GlobalValueSummarySymtabCodes::FS_CFI_FUNCTION_DECLS => "CFI_FUNCTION_DECLS",
                GlobalValueSummarySymtabCodes::FS_TYPE_ID => "TYPE_ID",
                GlobalValueSummarySymtabCodes::FS_TYPE_ID_METADATA => "TYPE_ID_METADATA",
                GlobalValueSummarySymtabCodes::FS_BLOCK_COUNT => "BLOCK_COUNT",
                GlobalValueSummarySymtabCodes::FS_PARAM_ACCESS => "PARAM_ACCESS",
                GlobalValueSummarySymtabCodes::FS_PERMODULE_CALLSITE_INFO => {
                    "PERMODULE_CALLSITE_INFO"
                }
                GlobalValueSummarySymtabCodes::FS_PERMODULE_ALLOC_INFO => "PERMODULE_ALLOC_INFO",
                GlobalValueSummarySymtabCodes::FS_COMBINED_CALLSITE_INFO => {
                    "COMBINED_CALLSITE_INFO"
                }
                GlobalValueSummarySymtabCodes::FS_COMBINED_ALLOC_INFO => "COMBINED_ALLOC_INFO",
                GlobalValueSummarySymtabCodes::FS_STACK_IDS => "STACK_IDS",
                GlobalValueSummarySymtabCodes::FS_ALLOC_CONTEXT_IDS => "ALLOC_CONTEXT_IDS",
                GlobalValueSummarySymtabCodes::FS_CONTEXT_RADIX_TREE_ARRAY => {
                    "CONTEXT_RADIX_TREE_ARRAY"
                }
            }
        }
        crate::Blocks::METADATA_KIND_BLOCK_ID
        | crate::Blocks::METADATA_BLOCK_ID
        | Blocks::METADATA_ATTACHMENT_ID => match MetadataCodes::try_from(record).unwrap() {
            MetadataCodes::METADATA_ATTACHMENT => "ATTACHMENT",
            MetadataCodes::METADATA_STRING_OLD => "STRING_OLD",
            MetadataCodes::METADATA_VALUE => "VALUE",
            MetadataCodes::METADATA_NODE => "NODE",
            MetadataCodes::METADATA_NAME => "NAME",
            MetadataCodes::METADATA_DISTINCT_NODE => "DISTINCT_NODE",
            MetadataCodes::METADATA_KIND => "KIND",
            MetadataCodes::METADATA_LOCATION => "LOCATION",
            MetadataCodes::METADATA_OLD_NODE => "OLD_NODE",
            MetadataCodes::METADATA_OLD_FN_NODE => "OLD_FN_NODE",
            MetadataCodes::METADATA_NAMED_NODE => "NAMED_NODE",
            MetadataCodes::METADATA_GENERIC_DEBUG => "GENERIC_DEBUG",
            MetadataCodes::METADATA_SUBRANGE => "SUBRANGE",
            MetadataCodes::METADATA_ENUMERATOR => "ENUMERATOR",
            MetadataCodes::METADATA_BASIC_TYPE => "BASIC_TYPE",
            MetadataCodes::METADATA_FILE => "FILE",
            MetadataCodes::METADATA_DERIVED_TYPE => "DERIVED_TYPE",
            MetadataCodes::METADATA_COMPOSITE_TYPE => "COMPOSITE_TYPE",
            MetadataCodes::METADATA_SUBROUTINE_TYPE => "SUBROUTINE_TYPE",
            MetadataCodes::METADATA_COMPILE_UNIT => "COMPILE_UNIT",
            MetadataCodes::METADATA_SUBPROGRAM => "SUBPROGRAM",
            MetadataCodes::METADATA_LEXICAL_BLOCK => "LEXICAL_BLOCK",
            MetadataCodes::METADATA_LEXICAL_BLOCK_FILE => "LEXICAL_BLOCK_FILE",
            MetadataCodes::METADATA_NAMESPACE => "NAMESPACE",
            MetadataCodes::METADATA_TEMPLATE_TYPE => "TEMPLATE_TYPE",
            MetadataCodes::METADATA_TEMPLATE_VALUE => "TEMPLATE_VALUE",
            MetadataCodes::METADATA_GLOBAL_VAR => "GLOBAL_VAR",
            MetadataCodes::METADATA_LOCAL_VAR => "LOCAL_VAR",
            MetadataCodes::METADATA_EXPRESSION => "EXPRESSION",
            MetadataCodes::METADATA_OBJC_PROPERTY => "OBJC_PROPERTY",
            MetadataCodes::METADATA_IMPORTED_ENTITY => "IMPORTED_ENTITY",
            MetadataCodes::METADATA_MODULE => "MODULE",
            MetadataCodes::METADATA_MACRO => "MACRO",
            MetadataCodes::METADATA_MACRO_FILE => "MACRO_FILE",
            MetadataCodes::METADATA_STRINGS => "STRINGS",
            MetadataCodes::METADATA_GLOBAL_DECL_ATTACHMENT => "GLOBAL_DECL_ATTACHMENT",
            MetadataCodes::METADATA_GLOBAL_VAR_EXPR => "GLOBAL_VAR_EXPR",
            MetadataCodes::METADATA_INDEX_OFFSET => "INDEX_OFFSET",
            MetadataCodes::METADATA_INDEX => "INDEX",
            MetadataCodes::METADATA_ARG_LIST => "ARG_LIST",
            MetadataCodes::METADATA_LABEL => "LABEL",
            MetadataCodes::METADATA_STRING_TYPE => "STRING_TYPE",
            MetadataCodes::METADATA_COMMON_BLOCK => "COMMON_BLOCK",
            MetadataCodes::METADATA_GENERIC_SUBRANGE => "GENERIC_SUBRANGE",
            MetadataCodes::METADATA_ASSIGN_ID => "ASSIGN_ID",
        },
        Blocks::USELIST_BLOCK_ID => match UseListCodes::try_from(record).unwrap() {
            UseListCodes::USELIST_CODE_DEFAULT => "DEFAULT",
            UseListCodes::USELIST_CODE_BB => "BB",
        },
        Blocks::OPERAND_BUNDLE_TAGS_BLOCK_ID => {
            match OperandBundleTagCode::try_from(record).unwrap() {
                OperandBundleTagCode::OPERAND_BUNDLE_TAG => "OPERAND_BUNDLE_TAG",
            }
        }
        Blocks::STRTAB_BLOCK_ID => match StrtabCodes::try_from(record).unwrap() {
            StrtabCodes::STRTAB_BLOB => "BLOB",
        },
        Blocks::SYMTAB_BLOCK_ID => match SymtabCodes::try_from(record).unwrap() {
            SymtabCodes::SYMTAB_BLOB => "BLOB",
        },
        Blocks::SYNC_SCOPE_NAMES_BLOCK_ID => "UnknownCode1",
    }
}

#[derive(TryFromPrimitive)]
#[repr(u32)]
enum Blocks {
    MODULE_BLOCK_ID = 8,
    PARAMATTR_BLOCK_ID,
    PARAMATTR_GROUP_BLOCK_ID,
    CONSTANTS_BLOCK_ID,
    FUNCTION_BLOCK_ID,
    IDENTIFICATION_BLOCK_ID,
    VALUE_SYMTAB_BLOCK_ID,
    METADATA_BLOCK_ID,
    METADATA_ATTACHMENT_ID,
    TYPE_BLOCK_ID_NEW,
    USELIST_BLOCK_ID,
    MODULE_STRTAB_BLOCK_ID,
    GLOBALVAL_SUMMARY_BLOCK_ID,
    OPERAND_BUNDLE_TAGS_BLOCK_ID,
    METADATA_KIND_BLOCK_ID,
    STRTAB_BLOCK_ID,
    FULL_LTO_GLOBALVAL_SUMMARY_BLOCK_ID,
    SYMTAB_BLOCK_ID,
    SYNC_SCOPE_NAMES_BLOCK_ID,
}

/// Identification block contains a string that describes the producer details,
/// and an epoch that defines the auto-upgrade capability.
#[derive(TryFromPrimitive)]
#[repr(u32)]
enum IdentificationCodes {
    IDENTIFICATION_CODE_STRING = 1, // IDENTIFICATION:      [strchr x N]
    IDENTIFICATION_CODE_EPOCH = 2,  // EPOCH:               [epoch#]
}

/// The epoch that defines the auto-upgrade compatibility for the bitcode.
///
/// LLVM guarantees in a major release that a minor release can read bitcode
/// generated by previous minor releases. We translate this by making the reader
/// accepting only bitcode with the same epoch, except for the X.0 release which
/// also accepts N-1.
#[derive(TryFromPrimitive)]
#[repr(u32)]
enum EpochCode {
    BITCODE_CURRENT_EPOCH = 0,
}

/// MODULE blocks have a number of optional fields and subblocks.
#[derive(TryFromPrimitive)]
#[repr(u32)]
enum ModuleCodes {
    MODULE_CODE_VERSION = 1,     // VERSION:     [version#]
    MODULE_CODE_TRIPLE = 2,      // TRIPLE:      [strchr x N]
    MODULE_CODE_DATALAYOUT = 3,  // DATALAYOUT:  [strchr x N]
    MODULE_CODE_ASM = 4,         // ASM:         [strchr x N]
    MODULE_CODE_SECTIONNAME = 5, // SECTIONNAME: [strchr x N]

    // Deprecated, but still needed to read old bitcode files.
    MODULE_CODE_DEPLIB = 6, // DEPLIB:      [strchr x N]

    // GLOBALVAR: [pointer type, isconst, initid,
    //             linkage, alignment, section, visibility, threadlocal]
    MODULE_CODE_GLOBALVAR = 7,

    // FUNCTION:  [type, callingconv, isproto, linkage, paramattrs, alignment,
    //             section, visibility, gc, unnamed_addr]
    MODULE_CODE_FUNCTION = 8,

    // ALIAS: [alias type, aliasee val#, linkage, visibility]
    MODULE_CODE_ALIAS_OLD = 9,

    MODULE_CODE_GCNAME = 11, // GCNAME: [strchr x N]
    MODULE_CODE_COMDAT = 12, // COMDAT: [selection_kind, name]

    MODULE_CODE_VSTOFFSET = 13, // VSTOFFSET: [offset]

    // ALIAS: [alias value type, addrspace, aliasee val#, linkage, visibility]
    MODULE_CODE_ALIAS = 14,

    MODULE_CODE_METADATA_VALUES_UNUSED = 15,

    // SOURCE_FILENAME: [namechar x N]
    MODULE_CODE_SOURCE_FILENAME = 16,

    // HASH: [5*i32]
    MODULE_CODE_HASH = 17,

    // IFUNC: [ifunc value type, addrspace, resolver val#, linkage, visibility]
    MODULE_CODE_IFUNC = 18,
}

/// PARAMATTR blocks have code for defining a parameter attribute set.
#[derive(TryFromPrimitive)]
#[repr(u32)]
enum AttributeCodes {
    // Deprecated, but still needed to read old bitcode files.
    PARAMATTR_CODE_ENTRY_OLD = 1, // ENTRY: [paramidx0, attr0,
    //         paramidx1, attr1...]
    PARAMATTR_CODE_ENTRY = 2,     // ENTRY: [attrgrp0, attrgrp1, ...]
    PARAMATTR_GRP_CODE_ENTRY = 3, // ENTRY: [grpid, idx, attr0, attr1, ...]
}

/// TYPE blocks have codes for each type primitive they use.
#[derive(TryFromPrimitive)]
#[repr(u32)]
enum TypeCodes {
    TYPE_CODE_NUMENTRY = 1, // NUMENTRY: [numentries]

    // Type Codes
    TYPE_CODE_VOID = 2,    // VOID
    TYPE_CODE_FLOAT = 3,   // FLOAT
    TYPE_CODE_DOUBLE = 4,  // DOUBLE
    TYPE_CODE_LABEL = 5,   // LABEL
    TYPE_CODE_OPAQUE = 6,  // OPAQUE
    TYPE_CODE_INTEGER = 7, // INTEGER: [width]
    TYPE_CODE_POINTER = 8, // POINTER: [pointee type]

    TYPE_CODE_FUNCTION_OLD = 9, // FUNCTION: [vararg, attrid, retty,
    //            paramty x N]
    TYPE_CODE_HALF = 10, // HALF

    TYPE_CODE_ARRAY = 11,  // ARRAY: [num_elements, elements_type]
    TYPE_CODE_VECTOR = 12, // VECTOR: [num_elements, elements_type]

    // These are not with the other floating point types because they're
    // a late addition, and putting them in the right place breaks
    // binary compatibility.
    TYPE_CODE_X86_FP80 = 13,  // X86 LONG DOUBLE
    TYPE_CODE_FP128 = 14,     // LONG DOUBLE (112 bit mantissa)
    TYPE_CODE_PPC_FP128 = 15, // PPC LONG DOUBLE (2 doubles)

    TYPE_CODE_METADATA = 16, // METADATA

    TYPE_CODE_X86_MMX = 17, // X86 MMX

    TYPE_CODE_STRUCT_ANON = 18, // STRUCT_ANON: [ispacked, elements_type x N]
    TYPE_CODE_STRUCT_NAME = 19, // STRUCT_NAME: [strchr x N]
    TYPE_CODE_STRUCT_NAMED = 20, // STRUCT_NAMED: [ispacked, elements_type x N]

    TYPE_CODE_FUNCTION = 21, // FUNCTION: [vararg, retty, paramty x N]

    TYPE_CODE_TOKEN = 22, // TOKEN

    TYPE_CODE_BFLOAT = 23,  // BRAIN FLOATING POINT
    TYPE_CODE_X86_AMX = 24, // X86 AMX

    TYPE_CODE_OPAQUE_POINTER = 25, // OPAQUE_POINTER: [addrspace]

    TYPE_CODE_TARGET_TYPE = 26, // TARGET_TYPE
}

#[derive(TryFromPrimitive)]
#[repr(u32)]
enum OperandBundleTagCode {
    OPERAND_BUNDLE_TAG = 1, // TAG: [strchr x N]
}

#[derive(TryFromPrimitive)]
#[repr(u32)]
enum SyncScopeNameCode {
    SYNC_SCOPE_NAME = 1,
}

// Value symbol table codes.
#[derive(TryFromPrimitive)]
#[repr(u32)]
enum ValueSymtabCodes {
    VST_CODE_ENTRY = 1,   // VST_ENTRY: [valueid, namechar x N]
    VST_CODE_BBENTRY = 2, // VST_BBENTRY: [bbid, namechar x N]
    VST_CODE_FNENTRY = 3, // VST_FNENTRY: [valueid, offset, namechar x N]
    // VST_COMBINED_ENTRY: [valueid, refguid]
    VST_CODE_COMBINED_ENTRY = 5,
}

// The module path symbol table only has one code (MST_CODE_ENTRY).
#[derive(TryFromPrimitive)]
#[repr(u32)]
enum ModulePathSymtabCodes {
    MST_CODE_ENTRY = 1, // MST_ENTRY: [modid, namechar x N]
    MST_CODE_HASH = 2,  // MST_HASH:  [5*i32]
}

// The summary section uses different codes in the per-module
// and combined index cases.
#[derive(TryFromPrimitive)]
#[repr(u32)]
enum GlobalValueSummarySymtabCodes {
    // PERMODULE: [valueid, flags, instcount, numrefs, numrefs x valueid,
    //             n x (valueid)]
    FS_PERMODULE = 1,
    // PERMODULE_PROFILE: [valueid, flags, instcount, numrefs,
    //                     numrefs x valueid,
    //                     n x (valueid, hotness+tailcall)]
    FS_PERMODULE_PROFILE = 2,
    // PERMODULE_GLOBALVAR_INIT_REFS: [valueid, flags, n x valueid]
    FS_PERMODULE_GLOBALVAR_INIT_REFS = 3,
    // COMBINED: [valueid, modid, flags, instcount, numrefs, numrefs x valueid,
    //            n x (valueid)]
    FS_COMBINED = 4,
    // COMBINED_PROFILE: [valueid, modid, flags, instcount, numrefs,
    //                    numrefs x valueid,
    //                    n x (valueid, hotness+tailcall)]
    FS_COMBINED_PROFILE = 5,
    // COMBINED_GLOBALVAR_INIT_REFS: [valueid, modid, flags, n x valueid]
    FS_COMBINED_GLOBALVAR_INIT_REFS = 6,
    // ALIAS: [valueid, flags, valueid]
    FS_ALIAS = 7,
    // COMBINED_ALIAS: [valueid, modid, flags, valueid]
    FS_COMBINED_ALIAS = 8,
    // COMBINED_ORIGINAL_NAME: [original_name_hash]
    FS_COMBINED_ORIGINAL_NAME = 9,
    // VERSION of the summary, bumped when adding flags for instance.
    FS_VERSION = 10,
    // The list of llvm.type.test type identifiers used by the following function
    // that are used other than by an llvm.assume.
    // [n x typeid]
    FS_TYPE_TESTS = 11,
    // The list of virtual calls made by this function using
    // llvm.assume(llvm.type.test) intrinsics that do not have all constant
    // integer arguments.
    // [n x (typeid, offset)]
    FS_TYPE_TEST_ASSUME_VCALLS = 12,
    // The list of virtual calls made by this function using
    // llvm.type.checked.load intrinsics that do not have all constant integer
    // arguments.
    // [n x (typeid, offset)]
    FS_TYPE_CHECKED_LOAD_VCALLS = 13,
    // Identifies a virtual call made by this function using an
    // llvm.assume(llvm.type.test) intrinsic with all constant integer arguments.
    // [typeid, offset, n x arg]
    FS_TYPE_TEST_ASSUME_CONST_VCALL = 14,
    // Identifies a virtual call made by this function using an
    // llvm.type.checked.load intrinsic with all constant integer arguments.
    // [typeid, offset, n x arg]
    FS_TYPE_CHECKED_LOAD_CONST_VCALL = 15,
    // Assigns a GUID to a value ID. This normally appears only in combined
    // summaries, but it can also appear in per-module summaries for PGO data.
    // [valueid, guid]
    FS_VALUE_GUID = 16,
    // The list of local functions with CFI jump tables. Function names are
    // strings in strtab.
    // [n * name]
    FS_CFI_FUNCTION_DEFS = 17,
    // The list of external functions with CFI jump tables. Function names are
    // strings in strtab.
    // [n * name]
    FS_CFI_FUNCTION_DECLS = 18,
    // Per-module summary that also adds relative block frequency to callee info.
    // PERMODULE_RELBF: [valueid, flags, instcount, numrefs,
    //                   numrefs x valueid,
    //                   n x (valueid, relblockfreq+tailcall)]
    FS_PERMODULE_RELBF = 19,
    // Index-wide flags
    FS_FLAGS = 20,
    // Maps type identifier to summary information for that type identifier.
    // Produced by the thin link (only lives in combined index).
    // TYPE_ID: [typeid, kind, bitwidth, align, size, bitmask, inlinebits,
    //           n x (typeid, kind, name, numrba,
    //                numrba x (numarg, numarg x arg, kind, info, byte, bit))]
    FS_TYPE_ID = 21,
    // For background see overview at https://llvm.org/docs/TypeMetadata.html.
    // The type metadata includes both the type identifier and the offset of
    // the address point of the type (the address held by objects of that type
    // which may not be the beginning of the virtual table). Vtable definitions
    // are decorated with type metadata for the types they are compatible with.
    //
    // Maps type identifier to summary information for that type identifier
    // computed from type metadata: the valueid of each vtable definition
    // decorated with a type metadata for that identifier, and the offset from
    // the corresponding type metadata.
    // Exists in the per-module summary to provide information to thin link
    // for index-based whole program devirtualization.
    // TYPE_ID_METADATA: [typeid, n x (valueid, offset)]
    FS_TYPE_ID_METADATA = 22,
    // Summarizes vtable definition for use in index-based whole program
    // devirtualization during the thin link.
    // PERMODULE_VTABLE_GLOBALVAR_INIT_REFS: [valueid, flags, varflags,
    //                                        numrefs, numrefs x valueid,
    //                                        n x (valueid, offset)]
    FS_PERMODULE_VTABLE_GLOBALVAR_INIT_REFS = 23,
    // The total number of basic blocks in the module.
    FS_BLOCK_COUNT = 24,
    // Range information for accessed offsets for every argument.
    // [n x (paramno, range, numcalls, numcalls x (callee_guid, paramno, range))]
    FS_PARAM_ACCESS = 25,
    // Summary of per-module memprof callsite metadata.
    // [valueid, n x stackidindex]
    FS_PERMODULE_CALLSITE_INFO = 26,
    // Summary of per-module allocation memprof metadata.
    // [nummib, nummib x (alloc type, context radix tree index),
    // [nummib x (numcontext x total size)]?]
    FS_PERMODULE_ALLOC_INFO = 27,
    // Summary of combined index memprof callsite metadata.
    // [valueid, context radix tree index, numver,
    //  numver x version]
    FS_COMBINED_CALLSITE_INFO = 28,
    // Summary of combined index allocation memprof metadata.
    // [nummib, numver,
    //  nummib x (alloc type, numstackids, numstackids x stackidindex),
    //  numver x version]
    FS_COMBINED_ALLOC_INFO = 29,
    // List of all stack ids referenced by index in the callsite and alloc infos.
    // [n x stack id]
    FS_STACK_IDS = 30,
    // List of all full stack id pairs corresponding to the total sizes recorded
    // at the end of the alloc info when reporting of hinted bytes is enabled.
    // We use a fixed-width array, which is more efficient as these ids typically
    // are close to 64 bits in size. The max fixed width value supported is 32
    // bits so each 64-bit context id hash is recorded as a pair (upper 32 bits
    // first). This record must immediately precede the associated alloc info, and
    // the entries must be in the exact same order as the corresponding sizes.
    // [nummib x (numcontext x full stack id)]
    FS_ALLOC_CONTEXT_IDS = 31,
    // Linearized radix tree of allocation contexts. See the description above the
    // CallStackRadixTreeBuilder class in ProfileData/MemProf.h for format.
    // [n x entry]
    FS_CONTEXT_RADIX_TREE_ARRAY = 32,
}

#[derive(TryFromPrimitive)]
#[repr(u32)]
enum MetadataCodes {
    METADATA_STRING_OLD = 1,              // MDSTRING:      [values]
    METADATA_VALUE = 2,                   // VALUE:         [type num, value num]
    METADATA_NODE = 3,                    // NODE:          [n x md num]
    METADATA_NAME = 4,                    // STRING:        [values]
    METADATA_DISTINCT_NODE = 5,           // DISTINCT_NODE: [n x md num]
    METADATA_KIND = 6,                    // [n x [id, name]]
    METADATA_LOCATION = 7,                // [distinct, line, col, scope, inlined-at?]
    METADATA_OLD_NODE = 8,                // OLD_NODE:      [n x (type num, value num)]
    METADATA_OLD_FN_NODE = 9,             // OLD_FN_NODE:   [n x (type num, value num)]
    METADATA_NAMED_NODE = 10,             // NAMED_NODE:    [n x mdnodes]
    METADATA_ATTACHMENT = 11,             // [m x [value, [n x [id, mdnode]]]
    METADATA_GENERIC_DEBUG = 12,          // [distinct, tag, vers, header, n x md num]
    METADATA_SUBRANGE = 13,               // [distinct, count, lo]
    METADATA_ENUMERATOR = 14,             // [isUnsigned|distinct, value, name]
    METADATA_BASIC_TYPE = 15,             // [distinct, tag, name, size, align, enc]
    METADATA_FILE = 16,                   // [distinct, filename, directory, checksumkind, checksum]
    METADATA_DERIVED_TYPE = 17,           // [distinct, ...]
    METADATA_COMPOSITE_TYPE = 18,         // [distinct, ...]
    METADATA_SUBROUTINE_TYPE = 19,        // [distinct, flags, types, cc]
    METADATA_COMPILE_UNIT = 20,           // [distinct, ...]
    METADATA_SUBPROGRAM = 21,             // [distinct, ...]
    METADATA_LEXICAL_BLOCK = 22,          // [distinct, scope, file, line, column]
    METADATA_LEXICAL_BLOCK_FILE = 23,     //[distinct, scope, file, discriminator]
    METADATA_NAMESPACE = 24,              // [distinct, scope, file, name, line, exportSymbols]
    METADATA_TEMPLATE_TYPE = 25,          // [distinct, scope, name, type, ...]
    METADATA_TEMPLATE_VALUE = 26,         // [distinct, scope, name, type, value, ...]
    METADATA_GLOBAL_VAR = 27,             // [distinct, ...]
    METADATA_LOCAL_VAR = 28,              // [distinct, ...]
    METADATA_EXPRESSION = 29,             // [distinct, n x element]
    METADATA_OBJC_PROPERTY = 30,          // [distinct, name, file, line, ...]
    METADATA_IMPORTED_ENTITY = 31,        // [distinct, tag, scope, entity, line, name]
    METADATA_MODULE = 32,                 // [distinct, scope, name, ...]
    METADATA_MACRO = 33,                  // [distinct, macinfo, line, name, value]
    METADATA_MACRO_FILE = 34,             // [distinct, macinfo, line, file, ...]
    METADATA_STRINGS = 35,                // [count, offset] blob([lengths][chars])
    METADATA_GLOBAL_DECL_ATTACHMENT = 36, // [valueid, n x [id, mdnode]]
    METADATA_GLOBAL_VAR_EXPR = 37,        // [distinct, var, expr]
    METADATA_INDEX_OFFSET = 38,           // [offset]
    METADATA_INDEX = 39,                  // [bitpos]
    METADATA_LABEL = 40,                  // [distinct, scope, name, file, line]
    METADATA_STRING_TYPE = 41,            // [distinct, name, size, align,...]
    // Codes 42 and 43 are reserved for support for Fortran array specific debug
    // info.
    METADATA_COMMON_BLOCK = 44, // [distinct, scope, name, variable,...]
    METADATA_GENERIC_SUBRANGE = 45, // [distinct, count, lo, up, stride]
    METADATA_ARG_LIST = 46,     // [n x [type num, value num]]
    METADATA_ASSIGN_ID = 47,    // [distinct, ...]
}

// The constants block (CONSTANTS_BLOCK_ID) describes emission for each
// constant and maintains an implicit current type value.
#[derive(TryFromPrimitive)]
#[repr(u32)]
enum ConstantsCodes {
    CST_CODE_SETTYPE = 1,        // SETTYPE:       [typeid]
    CST_CODE_NULL = 2,           // NULL
    CST_CODE_UNDEF = 3,          // UNDEF
    CST_CODE_INTEGER = 4,        // INTEGER:       [intval]
    CST_CODE_WIDE_INTEGER = 5,   // WIDE_INTEGER:  [n x intval]
    CST_CODE_FLOAT = 6,          // FLOAT:         [fpval]
    CST_CODE_AGGREGATE = 7,      // AGGREGATE:     [n x value number]
    CST_CODE_STRING = 8,         // STRING:        [values]
    CST_CODE_CSTRING = 9,        // CSTRING:       [values]
    CST_CODE_CE_BINOP = 10,      // CE_BINOP:      [opcode, opval, opval]
    CST_CODE_CE_CAST = 11,       // CE_CAST:       [opcode, opty, opval]
    CST_CODE_CE_GEP_OLD = 12,    // CE_GEP:        [n x operands]
    CST_CODE_CE_SELECT = 13,     // CE_SELECT:     [opval, opval, opval]
    CST_CODE_CE_EXTRACTELT = 14, // CE_EXTRACTELT: [opty, opval, opval]
    CST_CODE_CE_INSERTELT = 15,  // CE_INSERTELT:  [opval, opval, opval]
    CST_CODE_CE_SHUFFLEVEC = 16, // CE_SHUFFLEVEC: [opval, opval, opval]
    CST_CODE_CE_CMP = 17,        // CE_CMP:        [opty, opval, opval, pred]
    CST_CODE_INLINEASM_OLD = 18, // INLINEASM:     [sideeffect|alignstack,
    //                 asmstr,conststr]
    CST_CODE_CE_SHUFVEC_EX = 19, // SHUFVEC_EX:    [opty, opval, opval, opval]
    CST_CODE_CE_INBOUNDS_GEP = 20, // INBOUNDS_GEP:  [n x operands]
    CST_CODE_BLOCKADDRESS = 21,  // CST_CODE_BLOCKADDRESS [fnty, fnval, bb#]
    CST_CODE_DATA = 22,          // DATA:          [n x elements]
    CST_CODE_INLINEASM_OLD2 = 23, // INLINEASM:     [sideeffect|alignstack|
    //                 asmdialect,asmstr,conststr]
    CST_CODE_CE_GEP_WITH_INRANGE_INDEX_OLD = 24, //  [opty, flags, n x operands]
    CST_CODE_CE_UNOP = 25,                       // CE_UNOP:      [opcode, opval]
    CST_CODE_POISON = 26,                        // POISON
    CST_CODE_DSO_LOCAL_EQUIVALENT = 27,          // DSO_LOCAL_EQUIVALENT [gvty, gv]
    CST_CODE_INLINEASM_OLD3 = 28,                // INLINEASM:     [sideeffect|alignstack|
    //                 asmdialect|unwind,
    //                 asmstr,conststr]
    CST_CODE_NO_CFI_VALUE = 29, // NO_CFI [ fty, f ]
    CST_CODE_INLINEASM = 30,    // INLINEASM:     [fnty,
    //                 sideeffect|alignstack|
    //                 asmdialect|unwind,
    //                 asmstr,conststr]
    CST_CODE_CE_GEP_WITH_INRANGE = 31, // [opty, flags, range, n x operands]
    CST_CODE_CE_GEP = 32,              // [opty, flags, n x operands]
    CST_CODE_PTRAUTH = 33,             // [ptr, key, disc, addrdisc]
}

// The function body block (FUNCTION_BLOCK_ID) describes function bodies.  It
// can contain a constant block (CONSTANTS_BLOCK_ID).
#[derive(TryFromPrimitive)]
#[repr(u32)]
enum FunctionCodes {
    FUNC_CODE_DECLAREBLOCKS = 1, // DECLAREBLOCKS: [n]

    FUNC_CODE_INST_BINOP = 2,      // BINOP:      [opcode, ty, opval, opval]
    FUNC_CODE_INST_CAST = 3,       // CAST:       [opcode, ty, opty, opval]
    FUNC_CODE_INST_GEP_OLD = 4,    // GEP:        [n x operands]
    FUNC_CODE_INST_SELECT = 5,     // SELECT:     [ty, opval, opval, opval]
    FUNC_CODE_INST_EXTRACTELT = 6, // EXTRACTELT: [opty, opval, opval]
    FUNC_CODE_INST_INSERTELT = 7,  // INSERTELT:  [ty, opval, opval, opval]
    FUNC_CODE_INST_SHUFFLEVEC = 8, // SHUFFLEVEC: [ty, opval, opval, opval]
    FUNC_CODE_INST_CMP = 9,        // CMP:        [opty, opval, opval, pred]

    FUNC_CODE_INST_RET = 10,    // RET:        [opty,opval<both optional>]
    FUNC_CODE_INST_BR = 11,     // BR:         [bb#, bb#, cond] or [bb#]
    FUNC_CODE_INST_SWITCH = 12, // SWITCH:     [opty, op0, op1, ...]
    FUNC_CODE_INST_INVOKE = 13, // INVOKE:     [attr, fnty, op0,op1, ...]
    // 14 is unused.
    FUNC_CODE_INST_UNREACHABLE = 15, // UNREACHABLE

    FUNC_CODE_INST_PHI = 16, // PHI:        [ty, val0,bb0, ...]
    // 17 is unused.
    // 18 is unused.
    FUNC_CODE_INST_ALLOCA = 19, // ALLOCA:     [instty, opty, op, align]
    FUNC_CODE_INST_LOAD = 20,   // LOAD:       [opty, op, align, vol]
    // 21 is unused.
    // 22 is unused.
    FUNC_CODE_INST_VAARG = 23, // VAARG:      [valistty, valist, instty]
    // This store code encodes the pointer type, rather than the value type
    // this is so information only available in the pointer type (e.g. address
    // spaces) is retained.
    FUNC_CODE_INST_STORE_OLD = 24, // STORE:      [ptrty,ptr,val, align, vol]
    // 25 is unused.
    FUNC_CODE_INST_EXTRACTVAL = 26, // EXTRACTVAL: [n x operands]
    FUNC_CODE_INST_INSERTVAL = 27,  // INSERTVAL:  [n x operands]
    // fcmp/icmp returning Int1TY or vector of Int1Ty. Same as CMP, exists to
    // support legacy vicmp/vfcmp instructions.
    FUNC_CODE_INST_CMP2 = 28, // CMP2:       [opty, opval, opval, pred]
    // new select on i1 or [N x i1]
    FUNC_CODE_INST_VSELECT = 29, // VSELECT:    [ty,opval,opval,predty,pred]
    FUNC_CODE_INST_INBOUNDS_GEP_OLD = 30, // INBOUNDS_GEP: [n x operands]
    FUNC_CODE_INST_INDIRECTBR = 31, // INDIRECTBR: [opty, op0, op1, ...]
    // 32 is unused.
    FUNC_CODE_DEBUG_LOC_AGAIN = 33, // DEBUG_LOC_AGAIN

    FUNC_CODE_INST_CALL = 34, // CALL:    [attr, cc, fnty, fnid, args...]

    FUNC_CODE_DEBUG_LOC = 35,  // DEBUG_LOC:  [Line,Col,ScopeVal, IAVal]
    FUNC_CODE_INST_FENCE = 36, // FENCE: [ordering, synchscope]
    FUNC_CODE_INST_CMPXCHG_OLD = 37, // CMPXCHG: [ptrty, ptr, cmp, val, vol,
    //            ordering, synchscope,
    //            failure_ordering?, weak?]
    FUNC_CODE_INST_ATOMICRMW_OLD = 38, // ATOMICRMW: [ptrty,ptr,val, operation,
    //             align, vol,
    //             ordering, synchscope]
    FUNC_CODE_INST_RESUME = 39,         // RESUME:     [opval]
    FUNC_CODE_INST_LANDINGPAD_OLD = 40, // LANDINGPAD: [ty,val,val,num,id0,val0...]
    FUNC_CODE_INST_LOADATOMIC = 41,     // LOAD: [opty, op, align, vol,
    //        ordering, synchscope]
    FUNC_CODE_INST_STOREATOMIC_OLD = 42, // STORE: [ptrty,ptr,val, align, vol
    //         ordering, synchscope]
    FUNC_CODE_INST_GEP = 43,         // GEP:  [inbounds, n x operands]
    FUNC_CODE_INST_STORE = 44,       // STORE: [ptrty,ptr,valty,val, align, vol]
    FUNC_CODE_INST_STOREATOMIC = 45, // STORE: [ptrty,ptr,val, align, vol
    FUNC_CODE_INST_CMPXCHG = 46,     // CMPXCHG: [ptrty, ptr, cmp, val, vol,
    //           success_ordering, synchscope,
    //           failure_ordering, weak]
    FUNC_CODE_INST_LANDINGPAD = 47, // LANDINGPAD: [ty,val,num,id0,val0...]
    FUNC_CODE_INST_CLEANUPRET = 48, // CLEANUPRET: [val] or [val,bb#]
    FUNC_CODE_INST_CATCHRET = 49,   // CATCHRET: [val,bb#]
    FUNC_CODE_INST_CATCHPAD = 50,   // CATCHPAD: [bb#,bb#,num,args...]
    FUNC_CODE_INST_CLEANUPPAD = 51, // CLEANUPPAD: [num,args...]
    FUNC_CODE_INST_CATCHSWITCH = 52, // CATCHSWITCH: [num,args...] or [num,args...,bb]
    // 53 is unused.
    // 54 is unused.
    FUNC_CODE_OPERAND_BUNDLE = 55, // OPERAND_BUNDLE: [tag#, value...]
    FUNC_CODE_INST_UNOP = 56,      // UNOP:       [opcode, ty, opval]
    FUNC_CODE_INST_CALLBR = 57,    // CALLBR:     [attr, cc, norm, transfs,
    //              fnty, fnid, args...]
    FUNC_CODE_INST_FREEZE = 58,    // FREEZE: [opty, opval]
    FUNC_CODE_INST_ATOMICRMW = 59, // ATOMICRMW: [ptrty, ptr, valty, val,
    //             operation, align, vol,
    //             ordering, synchscope]
    FUNC_CODE_BLOCKADDR_USERS = 60, // BLOCKADDR_USERS: [value...]

    FUNC_CODE_DEBUG_RECORD_VALUE = 61, // [DILocation, DILocalVariable, DIExpression, ValueAsMetadata]
    FUNC_CODE_DEBUG_RECORD_DECLARE = 62, // [DILocation, DILocalVariable, DIExpression, ValueAsMetadata]
    FUNC_CODE_DEBUG_RECORD_ASSIGN = 63, // [DILocation, DILocalVariable, DIExpression, ValueAsMetadata,
    //  DIAssignID, DIExpression (addr), ValueAsMetadata (addr)]
    FUNC_CODE_DEBUG_RECORD_VALUE_SIMPLE = 64, // [DILocation, DILocalVariable, DIExpression, Value]
    FUNC_CODE_DEBUG_RECORD_LABEL = 65,        // [DILocation, DILabel]
}

#[derive(TryFromPrimitive)]
#[repr(u32)]
enum UseListCodes {
    USELIST_CODE_DEFAULT = 1, // DEFAULT: [index..., value-id]
    USELIST_CODE_BB = 2,      // BB: [index..., bb-id]
}

#[derive(TryFromPrimitive)]
#[repr(u32)]
enum AttributeKindCodes {
    // = 0 is unused
    ATTR_KIND_ALIGNMENT = 1,
    ATTR_KIND_ALWAYS_INLINE = 2,
    ATTR_KIND_BY_VAL = 3,
    ATTR_KIND_INLINE_HINT = 4,
    ATTR_KIND_IN_REG = 5,
    ATTR_KIND_MIN_SIZE = 6,
    ATTR_KIND_NAKED = 7,
    ATTR_KIND_NEST = 8,
    ATTR_KIND_NO_ALIAS = 9,
    ATTR_KIND_NO_BUILTIN = 10,
    ATTR_KIND_NO_CAPTURE = 11,
    ATTR_KIND_NO_DUPLICATE = 12,
    ATTR_KIND_NO_IMPLICIT_FLOAT = 13,
    ATTR_KIND_NO_INLINE = 14,
    ATTR_KIND_NON_LAZY_BIND = 15,
    ATTR_KIND_NO_RED_ZONE = 16,
    ATTR_KIND_NO_RETURN = 17,
    ATTR_KIND_NO_UNWIND = 18,
    ATTR_KIND_OPTIMIZE_FOR_SIZE = 19,
    ATTR_KIND_READ_NONE = 20,
    ATTR_KIND_READ_ONLY = 21,
    ATTR_KIND_RETURNED = 22,
    ATTR_KIND_RETURNS_TWICE = 23,
    ATTR_KIND_S_EXT = 24,
    ATTR_KIND_STACK_ALIGNMENT = 25,
    ATTR_KIND_STACK_PROTECT = 26,
    ATTR_KIND_STACK_PROTECT_REQ = 27,
    ATTR_KIND_STACK_PROTECT_STRONG = 28,
    ATTR_KIND_STRUCT_RET = 29,
    ATTR_KIND_SANITIZE_ADDRESS = 30,
    ATTR_KIND_SANITIZE_THREAD = 31,
    ATTR_KIND_SANITIZE_MEMORY = 32,
    ATTR_KIND_UW_TABLE = 33,
    ATTR_KIND_Z_EXT = 34,
    ATTR_KIND_BUILTIN = 35,
    ATTR_KIND_COLD = 36,
    ATTR_KIND_OPTIMIZE_NONE = 37,
    ATTR_KIND_IN_ALLOCA = 38,
    ATTR_KIND_NON_NULL = 39,
    ATTR_KIND_JUMP_TABLE = 40,
    ATTR_KIND_DEREFERENCEABLE = 41,
    ATTR_KIND_DEREFERENCEABLE_OR_NULL = 42,
    ATTR_KIND_CONVERGENT = 43,
    ATTR_KIND_SAFESTACK = 44,
    ATTR_KIND_ARGMEMONLY = 45,
    ATTR_KIND_SWIFT_SELF = 46,
    ATTR_KIND_SWIFT_ERROR = 47,
    ATTR_KIND_NO_RECURSE = 48,
    ATTR_KIND_INACCESSIBLEMEM_ONLY = 49,
    ATTR_KIND_INACCESSIBLEMEM_OR_ARGMEMONLY = 50,
    ATTR_KIND_ALLOC_SIZE = 51,
    ATTR_KIND_WRITEONLY = 52,
    ATTR_KIND_SPECULATABLE = 53,
    ATTR_KIND_STRICT_FP = 54,
    ATTR_KIND_SANITIZE_HWADDRESS = 55,
    ATTR_KIND_NOCF_CHECK = 56,
    ATTR_KIND_OPT_FOR_FUZZING = 57,
    ATTR_KIND_SHADOWCALLSTACK = 58,
    ATTR_KIND_SPECULATIVE_LOAD_HARDENING = 59,
    ATTR_KIND_IMMARG = 60,
    ATTR_KIND_WILLRETURN = 61,
    ATTR_KIND_NOFREE = 62,
    ATTR_KIND_NOSYNC = 63,
    ATTR_KIND_SANITIZE_MEMTAG = 64,
    ATTR_KIND_PREALLOCATED = 65,
    ATTR_KIND_NO_MERGE = 66,
    ATTR_KIND_NULL_POINTER_IS_VALID = 67,
    ATTR_KIND_NOUNDEF = 68,
    ATTR_KIND_BYREF = 69,
    ATTR_KIND_MUSTPROGRESS = 70,
    ATTR_KIND_NO_CALLBACK = 71,
    ATTR_KIND_HOT = 72,
    ATTR_KIND_NO_PROFILE = 73,
    ATTR_KIND_VSCALE_RANGE = 74,
    ATTR_KIND_SWIFT_ASYNC = 75,
    ATTR_KIND_NO_SANITIZE_COVERAGE = 76,
    ATTR_KIND_ELEMENTTYPE = 77,
    ATTR_KIND_DISABLE_SANITIZER_INSTRUMENTATION = 78,
    ATTR_KIND_NO_SANITIZE_BOUNDS = 79,
    ATTR_KIND_ALLOC_ALIGN = 80,
    ATTR_KIND_ALLOCATED_POINTER = 81,
    ATTR_KIND_ALLOC_KIND = 82,
    ATTR_KIND_PRESPLIT_COROUTINE = 83,
    ATTR_KIND_FNRETTHUNK_EXTERN = 84,
    ATTR_KIND_SKIP_PROFILE = 85,
    ATTR_KIND_MEMORY = 86,
    ATTR_KIND_NOFPCLASS = 87,
    ATTR_KIND_OPTIMIZE_FOR_DEBUGGING = 88,
    ATTR_KIND_WRITABLE = 89,
    ATTR_KIND_CORO_ONLY_DESTROY_WHEN_COMPLETE = 90,
    ATTR_KIND_DEAD_ON_UNWIND = 91,
    ATTR_KIND_RANGE = 92,
    ATTR_KIND_SANITIZE_NUMERICAL_STABILITY = 93,
    ATTR_KIND_INITIALIZES = 94,
    ATTR_KIND_HYBRID_PATCHABLE = 95,
    ATTR_KIND_SANITIZE_REALTIME = 96,
    ATTR_KIND_SANITIZE_REALTIME_BLOCKING = 97,
    ATTR_KIND_CORO_ELIDE_SAFE = 98,
    ATTR_KIND_NO_EXT = 99,
    ATTR_KIND_NO_DIVERGENCE_SOURCE = 100,
    ATTR_KIND_SANITIZE_TYPE = 101,
    ATTR_KIND_CAPTURES = 102,
}

#[derive(TryFromPrimitive)]
#[repr(u32)]
enum ComdatSelectionKindCodes {
    COMDAT_SELECTION_KIND_ANY = 1,
    COMDAT_SELECTION_KIND_EXACT_MATCH = 2,
    COMDAT_SELECTION_KIND_LARGEST = 3,
    COMDAT_SELECTION_KIND_NO_DUPLICATES = 4,
    COMDAT_SELECTION_KIND_SAME_SIZE = 5,
}

#[derive(TryFromPrimitive)]
#[repr(u32)]
enum StrtabCodes {
    STRTAB_BLOB = 1,
}

#[derive(TryFromPrimitive)]
#[repr(u32)]
enum SymtabCodes {
    SYMTAB_BLOB = 1,
}
