/// Value ID may be global or function-local
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct ValueId(pub u32);

/// Basic block ID in the function
///
/// BitcodeWriter uses getValueID to serialize these, but they're plain BB indices, not ValueList IDs.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct BbId(pub u32);

/// Global type ID
pub type TypeId = u32;

/// Attribute
pub type ParamAttrGroupId = u32;

/// Confusingly, LLVM's `BitcodeWriter` uses an `InstID` variable that is **not** the same as instruction ID, and does not uniquely identify instructions. LLVM's `InstID` tracks number of non-`Void` `Value`s.
///
/// This type is the equivalent of LLVM's `InstructionCount`, that is incremented for every instruction (`Void` ones too). It does not track debug records that may be in the same bitcode block as instruction records.
pub type InstIndex = usize;

use crate::schema::values::*;
use num_enum::TryFromPrimitive;
use std::fmt;
use std::num::NonZero;
use std::ops::Range;
use std::sync::Arc;

// MODULE_BLOCK (id=8) - Detailed records

/// MODULE_CODE_VERSION (code=1)
#[derive(Debug)]
pub struct ModuleVersionRecord {
    /// The module version number.
    pub version: u64,
}

/// MODULE_CODE_TRIPLE (code=2)
#[derive(Debug)]
pub struct ModuleTripleRecord {
    /// The bytes of the target triple string.
    pub triple: String,
}

/// `MODULE_CODE_DATALAYOUT` (code=3)
#[derive(Debug)]
pub struct ModuleDataLayoutRecord {
    /// The bytes of the data layout specification string.
    pub datalayout: String,
}

/// `MODULE_CODE_ASM` (code=4)
#[derive(Debug)]
pub struct ModuleAsmRecord {
    /// The bytes of module-level assembly.
    pub asm: String,
}

/// `MODULE_CODE_HASH`
#[derive(Debug)]
pub struct ModuleHashRecord(pub [u32; 5]);

/// A section name is emitted for each unique section in the module.
///
/// Record ID: `MODULE_CODE_SECTIONNAME` = 5
#[derive(Debug)]
pub struct ModuleSectionNameRecord {
    /// The bytes of one section name.
    pub section_name: String,
}

/// `MODULE_CODE_DEPLIB` (code=6)
#[derive(Debug)]
pub struct ModuleDepLibRecord {
    /// The bytes of one dependent library name.
    pub deplib_name: String,
}

/// `MODULE_CODE_GLOBALVAR` (code=7)
///
/// Global variable record.
///
/// The record layout is:
/// [strtab offset, strtab size, type, flags, initid, linkage, alignment,
///  section, visibility, threadlocal, unnamed_addr, externally_initialized,
///  dllstorageclass, comdat, attributes, DSO_Local, GlobalSanitizer, code_model]
#[derive(Debug)]
pub struct ModuleGlobalVarRecord {
    pub name: Range<usize>,
    /// The type ID of the global variable's type.
    pub type_id: u32,
    /// A packed flags field: (address_space << 2) | (explicitType flag << 1) | (is_constant flag).
    /// Packed flags: (address_space << 2) | (explicit_type << 1) | is_constant.
    pub flags: u32,
    /// If the variable is defined (not a declaration), then (initializer value ID + 1),
    /// or 0 otherwise.
    ///
    // if (unsigned InitID = Record[2])
    // GlobalInits.push_back(std::make_pair(NewGV, InitID - 1));
    pub init_id: Option<NonZero<u64>>,

    /// Encoded linkage.
    pub linkage: Linkage,

    /// log2(alignment) + 1
    pub alignment: Option<NonZero<u32>>,

    /// 1-based index into SECTIONNAME records
    pub section: Option<NonZero<u32>>,

    /// e.g. 0=default, 1=hidden, 2=protected
    pub visibility: u8,

    /// not thread local: code 0 ,    thread local; default TLS model: code 1 , localdynamic: code 2 , initialexec: code 3 , localexec: code 4 ,
    pub thread_local: u8,

    /// 0=none, 1=unnamed_addr, 2=local_unnamed_addr
    pub unnamed_addr: Option<NonZero<u8>>,

    // /// For externally_initialized (boolean), we store it separately:
    // pub externally_initialized: bool,
    /// e.g. 0=default, 1=dllimport, 2=dllexport
    pub dll_storage_class: DllStorageClass,

    /// An encoding of the COMDAT of this function
    pub comdat: Option<NonZero<u64>>,

    /// 1-based index into the table of AttributeLists, if present
    pub attributes: Option<NonZero<u32>>,

    /// e.g. 0=dso_preemptable, 1=dso_local
    pub dso_local: bool,

    /// Partition: stored as (strtab offset, length) if nonempty.
    pub partition: Range<usize>,

    /// `llvm::GlobalValue::SanitizerMetadata`
    pub global_sanitizer: Option<NonZero<u32>>,

    pub code_model: u32,
}

/// Per–module global variable initializer reference record.
/// (FS_PERMODULE_GLOBALVAR_INIT_REFS = 3)
#[derive(Debug)]
pub struct PerModuleGlobalVarInitRefsRecord {
    pub value_id: ValueId,
    pub flags: u64,
    /// A vector of referenced value IDs.
    pub init_refs: Vec<ValueId>,
}

/// `MODULE_CODE_FUNCTION` (code=8)
#[derive(Debug)]
pub struct ModuleFunctionRecord {
    pub symbol_strtab_range: Range<usize>,
    /// The type index of the function type describing this function
    pub ty: TypeId,
    pub calling_conv: CallConv,
    /// Non-zero if this entry represents a declaration rather than a definition
    pub is_proto: bool,
    /// An encoding of the linkage type for this function
    pub linkage: Linkage,

    /// In the bitcode it's stored as 1-based index into `PARAMATTR_BLOCK` if present,
    /// here it's just the index
    pub attributes_index: Option<u32>,

    /// log2(alignment) + 1
    pub alignment: Option<NonZero<u32>>,

    /// 1-based index into SECTIONNAME records
    pub section: Option<NonZero<u32>>,

    ///   default: code 0  hidden: code 1  protected: code 2
    pub visibility: u8,

    /// 1-based index into GCNAME records if present
    pub gc: Option<NonZero<u64>>,

    /// not unnamed_addr: 0,  unnamed_addr: 1,  local_unnamed_addr: 2,
    pub unnamed_addr: Option<NonZero<u8>>,

    /// If non-zero, the value index of the prologue data + 1
    pub prologue_data: Option<NonZero<u64>>,

    /// If present, an encoding of the DLL storage class of this variable:  default: code 0  dllimport: code 1  dllexport: code 2
    pub dll_storage_class: DllStorageClass,

    /// An encoding of the COMDAT of this function
    pub comdat: Option<NonZero<u64>>,

    /// If non-zero, the value index + 1
    pub prefix_data: Option<NonZero<u64>>,

    /// f non-zero, the value index of the personality function for this function, plus 1.
    pub personality_fn: Option<NonZero<u64>>,

    /// e.g. 0=dso_preemptable, 1=dso_local
    pub dso_local: bool,

    /// Unused
    pub address_space: u64,

    pub partition_name: Range<usize>,
}

/// MODULE_CODE_ALIAS (code=9)
#[derive(Debug)]
pub struct ModuleAliasRecord {
    pub name: Range<usize>,
    pub alias_type: TypeId,
    pub aliasee_val: ValueId,
    pub linkage: u64,

    pub visibility: u8,
    pub dll_storage_class: DllStorageClass,
    pub threadlocal: u8,
    pub unnamed_addr: Option<NonZero<u8>>,
    pub preemption_specifier: u64,
}

/// MODULE_CODE_GCNAME (code=11)
#[derive(Debug)]
pub struct ModuleGCNameRecord {
    pub gc_name: String,
}

// PARAMATTR_CODE_ENTRY_OLD (code=1)
// A legacy format with pairs of (paramidx, attrbits)
#[derive(Debug)]
#[deprecated]
pub struct ParamAttrPair {
    /// 0=return-value, 0xFFFFFFFF=function attributes, else 1-based parameter.
    pub param_idx: u64,
    /// A bitmap of attribute bits (zeroext, signext, noreturn, etc.).
    pub attr_bits: u64,
}

/// ---------------------------------------------------------------------------
/// TYPE_BLOCK (id=17)
/// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct TypeNumEntryRecord {
    pub numentries: u64,
}

// /// TYPE_CODE_POINTER (code=8)
// #[derive(Debug)]
// pub struct TypePointerRecord {
//     pub pointee_type: u32,
//     /// If not present in the stream, it's `0` by default.
//     pub address_space: u64,
// }

/// TYPE_CODE_OPAQUE_POINTER (code=25)
#[derive(Debug)]
pub struct TypeOpaquePointerRecord {
    pub address_space: u64,
}

/// TYPE_CODE_ARRAY (code=11)
#[derive(Debug)]
pub struct TypeArrayRecord {
    pub num_elements: u64,
    pub elements_type: u32,
}

/// TYPE_CODE_VECTOR (code=12)
#[derive(Debug)]
pub struct TypeVectorRecord {
    pub num_elements: u64,
    pub elements_type: u32,
}

/// TYPE_CODE_STRUCT_ANON (code=18)
/// TYPE_CODE_STRUCT_NAMED (code=20)
#[derive(Debug)]
pub struct TypeStructRecord {
    /// Anon if None
    pub name: Option<String>,
    pub is_packed: bool,
    pub element_types: Vec<u32>,
}

/// TYPE_CODE_FUNCTION (code=21)
#[derive(Debug)]
pub struct TypeFunctionRecord {
    pub vararg: bool,
    pub ret_ty: Option<u32>,
    // Ignore param 0?
    pub param_types: Vec<u32>,
}

/// TYPE_CODE_TARGET_TYPE (code=26)
#[derive(Debug)]
pub struct TypeTargetTypeRecord {
    pub ty_params: Vec<u32>,
    pub int_params: Vec<u64>,
}

#[derive(Debug)]
pub struct FunctionBlockRecord {
    // Contains instruction records, references to local constants, sub-blocks
    // for metadata attachments, etc.
}

// Global metadata attachment record. (METADATA_GLOBAL_DECL_ATTACHMENT = 36)
// Attaches metadata to global declarations. Not a !N node.
// [valueid, n x [id, mdnode]]

/// Per–instruction metadata attachment record. (METADATA_ATTACHMENT = 11)
/// [m x [value, [n x [id, mdnode]]])
#[derive(Debug)]
pub struct MetadataAttachment {
    pub value_id: ValueId,
    pub attachments: Vec<(u64, u64)>,
}

/// A metadata kind record. (METADATA_KIND = 6
/// [n x [id, name]]
#[derive(Debug)]
pub struct MetadataKindRecord {
    pub kind_id: u64,
    pub name: String,
}

/// METADATA_STRINGS = 35
///
/// All the metadata strings in a metadata block are emitted in a single
/// record.  The sizes and strings themselves are shoved into a blob.
///
/// [count, offset] blob([lengths][chars])
pub struct MetadataStringsRecord {
    pub ranges: Vec<Range<usize>>,
    pub strings: Vec<u8>,
}

impl MetadataStringsRecord {
    pub fn demangled(&self) -> impl Iterator<Item = (&str, String)> {
        self.strings().map(|s| {
            (
                s,
                try_demangle(s).map(|d| d.to_string()).unwrap_or_default(),
            )
        })
    }

    pub fn strings(&self) -> impl Iterator<Item = &str> {
        self.ranges.iter().map(|r| {
            let s = self.strings.get(r.clone()).unwrap_or_default();
            std::str::from_utf8(s).unwrap_or_default()
        })
    }
}

impl fmt::Debug for MetadataStringsRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.demangled()).finish()
    }
}

// /// Metadata index offset record. (METADATA_INDEX_OFFSET = 38, [offset])
//
// only an optimization
// #[derive(Debug)]
// pub struct MetadataIndexOffsetRecord {
//     pub offset: u32,
//     pub chunk_size: u32,
// }

/// Metadata index record. (METADATA_INDEX = 39)
// only an optimization
// #[derive(Debug)]
// pub struct MetadataIndexRecord {
//     pub deltas: Vec<u64>,
// }

// STRTAB_BLOCK (id=23)

/// `STRTAB_BLOB` (record code=1): A single blob operand containing the file’s
/// string table. Strings are *not* null-terminated and are referenced by
/// (offset, size) pairs in other records.
pub struct StrtabBlobRecord<'a> {
    pub blob: &'a [u8],
}

impl fmt::Debug for StrtabBlobRecord<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Strtab")
            .field("blob", &String::from_utf8_lossy(self.blob))
            .finish()
    }
}

#[derive(Debug)]
pub struct SymtabBlobRecord<'a> {
    pub blob: &'a [u8],
}

#[derive(Debug, Default)]
pub struct Module {
    pub version: Option<ModuleVersionRecord>,
    pub triple: Option<ModuleTripleRecord>,
    pub data_layout: Option<ModuleDataLayoutRecord>,
    pub asm: Option<ModuleAsmRecord>,
    pub section_name: Option<ModuleSectionNameRecord>,
    pub source_filename: Option<String>,
    pub dep_lib: Option<ModuleDepLibRecord>,
    pub global_var: Vec<ModuleGlobalVarRecord>,
    pub function: Option<ModuleFunctionRecord>,
    pub alias: Option<ModuleAliasRecord>,
    pub gc_name: Option<ModuleGCNameRecord>,
    pub comdats: Vec<(Range<usize>, u64)>,
    pub vst_offset: Option<u64>,
    pub hash: Option<ModuleHashRecord>,
}

// #[derive(Debug)]
// pub enum TypedRecord<'a> {
//     Generic(u64, Vec<u64>),
//     // IDENTIFICATION_BLOCK
//     /// Contains a human–readable string identifying the bitcode producer,
//     /// for example: "LLVM3.9.0".
//     IdentificationString(String),
//     IdentificationEpoch(u64),

//     // that is a block for itself
//     MetadataKind(MetadataKindRecord),

//     Metadata(MetadataRecord),

//     // PARAMATTR_BLOCK
//     ParamAttrEntry(AttributesEntryRecord),
//     // ParamAttrEntryOld(AttributesEntryOldRecord),

//     // PARAMATTR_GROUP_BLOCK
//     Attributes(Vec<u64>),
//     ParamAttrGrpEntry(AttributeGroupEntry),

//     // TYPE_BLOCK
//     Type(Type),

//     // globalvalsummary
//     PerModuleGlobalVarInitRefs(PerModuleGlobalVarInitRefsRecord),

//     OperandBundleTag(String),

//     // STRTAB_BLOCK
//     StrtabBlob(StrtabBlobRecord<'a>),

//     Constant(ConstantRecord),
//     FunctionBlock(FunctionBlockRecord),
//     // ...
//     ValueSymtab(ValueSymtab),

//     // FUNC

//     /// Debug Info
//     SyncScopeName(String),
// }
//
//
#[derive(Debug)]
pub enum FunctionRecord {
    FunctionInst(Inst),
    FunctionBlockRecord(FunctionBlockRecord),
    FunctionDeclareBlocks(u64),
    FunctionBlockAddrUsers(FunctionBlockAddrUsers),
    FunctionOperandBundle(FunctionOperandBundle),
    FunctionDI(DebugInstruction),
}

impl fmt::Debug for MetadataRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::String(r) => r.fmt(f),
            Self::Value(r) => r.fmt(f),
            Self::Node(r) => r.fmt(f),
            Self::DILocation(r) => r.fmt(f),
            Self::DIGenericNode(r) => r.fmt(f),
            Self::DISubrange(r) => r.fmt(f),
            Self::DIGenericSubrange(r) => r.fmt(f),
            Self::DIEnumerator(r) => r.fmt(f),
            Self::DIBasicType(r) => r.fmt(f),
            Self::DIStringType(r) => r.fmt(f),
            Self::DIDerivedType(r) => r.fmt(f),
            Self::DICompositeType(r) => r.fmt(f),
            Self::DISubroutineType(r) => r.fmt(f),
            Self::DIFile(r) => r.fmt(f),
            Self::DICompileUnit(r) => r.fmt(f),
            Self::DISubprogram(r) => r.fmt(f),
            Self::DILexicalBlock(r) => r.fmt(f),
            Self::DILexicalBlockFile(r) => r.fmt(f),
            Self::DICommonBlock(r) => r.fmt(f),
            Self::DINamespace(r) => r.fmt(f),
            Self::DIMacro(r) => r.fmt(f),
            Self::DIMacroFile(r) => r.fmt(f),
            Self::DIArgList(r) => r.fmt(f),
            Self::DIModule(r) => r.fmt(f),
            Self::DIAssignID(r) => r.fmt(f),
            Self::DITemplateTypeParameter(r) => r.fmt(f),
            Self::DITemplateValueParameter(r) => r.fmt(f),
            Self::DIGlobalVariable(r) => r.fmt(f),
            Self::DILocalVariable(r) => r.fmt(f),
            Self::DILabel(r) => r.fmt(f),
            Self::DIExpression(r) => r.fmt(f),
            Self::DIGlobalVariableExpression(r) => r.fmt(f),
            Self::DIObjCProperty(r) => r.fmt(f),
            Self::DIImportedEntity(r) => r.fmt(f),
            Self::UnresolvedReference(r) => write!(f, "(?{r}?)"),
        }
    }
}

pub enum MetadataRecord {
    /// `MDString` extracted from `METADATA_STRINGS` blob
    /// It _is_ part of the metadata node list, even though .ll representation hides it.
    String(String),

    /// Forward references?
    UnresolvedReference(u32),

    /// Record ID: METADATA_VALUE = 2
    /// MetadataClass: ValueAsMetadata / ConstantAsMetadata
    /// MacroSource: HANDLE_METADATA_BRANCH / HANDLE_METADATA_LEAF
    Value(metadata::MetadataValue),

    /// Record ID: METADATA_NODE = 3 or METADATA_DISTINCT_NODE = 5
    /// MetadataClass: MDTuple
    /// MacroSource: HANDLE_MDNODE_LEAF_UNIQUABLE
    Node(metadata::MetadataNode),

    /// Record ID: METADATA_LOCATION = 7
    /// MetadataClass: DILocation
    /// MacroSource: HANDLE_SPECIALIZED_MDNODE_LEAF_UNIQUABLE
    DILocation(metadata::DILocation),

    /// Record ID: METADATA_GENERIC_DEBUG = 12
    /// MetadataClass: GenericDINode
    /// MacroSource: HANDLE_SPECIALIZED_MDNODE_LEAF_UNIQUABLE
    DIGenericNode(metadata::DIGenericNode),

    /// Record ID: METADATA_SUBRANGE = 13
    /// MetadataClass: DISubrange
    /// MacroSource: HANDLE_SPECIALIZED_MDNODE_LEAF_UNIQUABLE
    DISubrange(metadata::DISubrange),

    /// Record ID: METADATA_GENERIC_SUBRANGE = 45
    /// MetadataClass: DIGenericSubrange
    /// MacroSource: HANDLE_SPECIALIZED_MDNODE_LEAF_UNIQUABLE
    DIGenericSubrange(metadata::DIGenericSubrange),

    /// Record ID: METADATA_ENUMERATOR = 14
    /// MetadataClass: DIEnumerator
    /// MacroSource: HANDLE_SPECIALIZED_MDNODE_LEAF_UNIQUABLE
    DIEnumerator(metadata::DIEnumerator),

    /// Record ID: METADATA_BASIC_TYPE = 15
    /// MetadataClass: DIBasicType
    /// MacroSource: HANDLE_SPECIALIZED_MDNODE_LEAF_UNIQUABLE
    DIBasicType(metadata::DIBasicType),

    /// Record ID: METADATA_STRING_TYPE = 41
    /// MetadataClass: DIStringType
    /// MacroSource: HANDLE_SPECIALIZED_MDNODE_LEAF_UNIQUABLE
    DIStringType(metadata::DIStringType),

    /// Record ID: METADATA_DERIVED_TYPE = 17
    /// MetadataClass: DIDerivedType
    /// MacroSource: HANDLE_SPECIALIZED_MDNODE_LEAF_UNIQUABLE
    DIDerivedType(metadata::DIDerivedType),

    /// Record ID: METADATA_COMPOSITE_TYPE = 18
    /// MetadataClass: DICompositeType
    /// MacroSource: HANDLE_SPECIALIZED_MDNODE_LEAF_UNIQUABLE
    DICompositeType(metadata::DICompositeType),

    /// Record ID: METADATA_SUBROUTINE_TYPE = 19
    /// MetadataClass: DISubroutineType
    /// MacroSource: HANDLE_SPECIALIZED_MDNODE_LEAF_UNIQUABLE
    DISubroutineType(metadata::DISubroutineType),

    /// Record ID: METADATA_FILE = 16
    /// MetadataClass: DIFile
    /// MacroSource: HANDLE_SPECIALIZED_MDNODE_LEAF_UNIQUABLE
    DIFile(metadata::DIFile),

    /// Record ID: METADATA_COMPILE_UNIT = 20
    /// MetadataClass: DICompileUnit
    /// MacroSource: HANDLE_SPECIALIZED_MDNODE_LEAF
    DICompileUnit(metadata::DICompileUnit),

    /// Record ID: METADATA_SUBPROGRAM = 21
    /// MetadataClass: DISubprogram
    /// MacroSource: HANDLE_SPECIALIZED_MDNODE_LEAF_UNIQUABLE
    DISubprogram(metadata::DISubprogram),

    /// Record ID: METADATA_LEXICAL_BLOCK = 22
    /// MetadataClass: DILexicalBlock
    /// MacroSource: HANDLE_SPECIALIZED_MDNODE_LEAF_UNIQUABLE
    DILexicalBlock(metadata::DILexicalBlock),

    /// Record ID: METADATA_LEXICAL_BLOCK_FILE = 23
    /// MetadataClass: DILexicalBlockFile
    /// MacroSource: HANDLE_SPECIALIZED_MDNODE_LEAF_UNIQUABLE
    DILexicalBlockFile(metadata::DILexicalBlockFile),

    /// Record ID: METADATA_COMMON_BLOCK = 44
    /// MetadataClass: DICommonBlock
    /// MacroSource: HANDLE_SPECIALIZED_MDNODE_LEAF_UNIQUABLE
    DICommonBlock(metadata::DICommonBlock),

    /// Record ID: METADATA_NAMESPACE = 24
    /// MetadataClass: DINamespace
    /// MacroSource: HANDLE_SPECIALIZED_MDNODE_LEAF_UNIQUABLE
    DINamespace(metadata::DINamespace),

    /// Record ID: METADATA_MACRO = 33
    /// MetadataClass: DIMacro
    /// MacroSource: HANDLE_SPECIALIZED_MDNODE_LEAF_UNIQUABLE
    DIMacro(metadata::DIMacro),

    /// Record ID: METADATA_MACRO_FILE = 34
    /// MetadataClass: DIMacroFile
    /// MacroSource: HANDLE_SPECIALIZED_MDNODE_LEAF_UNIQUABLE
    DIMacroFile(metadata::DIMacroFile),

    /// ??????
    ///
    ///   Not serialized as a !N node (used internally)
    /// MetadataClass: DIArgList
    /// MacroSource: HANDLE_METADATA_LEAF
    DIArgList(metadata::DIArgList),

    /// Record ID: METADATA_MODULE = 32
    /// MetadataClass: DIModule
    /// MacroSource: HANDLE_SPECIALIZED_MDNODE_LEAF_UNIQUABLE
    DIModule(metadata::DIModule),

    /// Record ID: METADATA_ASSIGN_ID = 47
    /// MetadataClass: DIAssignID
    /// MacroSource: HANDLE_SPECIALIZED_MDNODE_LEAF
    DIAssignID(metadata::DIAssignID),

    /// Record ID: METADATA_TEMPLATE_TYPE = 25
    /// MetadataClass: DITemplateTypeParameter
    /// MacroSource: HANDLE_SPECIALIZED_MDNODE_LEAF_UNIQUABLE
    DITemplateTypeParameter(metadata::DITemplateTypeParameter),

    /// Record ID: METADATA_TEMPLATE_VALUE = 26
    /// MetadataClass: DITemplateValueParameter
    /// MacroSource: HANDLE_SPECIALIZED_MDNODE_LEAF_UNIQUABLE
    DITemplateValueParameter(metadata::DITemplateValueParameter),

    /// Record ID: METADATA_GLOBAL_VAR = 27
    /// MetadataClass: DIGlobalVariable
    /// MacroSource: HANDLE_SPECIALIZED_MDNODE_LEAF_UNIQUABLE
    DIGlobalVariable(metadata::DIGlobalVariable),

    /// Record ID: METADATA_LOCAL_VAR = 28
    /// MetadataClass: DILocalVariable
    /// MacroSource: HANDLE_SPECIALIZED_MDNODE_LEAF_UNIQUABLE
    DILocalVariable(metadata::DILocalVariable),

    /// Record ID: METADATA_LABEL = 40
    /// MetadataClass: DILabel
    /// MacroSource: HANDLE_SPECIALIZED_MDNODE_LEAF_UNIQUABLE
    DILabel(metadata::DILabel),

    /// Record ID: METADATA_EXPRESSION = 29
    /// MetadataClass: DIExpression
    /// MacroSource: HANDLE_SPECIALIZED_MDNODE_LEAF_UNIQUABLE
    DIExpression(metadata::DIExpression),

    /// Record ID: METADATA_GLOBAL_VAR_EXPR = 37
    /// MetadataClass: DIGlobalVariableExpression
    /// MacroSource: HANDLE_SPECIALIZED_MDNODE_LEAF_UNIQUABLE
    DIGlobalVariableExpression(metadata::DIGlobalVariableExpression),

    /// Record ID: METADATA_OBJC_PROPERTY = 30
    /// MetadataClass: DIObjCProperty
    /// MacroSource: HANDLE_SPECIALIZED_MDNODE_LEAF_UNIQUABLE
    DIObjCProperty(metadata::DIObjCProperty),

    /// Record ID: METADATA_IMPORTED_ENTITY = 31
    /// MetadataClass: DIImportedEntity
    /// MacroSource: HANDLE_SPECIALIZED_MDNODE_LEAF_UNIQUABLE
    DIImportedEntity(metadata::DIImportedEntity),
}

#[derive(Debug)]
pub enum Type {
    // TYPE_CODE_NUMENTRY (code=1)
    /// `TYPE_CODE_VOID` (code=2)
    Void,

    /// `TYPE_CODE_HALF` (code=10)
    Half,

    /// `TYPE_CODE_BFLOAT` (code=23)
    BFloat,

    /// `TYPE_CODE_FLOAT` (code=3)
    Float,

    /// `TYPE_CODE_DOUBLE` (code=4)
    Double,

    /// `TYPE_CODE_LABEL` (code=5)
    Label,

    /// `TYPE_CODE_OPAQUE` (code=6)
    Opaque,

    /// TYPE_CODE_INTEGER (code=7)
    Integer {
        width: NonZero<u8>,
    },
    // Typed pointers are obsolete
    // Pointer(TypePointerRecord),
    Array(TypeArrayRecord),
    Vector(TypeVectorRecord),

    /// `TYPE_CODE_X86_FP80` (code=13)
    X86Fp80,

    /// `TYPE_CODE_FP128` (code=14)
    Fp128,

    /// `TYPE_CODE_PPC_FP128` (code=15)
    PpcFp128,

    /// `TYPE_CODE_METADATA` (code=16)
    Metadata,

    /// `TYPE_CODE_X86_MMX` (code=17) (deprecated)
    X86Mmx,

    Struct(TypeStructRecord),
    Function(TypeFunctionRecord),
    /// TYPE_CODE_X86_AMX (code=24)
    X86Amx,
    /// 25
    OpaquePointer(TypeOpaquePointerRecord),
    /// 26
    TargetType(TypeTargetTypeRecord),
    Token,
}

/// Write a `GlobalValue` VST to the module. The purpose of this data structure is
/// to allow clients to efficiently find the function body.
#[derive(Debug)]
pub enum ValueSymtab {
    Entry(ValueSymtabEntryRecord),
    Bbentry(ValueSymtabBbentryRecord),
    FnEntry(ValueSymtabFnentryRecord),
    CombinedEntry(ValueSymtabCombinedEntryRecord),
}

#[derive(Debug)]
pub enum DebugInstruction {
    Loc(DebugLoc),
    // FUNC_CODE_DEBUG_LOC_AGAIN = 33 record simply means "reuse the last debug location".
    RecordValue(DebugRecordValue),
    RecordDeclare(DebugRecordDeclare),
    RecordAssign(DebugRecordAssign),
    RecordValueSimple(DebugRecordValueSimple),
    RecordLabel(DebugRecordLabel),
}

#[derive(Debug)]
pub enum Inst {
    /// Binary operation (e.g., add, sub, mul, etc.)
    BinOp(InstBinOp),

    /// Type cast operation (e.g., bitcast, trunc, zext).
    Cast(InstCast),

    /// Extract an element from a vector.
    ExtractElt(InstExtractElt),

    /// Insert an element into a vector.
    InsertElt(InstInsertElt),

    /// Shuffle elements between two vectors.
    ShuffleVec(InstShuffleVec),

    /// Integer or floating-point comparison.
    Cmp(InstCmp),

    /// Return from function.
    ///
    /// (jump)
    Ret(InstRet),

    /// Conditional or unconditional branch.
    ///
    /// (jump)
    Br(InstBr),

    /// Multi-way branch based on a condition.
    ///
    /// (jump)
    Switch(InstSwitch),

    /// Call a function with normal and exceptional successors.
    ///
    /// (call + jump)
    Invoke(InstInvoke),

    /// Marks code that should never be reached.
    ///
    /// (jump)
    /// FUNC_CODE_INST_UNREACHABLE = 15
    Unreachable,

    /// PHI node for SSA form, used at basic block joins.
    Phi(InstPhi),

    /// Stack allocation of memory.
    Alloca(InstAlloca),

    /// Load from memory.
    Load(InstLoad),

    /// Access a variable argument (varargs).
    VAArg(InstVAArg),

    /// Extract a value from an aggregate.
    ExtractVal(InstExtractVal),

    /// Insert a value into an aggregate.
    InsertVal(InstInsertVal),

    /// Indirect branch to an address computed at runtime.
    ///
    /// (jump)
    IndirectBr(InstIndirectBr),

    /// Direct function call.
    ///
    /// Affects control flow.
    Call(InstCall),

    /// Memory fence for ordering operations.
    Fence(InstFence),

    /// Resume execution at a landing pad after an exception.
    ///
    /// (jump)
    Resume(InstResume),

    /// Select one of two values based on a condition.
    Select(InstSelect),

    /// Calculate address offsets (pointer arithmetic).
    Gep(InstGep),

    /// Store a value to memory.
    Store(InstStore),

    /// Atomic compare and exchange memory operation.
    CmpXchg(InstCmpXchg),

    /// Defines a landing pad block for exception handling.
    ///
    /// This affects control flow only as a region header, not a jump or call, but worth noting.
    LandingPad(InstLandingPad),

    /// Return from a catch pad to its successor block.
    ///
    /// (jump)
    CatchRet(InstCatchRet),

    CleanupRet(InstCleanupRet),

    /// Catch block for exception handling.
    CatchPad(InstCatchPad),

    /// Cleanup block in exception handling.
    ///
    /// These are used in EH dispatch blocks. Not direct jumps or calls, but part of control flow.
    CleanupPad(InstCleanupPad),

    /// Dispatch to a catch pad or cleanup.
    ///
    /// (jump)
    CatchSwitch(InstCatchSwitch),

    /// Unary operation (e.g., fneg).
    UnOp(InstUnOp),

    /// Call with potential continuation to multiple blocks.
    ///
    /// (call + jump)
    CallBr(InstCallBr),

    /// Freeze a value to avoid undef propagation.
    Freeze(InstFreeze),

    /// Atomic read-modify-write memory operation.
    AtomicRmw(InstAtomicRmw),
}

impl Inst {
    /// MUST match LLVM's `Instruction::getType()->isVoidTy()`
    #[must_use]
    pub fn is_void_type(&self, global_types: &Types) -> bool {
        let Some(ty_id) = self.ret_type_id(global_types) else {
            return true;
        };
        let Some(ty) = global_types.get(ty_id) else {
            return false;
        };
        matches!(ty, Type::Void)
    }

    #[must_use]
    pub fn ret_type_id(&self, global_types: &Types) -> Option<TypeId> {
        Some(match self {
            Self::BinOp(inst) => inst.operand_ty,
            Self::Cast(inst) => inst.result_ty,
            Self::ExtractElt(inst) => inst.op0_ty,
            Self::InsertElt(inst) => inst.op0_ty,
            Self::ShuffleVec(inst) => inst.vector_ty,
            Self::Cmp(_) => {
                // always i1 FIXME: save the type for easy lookup
                for (i, t) in global_types.types.iter().enumerate() {
                    if matches!(t, Type::Integer { width: n } if n.get() == 1) {
                        return Some(i as u32);
                    }
                }
                panic!()
            }
            Self::Ret(_inst) => return None, //inst.return_ty, // should it copy from outer fn type? is it void?
            Self::Br(_inst) => return None,  // void?
            Self::Select(inst) => inst.condition_ty,
            Self::Switch(_inst) => return None,
            Self::Invoke(inst) => global_types.get_fn(inst.function_ty)?.ret_ty?,
            Self::Unreachable => return None,
            Self::Phi(inst) => inst.ty, // shouldn't be void
            Self::Alloca(inst) => inst.result_ty,
            Self::Load(inst) => inst.ret_ty,
            Self::VAArg(inst) => inst.valist_ty,
            Self::ExtractVal(inst) => inst.ty,
            Self::InsertVal(inst) => inst.element_ty, // or aggregate?
            Self::IndirectBr(_inst) => return None,
            Self::Call(inst) => global_types.get_fn(inst.function_ty)?.ret_ty?,
            Self::Fence(_inst) => return None,
            Self::Resume(_inst) => return None,
            Self::Gep(inst) => {
                inst.base_ty
                // source type? or integer fallback or void!?
                // inst.operands.first().map(|&(_, t)| t).unwrap_or_else(|| {
                //     for (i, t) in global_types.types.iter().enumerate() {
                //         if matches!(t, Type::Integer { .. }) {
                //             return i as u32;
                //         }
                //     }
                //     panic!()
                // })
                //.unwrap_or(inst.source_type)
            }
            Self::Store(_inst) => return None,
            Self::CmpXchg(inst) => inst.ptr_ty,
            Self::LandingPad(inst) => inst.result_ty,
            Self::CatchRet(_inst) => unimplemented!(),
            Self::CleanupRet(_inst) => unimplemented!(),
            Self::CatchPad(_inst) => unimplemented!(),
            Self::CleanupPad(_inst) => unimplemented!(),
            Self::CatchSwitch(_inst) => unimplemented!(),
            Self::UnOp(inst) => inst.operand_ty,
            Self::CallBr(inst) => global_types.get_fn(inst.function_ty)?.ret_ty?,
            Self::Freeze(_inst) => unimplemented!(),
            Self::AtomicRmw(inst) => inst.ptr_ty, // FIXME: not sure which ty
        })
    }

    #[must_use]
    pub const fn is_terminator(&self) -> bool {
        matches!(
            self,
            Self::Ret(..)
                | Self::Br(..)
                | Self::Switch(..)
                | Self::IndirectBr(..)
                | Self::Invoke(..)
                | Self::Resume(..)
                | Self::Unreachable
                | Self::CleanupPad(..)
                | Self::CatchRet(..)
                // CleanupRet???
                | Self::CatchSwitch(..)
                | Self::CallBr(..)
        )
    }
}

/// `VST_CODE_ENTRY`
#[derive(Debug)]
pub struct ValueSymtabEntryRecord {
    pub value_id: ValueId,
    pub name: String,
}

/// `VST_CODE_BBENTRY` = 2
#[derive(Debug)]
pub struct ValueSymtabBbentryRecord {
    pub id: BbId,
    pub name: String,
}

/// `VST_CODE_FNENTRY`
#[derive(Debug)]
pub struct ValueSymtabFnentryRecord {
    pub linkage_value_id: ValueId,
    /// The function’s offset (in 32–bit words) from the start of the bitcode.
    pub function_offset: u64,
    pub name: Option<String>,
}
#[derive(Debug)]
pub struct ValueSymtabCombinedEntryRecord {
    pub linkage_value_id: ValueId,
    pub refguid: u64,
}

/// `FUNC_CODE_INST_BINOP` = 2
/// Binary operations like add, sub, mul, etc.
/// [opcode, ty???, opval, opval]
#[derive(Debug)]
pub struct InstBinOp {
    pub opcode: BinOpcode,
    /// The two operand values that the operation will be performed on
    pub op_vals: [ValueId; 2],
    /// Type of the operands (both operands have the same type)
    pub operand_ty: TypeId,
    pub flags: u8,
}

/// `FUNC_CODE_INST_CAST` = 3
/// Type conversion instruction (bitcast, inttoptr, etc.)
/// [opcode, ty, opty, opval]
#[derive(Debug)]
pub struct InstCast {
    pub opcode: CastOpcode,
    /// The target type to cast to
    pub result_ty: TypeId,
    /// The original type of the operand before casting
    pub operand_ty: TypeId,
    /// The value being cast
    pub operand_val: ValueId,
}

// `FUNC_CODE_INST_SELECT` = 5
//
// obsolete
// [ty, opval, opval, opval]

/// `FUNC_CODE_INST_VSELECT` = 29
/// [ty, opval, opval, predty, pred]
#[derive(Debug)]
pub struct InstSelect {
    pub result_ty: TypeId,
    pub true_val: ValueId,
    pub false_val: ValueId,
    pub condition_ty: TypeId,
    pub condition_val: ValueId,
    pub flags: u8,
}

/// `FUNC_CODE_INST_EXTRACTELT` = 6
/// Extracts a single element from a vector
/// [opty, opval, opval]
#[derive(Debug)]
pub struct InstExtractElt {
    pub op0_ty: TypeId,
    pub op0_val: ValueId,
    pub op1_ty: TypeId,
    pub op1_val: ValueId,
}

/// `FUNC_CODE_INST_INSERTELT` = 7
/// Inserts a value into a vector at specified position
/// [ty, opval, opval, opval]
#[derive(Debug)]
pub struct InstInsertElt {
    pub op0_ty: TypeId,
    pub op0_val: ValueId,
    pub op1: ValueId,
    pub op2_ty: TypeId,
    pub op2_val: ValueId,
}

/// `FUNC_CODE_INST_SHUFFLEVEC` = 8
/// Shuffles elements from two vectors according to a mask
/// [ty, opval, opval, opval]
#[derive(Debug)]
pub struct InstShuffleVec {
    pub vector_ty: TypeId,
    pub vector_val: ValueId,
    pub op: ValueId,
    /// The shuffle mask that indicates which elements to select
    pub mask: ValueId,
}

/// `FUNC_CODE_INST_RET` = 10
/// Returns from a function
///
/// Affects control flow.
/// [opty, opval] (both optional)
///
/// If both are absent, it's a `ret void`.
/// If present, you have the type + the operand.
#[derive(Debug)]
pub struct InstRet {
    /// The return value (None for void)
    pub value: Option<(ValueId, TypeId)>,
}

/// `FUNC_CODE_INST_BR` = 11
/// Branch instruction
/// Possible forms:
/// - [bb#] (unconditional branch)
/// - [bb#, bb#, cond] (conditional branch)
#[derive(Debug)]
pub enum InstBr {
    /// Unconditional branch to a specific basic block
    Uncond { dest_bb: BbId },
    /// Conditional branch to one of two basic blocks based on condition
    Cond {
        true_bb: BbId,
        false_bb: BbId,
        /// The condition value determining which path to take
        condition_val: ValueId,
    },
}

/// `FUNC_CODE_INST_SWITCH` = 12
/// Multi-way branch based on a value comparison
///
/// Affects control flow.
/// [opty, op0, op1, op2, ...]
///
/// Typically:
///   op0 = condition value ID
///   op1 = default basic block ID
///   then pairs of (case value, basic block).
#[derive(Debug)]
pub struct InstSwitch {
    /// The type of the condition value
    pub condition_ty: TypeId,
    /// The value being switched on
    pub condition_val: ValueId,
    /// The default destination basic block if no cases match
    pub default_bb: BbId,
    /// Pairs of (case_value, destination_basic_block)
    /// Each case_value is compared against condition_val
    pub cases: Vec<(ValueId, BbId)>,
}

/// FUNC_CODE_INST_INVOKE = 13
/// Function call with exception handling
///
/// Affects control flow.
/// [attr, fnty, op0, op1, ...]
/// Usually: [attr (bitmask?), function_ty, callee_val, normal_bb, unwind_bb, param0, param1, ...]
#[derive(Debug)]
pub struct InstInvoke {
    pub attr: u64,
    /// The type of the function being called
    pub function_ty: TypeId,
    /// The function value being called
    pub callee_val: ValueId,
    /// The calling convention for the function
    pub calling_conv: CallConv,
    /// The basic block to continue at if call completes normally
    pub normal_bb: BbId,
    /// The basic block to continue at if call raises an exception
    pub unwind_bb: BbId,
    /// Any number of additional argument operands.
    /// Additionally, the first field attr_index references an entry in the PARAMATTR_GROUP or PARAMATTR
    pub args: Vec<CallArg>,
}

/// FUNC_CODE_INST_PHI = 16
/// [ty, val0, bb0, val1, bb1, ...]
/// never a void ty
#[derive(Debug)]
pub struct InstPhi {
    /// The type of the phi node result
    pub ty: TypeId,
    /// Pairs of (value_id, basic_block_id) representing incoming values
    /// Each pair indicates which value to use when coming from a specific block
    pub incoming: Vec<(ValueId, BbId)>,
    pub flags: u8,
}

/// `FUNC_CODE_INST_ALLOCA` = 19
/// Allocates memory on the stack
/// [instty, opty, op, align]
#[derive(Debug)]
pub struct InstAlloca {
    /// The type of the allocation result (pointer type)
    pub result_ty: TypeId,
    /// The type of the array size value
    pub array_size_ty: TypeId,
    /// The value indicating the number of elements to allocate
    pub array_size_val: ValueId,
    pub alignment: u64,
}

/// `FUNC_CODE_INST_LOAD` = 20
/// Loads a value from memory
/// [opty, op, align, vol]
/// `FUNC_CODE_INST_LOADATOMIC` = 41
/// [opty, op, align, vol, ordering, synchscope]
#[derive(Debug)]
pub struct InstLoad {
    /// The type of the pointer being loaded from
    pub ptr_ty: TypeId,
    /// The pointer value to load from
    pub ptr_val: ValueId,
    /// The type of the loaded value
    pub ret_ty: TypeId,
    pub alignment: u64,
    /// Whether this is a volatile load (affects optimization)
    pub is_volatile: bool,
    /// For atomic loads: ordering constraints and synchronization scope
    /// Controls memory visibility guarantees across threads
    pub atomic: Option<(AtomicOrdering, u64)>,
}

/// `FUNC_CODE_INST_VAARG` = 23
/// Handles variable arguments in functions
/// [valistty, valist, instty]
#[derive(Debug)]
pub struct InstVAArg {
    /// The type of the `va_list`
    pub valist_ty: TypeId,
    /// The `va_list` value
    pub valist_val: ValueId,
    /// The type of the argument to extract
    pub result_ty: TypeId,
}

/// `FUNC_CODE_INST_EXTRACTVAL` = 26
/// Extracts a value from an aggregate type (struct/array)
/// [n x operands] — typically [aggregate, idx0, idx1, ...]
#[derive(Debug)]
pub struct InstExtractVal {
    /// The type of the aggregate
    pub ty: TypeId,
    /// The aggregate value to extract from
    pub val: ValueId,
    /// The indices specifying which element to extract
    /// For nested aggregates, multiple indices are used to navigate
    pub operands: Vec<u64>,
}

/// `FUNC_CODE_INST_INSERTVAL` = 27
/// Inserts a value into an aggregate type (struct/array)
/// [n x operands] — typically [aggregate, element, idx0, idx1, ...]
#[derive(Debug)]
pub struct InstInsertVal {
    /// The type of the aggregate
    pub aggregate_ty: TypeId,
    /// The original aggregate value
    pub aggregate_val: ValueId,
    /// The type of the element being inserted
    pub element_ty: TypeId,
    /// The element value to insert
    pub element_val: ValueId,
    /// The indices specifying where to insert the element
    /// For nested aggregates, multiple indices are used to navigate
    pub indices: Vec<u64>,
}

// FUNC_CODE_INST_CMP2 = 28
// [opty, opval, opval, pred]

/// `FUNC_CODE_INST_CMP` = 9
/// Compares two values according to a predicate
/// [opty, opval, opval, pred]
///
/// Same as CMP but returns i1 or [N x i1].
#[derive(Debug)]
pub struct InstCmp {
    /// The type of the operands being compared
    pub operand_ty: TypeId,
    /// The left-hand side value
    pub lhs_val: ValueId,
    /// The right-hand side value
    pub rhs_val: ValueId,
    /// The comparison predicate (e.g., eq, ne, slt, sgt)
    /// Determines how the comparison is performed
    pub predicate: u64,
    pub flags: u64,
}

/// `FUNC_CODE_INST_INDIRECTBR` = 31
/// Indirect branch to one of many possible destinations
/// [opty, op0, op1, ...]
///
/// op0 = address to jump through, then one or more possible destination bbs
#[derive(Debug)]
pub struct InstIndirectBr {
    pub ptr_ty: TypeId,
    /// The address value containing the jump target
    pub address_val: ValueId,
    /// List of all possible destination basic blocks
    /// The runtime value must point to one of these blocks
    pub destinations: Vec<BbId>,
}

/// `FUNC_CODE_INST_CALL` = 34
/// [attr, cc, fnty, fnid, args...]
#[derive(Debug)]
pub struct InstCall {
    /// Optional attribute index referencing `PARAMATTR_GROUP`. Stored as 1-indexed in the bitcode,
    /// but parsed as 0-indexed here.
    pub attributes_index: Option<u32>,
    /// The calling convention used for the call
    pub calling_conv: CallConv,
    pub math_flags: u8,
    /// The actual type of the function being called
    pub function_ty: TypeId,
    /// The function being called
    pub callee_val: ValueId,
    /// This is the type of the callee value, which could be a pointer to function, or potentially another value like a bitcast.
    /// It is derived from the callee operand and includes pointer information (e.g., address space, pointee type).
    pub callee_ty: TypeId,
    pub args: Vec<CallArg>,
}

#[derive(Debug)]
pub enum CallArg {
    /// Regular value argument
    Val(ValueId),
    /// Basic block label argument
    Label(BbId),
    /// Variadic argument with its type
    Var(ValueId, TypeId),
}

impl fmt::Debug for DebugLoc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = f.debug_struct("Loc");
        s.field("line", &format_args!("{}{}:{}", if self.implicit_code {"?"} else {""}, self.line, self.column));
        if let Some(f) = &self.scope { s.field("scope", f); }
        if let Some(f) = &self.inlined_at { s.field("inlined_at", f); }
        s.finish()
    }
}

/// FUNC_CODE_DEBUG_LOC = 35
/// Debug location information
/// [Line, Col, ScopeVal, IAVal]
#[derive(Clone)]
pub struct DebugLoc {
    /// Source code line number
    pub line: u32,
    /// Source code column number
    pub column: u32,
    /// metadata ID for the enclosing scope
    pub scope: Option<Arc<MetadataRecord>>,
    /// metadata ID for the inlined-at location, if any
    pub inlined_at: Option<Arc<MetadataRecord>>,
    /// true if this is implicit code
    pub implicit_code: bool,
}

/// `FUNC_CODE_INST_FENCE` = 36
/// Memory barrier for multi-threaded synchronization
/// [ordering, synchscope]
#[derive(Debug)]
pub struct InstFence {
    /// Memory ordering constraint (e.g., acquire, release, seq_cst)
    /// Controls memory visibility guarantees across threads
    pub ordering: AtomicOrdering,
    /// Synchronization scope (e.g., single-thread, cross-thread)
    pub synch_scope: u64,
}

/// `FUNC_CODE_INST_RESUME` = 39
/// Resumes propagation of an exception
///
/// Affects control flow.
/// [opval]
#[derive(Debug)]
pub struct InstResume {
    /// The type of the exception value
    pub exception_ty: TypeId,
    /// The exception value to resume
    pub exception_val: ValueId,
}

/// FUNC_CODE_INST_GEP = 43
/// GetElementPtr - calculates address within aggregate data structures
/// [inbounds, n x operands]
#[derive(Debug)]
pub struct InstGep {
    /// The base pointer value
    pub base_ptr: ValueId,
    /// The type of the base pointer (what it points to)
    pub base_ty: TypeId,

    /// GEP_INBOUNDS = 0,
    /// GEP_NUSW = 1,
    /// GEP_NUW = 2,
    /// flags
    /// if (GEP->isInBounds())
    /// 1 << bitc::GEP_INBOUNDS;
    /// if (GEP->hasNoUnsignedSignedWrap())
    /// 1 << bitc::GEP_NUSW;
    /// if (GEP->hasNoUnsignedWrap())
    /// 1 << bitc::GEP_NUW;
    /// GEP operation flags:
    /// GEP_INBOUNDS = 0 - Index is guaranteed within bounds (enables optimizations)
    /// GEP_NUSW = 1 - No unsigned/signed wrap
    /// GEP_NUW = 2 - No unsigned wrap
    pub flags: u8,
    /// The source type for the GEP operation
    pub source_type: u32,
    /// Each index navigates into the specified aggregate type
    pub operands: Vec<(ValueId, TypeId)>,
}

/// FUNC_CODE_INST_STORE = 44
/// Stores a value to memory
/// [ptrty, ptr, valty, val, align, vol]
/// FUNC_CODE_INST_STOREATOMIC = 45
/// [ptrty, ptr, val, align, vol, ordering, synchscope]
#[derive(Debug)]
pub struct InstStore {
    pub ptr_ty: TypeId,
    pub ptr_val: ValueId,
    pub stored_ty: TypeId,
    pub stored_val: ValueId,
    pub alignment: u64,
    pub is_volatile: bool,
    /// ordering, synch_scope
    pub atomic: Option<(AtomicOrdering, u64)>,
}

/// FUNC_CODE_INST_CMPXCHG = 46
/// Atomic compare-and-exchange operation
/// [ptrty, ptr, cmp, val, vol,
///  success_ordering, synchscope,
///  failure_ordering, weak]
#[derive(Debug)]
pub struct InstCmpXchg {
    pub ptr_ty: TypeId,
    /// The pointer value where the operation occurs
    pub ptr_val: ValueId,
    pub cmp_ty: TypeId,
    pub cmp_val: ValueId,
    pub new_val: ValueId,
    pub is_volatile: bool,
    /// Memory ordering constraint if operation succeeds
    pub success_ordering: AtomicOrdering,
    pub synch_scope: u64,
    pub failure_ordering: AtomicOrdering,
    /// Whether this is a weak cmpxchg (may spuriously fail)
    pub is_weak: bool,
    pub alignment: u64,
}

/// FUNC_CODE_INST_LANDINGPAD = 47
/// [ty, val, num, id0, val0, id1, val1, ...]
#[derive(Debug)]
pub struct InstLandingPad {
    pub result_ty: TypeId,
    pub is_cleanup: bool,
    // false if it's catch, true if it's filter
    pub clauses: Vec<(bool, (ValueId, TypeId))>,
}

/// `FUNC_CODE_INST_CLEANUPRET` = 48
/// Return from a cleanup handler

// Affects control flow.
/// [val] or [val, bb#]
#[derive(Debug)]
pub struct InstCleanupRet {
    pub cleanup_pad: ValueId,
    pub unwind_dest: Option<BbId>,
}

/// `FUNC_CODE_INST_CATCHRET` = 49
/// Return from a catch handler
///
/// Affects control flow.
/// [val, bb#]
#[derive(Debug)]
pub struct InstCatchRet {
    /// The catch pad being exited
    pub catch_pad: ValueId,
    /// The successor basic block to transfer control to
    ///
    pub successor: BbId,
}

/// `FUNC_CODE_INST_CATCHPAD` = 50
/// Catch pad for exception handling - part of control flow for exceptions
#[derive(Debug)]
pub struct InstCatchPad {
    /// The parent pad this catch pad belongs to
    pub parent_pad: ValueId,
    /// Arguments for the catch pad - typically exception info
    pub args: Vec<(ValueId, TypeId)>,
}

/// `FUNC_CODE_INST_CLEANUPPAD` = 51
/// Cleanup pad for exception handling - part of control flow for exceptions
#[derive(Debug)]
pub struct InstCleanupPad {
    /// The parent pad this cleanup pad belongs to
    pub parent_pad: ValueId,
    /// Arguments for the cleanup pad
    pub args: Vec<(ValueId, TypeId)>,
}

/// `FUNC_CODE_INST_CATCHSWITCH` = 52
/// Catch switch for exception handling
#[derive(Debug)]
pub struct InstCatchSwitch {
    /// The parent pad this catch switch belongs to
    pub parent_pad: ValueId,
    /// List of basic blocks (as ValueIds) containing catch handlers
    /// Control may transfer to any of these blocks when an exception occurs
    pub args: Vec<ValueId>,
    /// Optional unwind destination for exceptions not caught
    /// None means the exception propagates up the call stack
    pub unwind_dest: Option<BbId>,
}

/// `FUNC_CODE_OPERAND_BUNDLE` = 55
/// Bundle of operands for a call or invoke instruction
/// [tag#, value...]
/// A call or an invoke can be optionally prefixed with some variable
/// number of operand bundle blocks.  These blocks are read into
/// OperandBundles and consumed at the next call or invoke instruction.
#[derive(Debug)]
pub struct FunctionOperandBundle {
    /// The tag ID identifying this bundle's type
    /// Common bundles include "deopt", "funclet", "gc-transition"
    pub tag_id: u64,
    /// Values and their types contained in this bundle
    /// Specialized operands for the next call/invoke
    pub values_types: Vec<(ValueId, TypeId)>,
}

/// FUNC_CODE_INST_UNOP = 56
/// Unary operation on a single operand
/// [opcode, ty, opval]
#[derive(Debug)]
pub struct InstUnOp {
    /// The specific unary operation to perform
    pub opcode: u8,
    pub operand_ty: TypeId,
    /// The value to perform the operation on
    pub operand_val: ValueId,
    pub flags: u8,
}

/// FUNC_CODE_INST_CALLBR = 57
/// Call with both normal returns and branches
///
/// Affects control flow.
/// [attr, cc, norm, transfs, fnty, fnid, args...]
#[derive(Debug)]
pub struct InstCallBr {
    /// Attribute mask for the function call
    pub attr: u64,
    /// The calling convention used for the call
    pub calling_conv: CallConv,
    /// The default destination basic block for normal return
    pub normal_bb: BbId,
    /// List of indirect branch destination basic blocks
    /// Function may transfer control to any of these blocks
    pub indirect_bb: Vec<BbId>,
    /// The type of the function being called
    pub function_ty: TypeId,
    /// The function value being called
    pub callee_val: ValueId,
    pub callee_ty: TypeId,
    pub args: Vec<CallArg>,
}

/// `FUNC_CODE_INST_FREEZE` = 58
/// Freezes a value, preventing undefined behavior from poison/undef values
/// [opty, opval]
#[derive(Debug)]
pub struct InstFreeze {
    pub operand_ty: TypeId,
    /// The value to freeze
    pub operand_val: ValueId,
}

/// FUNC_CODE_INST_ATOMICRMW = 59
/// Atomic read-modify-write operation
/// [ptrty, ptr, valty, val,
///  operation, align, vol, ordering, synchscope]
#[derive(Debug)]
pub struct InstAtomicRmw {
    pub ptr_ty: TypeId,
    pub ptr_val: ValueId,
    pub val_ty: TypeId,
    pub stored_val: ValueId,
    /// The specific atomic operation to perform
    /// (e.g., add, sub, and, or, xchg)
    pub operation: u64,
    pub alignment: u64,
    pub is_volatile: bool,
    pub ordering: AtomicOrdering,
    pub synch_scope: u64,
}

/// FUNC_CODE_BLOCKADDR_USERS = 60
/// Records users of block addresses
/// [value...]
#[derive(Debug)]
pub struct FunctionBlockAddrUsers(pub Vec<ValueId>);

/// Represents a `dbg.value` intrinsic with metadata references.
///
/// FUNC_CODE_DEBUG_RECORD_VALUE = 61
/// [DILocation, DILocalVariable, DIExpression, ValueAsMetadata]
#[derive(Debug)]
pub struct DebugRecordValue {
    /// Metadata ID referencing a `DILocation`
    pub di_location: Arc<MetadataRecord>,
    /// Metadata ID referencing a `DILocalVariable`
    pub di_local_variable: Arc<MetadataRecord>,
    /// Metadata ID referencing a `DIExpression`
    pub di_expression: Arc<MetadataRecord>,
    /// Metadata ID referencing a `ValueAsMetadata`
    pub value_as_metadata: Arc<MetadataRecord>,
}

/// Represents a `dbg.declare` intrinsic with metadata references.
///
/// FUNC_CODE_DEBUG_RECORD_DECLARE = 62
/// [DILocation, DILocalVariable, DIExpression, ValueAsMetadata]
#[derive(Debug)]
pub struct DebugRecordDeclare {
    /// Metadata ID referencing a `DILocation`
    pub di_location: Arc<MetadataRecord>,
    /// Metadata ID referencing a `DILocalVariable`
    pub di_local_variable: Arc<MetadataRecord>,
    /// Metadata ID referencing a `DIExpression`
    pub di_expression: Arc<MetadataRecord>,
    /// Metadata ID referencing a `ValueAsMetadata`
    pub value_as_metadata: Arc<MetadataRecord>,
}

/// Represents a `dbg.assign` intrinsic with metadata references.
///
/// FUNC_CODE_DEBUG_RECORD_ASSIGN = 63
/// [DILocation, DILocalVariable, DIExpression, ValueAsMetadata,
///  DIAssignID, DIExpression (addr), ValueAsMetadata (addr)]
#[derive(Debug)]
pub struct DebugRecordAssign {
    /// Metadata ID referencing a `DILocation`
    pub di_location: Arc<MetadataRecord>,
    /// Metadata ID referencing a `DILocalVariable`
    pub di_local_variable: Arc<MetadataRecord>,
    /// Metadata ID referencing a `DIExpression`
    pub di_expression: Arc<MetadataRecord>,
    /// Metadata ID referencing a `ValueAsMetadata`
    pub value_as_metadata: Arc<MetadataRecord>,
    /// Metadata ID referencing a `DIAssignID`
    pub di_assign_id: Arc<MetadataRecord>,
    /// Metadata ID referencing a `DIExpression` for the address
    pub di_expression_addr: Arc<MetadataRecord>,
    /// Metadata ID referencing a `ValueAsMetadata` for the address
    pub value_as_metadata_addr: Arc<MetadataRecord>,
}

/// Represents a `dbg.value` intrinsic with a raw SSA value rather than metadata.
///
/// FUNC_CODE_DEBUG_RECORD_VALUE_SIMPLE = 64
/// [DILocation, DILocalVariable, DIExpression, Value]
#[derive(Debug)]
pub struct DebugRecordValueSimple {
    /// Metadata ID referencing a `DILocation`
    pub di_location: Arc<MetadataRecord>,
    /// Metadata ID referencing a `DILocalVariable`
    pub di_local_variable: Arc<MetadataRecord>,
    /// Metadata ID referencing a `DIExpression`
    pub di_expression: Arc<MetadataRecord>,
    /// Value ID of the SSA value
    pub value: ValueId,
}

/// FUNC_CODE_DEBUG_RECORD_LABEL = 65
/// [DILocation, DILabel]
#[derive(Debug)]
pub struct DebugRecordLabel {
    pub di_location: Arc<MetadataRecord>,
    pub di_label: Arc<MetadataRecord>,
}

pub mod metadata {
    use super::{DebugLoc, MetadataRecord, TypeId, ValueId};
    use std::fmt::{self, Write};
    use std::num::NonZero;
    use std::sync::Arc;

    /// Record ID: METADATA_VALUE = 2
    /// Serialized as a record with two fields: [type_id, value_id].
    /// - type_id: The ID of the value’s type.
    /// - value_id: The ID of the referenced value.
    /// MetadataClass: ValueAsMetadata
    /// MacroSource: HANDLE_METADATA_BRANCH(ValueAsMetadata)
    /// This does NOT receive a !N ID directly, but wraps a value to be used in metadata.
    #[derive(Debug)]
    pub struct MetadataValue {
        pub type_id: TypeId,   // type ID (of a type record)
        pub value_id: ValueId, // value ID (of a Value)
    }

    /// Record ID: METADATA_NODE = 3 (non‐distinct) and METADATA_DISTINCT_NODE = 5 (distinct)
    /// This record represents an MDNode (a tuple of metadata references).
    /// MetadataClass: MDNode
    #[derive(Debug)]
    pub struct MetadataNode {
        /// Whether this metadata node is "distinct".
        pub distinct: bool,
        /// The ordered list of metadata IDs (each an index into the metadata table)
        pub operands: Vec<Option<Arc<MetadataRecord>>>,
    }

    /// Record ID: METADATA_LOCATION = 7
    /// This record encodes a debug location.
    /// Fields:
    ///   [ is_distinct, line, column, scope_id, inlined_at_id?, is_implicit_code ]
    /// MetadataClass: DILocation
    #[derive(Clone)]
    pub struct DILocation {
        /// Whether this metadata node is "distinct".
        pub distinct: bool,
        pub loc: DebugLoc,
    }

    /// Record ID: METADATA_GENERIC_DEBUG = 12
    /// This record is used to encode "generic DI nodes" (a generic debug info node).
    /// Fields:
    ///   [ is_distinct, tag, version, operands... ]
    /// MetadataClass: GenericDINode
    /// This DOES receive a !N ID in bitcode.
    #[derive(Debug)]
    pub struct DIGenericNode {
        pub distinct: bool,
        pub tag: u32,           // e.g. the DW_TAG value
        pub version: u8,        // per‐tag version field (currently always 0)
        pub operands: Vec<u64>, // list of metadata IDs (operands)
    }

    /// Record ID: METADATA_SUBRANGE = 13
    /// This record encodes a DISubrange.
    /// Fields:
    ///   [ (is_distinct | version bits), raw_count_node, raw_lower_bound,
    ///     raw_upper_bound, raw_stride ]
    /// MacroSource: HANDLE_SPECIALIZED_MDNODE_LEAF_UNIQUABLE(DISubrange)
    /// This DOES receive a !N ID in bitcode.
    #[derive(Debug)]
    pub struct DISubrange {
        pub distinct: bool,
        /// Optional metadata ID for the "count" field.
        pub count: Option<u64>,
        /// Optional metadata ID for the lower bound.
        pub lower_bound: Option<u64>,
        /// Optional metadata ID for the upper bound.
        pub upper_bound: Option<u64>,
        /// Optional metadata ID for the stride.
        pub stride: Option<u64>,
    }

    /// Record ID: `METADATA_GENERIC_SUBRANGE`
    /// Similar to `DISubrange` but used for "generic" subranges.
    /// Fields are the same as for `DISubrange`.
    /// MacroSource: HANDLE_SPECIALIZED_MDNODE_LEAF_UNIQUABLE(DIGenericSubrange)
    /// This DOES receive a !N ID in bitcode.
    #[derive(Debug)]
    pub struct DIGenericSubrange {
        pub distinct: bool,
        pub count: Option<u64>,
        pub lower_bound: Option<u64>,
        pub upper_bound: Option<u64>,
        pub stride: Option<u64>,
    }

    /// Record ID: METADATA_ENUMERATOR = 14
    /// This record encodes a DIEnumerator.
    /// Fields:
    ///   [ flags, bit_width, raw_name, wide_value ]
    ///
    /// The first field packs three booleans (is_big_int, is_unsigned, distinct).
    /// MacroSource: HANDLE_SPECIALIZED_MDNODE_LEAF_UNIQUABLE(DIEnumerator)
    /// This DOES receive a !N ID in bitcode.
    #[derive(Debug)]
    pub struct DIEnumerator {
        pub distinct: bool,
        pub is_unsigned: bool,
        pub is_big_int: bool,
        pub bit_width: u32,
        pub name: Option<Arc<MetadataRecord>>, // metadata ID for the name (a string)
        /// The enumerator’s value stored as one or more i64 words.
        pub value: Vec<i64>,
    }

    /// Record ID: METADATA_BASIC_TYPE = 15
    /// This record encodes a DIBasicType.
    /// Fields:
    ///   [ distinct, tag, raw_name, size_in_bits, align_in_bits, encoding, flags ]
    #[derive(Debug)]
    pub struct DIBasicType {
        pub distinct: bool,
        pub tag: u32,
        pub name: Option<Arc<MetadataRecord>>,
        pub size_in_bits: u64,
        pub align_in_bits: u64,
        pub encoding: u64,
        pub flags: u64,
    }

    /// Record ID: `METADATA_STRING_TYPE`
    /// This record encodes a `DIStringType`.
    /// Fields:
    ///   [ distinct, tag, raw_name, string_length, string_length_exp,
    ///     string_location_exp, size_in_bits, align_in_bits, encoding ]
    #[derive(Debug)]
    pub struct DIStringType {
        pub distinct: bool,
        /// DITypeTag,
        pub tag: u32,
        pub raw_name: Option<Arc<MetadataRecord>>,
        pub string_length: Option<Arc<MetadataRecord>>,
        pub string_length_exp: Option<Arc<MetadataRecord>>,
        pub string_location_exp: Option<Arc<MetadataRecord>>,
        pub size_in_bits: u8,
        pub align_in_bits: u8,
        /// DW_ENCODING
        pub encoding: u32,
    }

    /// Record ID: `METADATA_DERIVED_TYPE` = 17
    /// This record encodes a `DIDerivedType`.
    /// Fields:
    ///   [ distinct, tag, raw_name, file, line, scope, base_type,
    ///     size_in_bits, align_in_bits, offset_in_bits, flags, extra_data,
    ///     dwarf_address_space, annotations, ptr_auth_data ]
    #[derive(Debug)]
    pub struct DIDerivedType {
        pub distinct: bool,
        pub tag: u32,
        pub name: Option<Arc<MetadataRecord>>,
        pub file: Option<Arc<MetadataRecord>>,
        pub line: u32,
        pub scope: Option<Arc<MetadataRecord>>,
        pub base_type: Option<Arc<MetadataRecord>>,
        pub size_in_bits: u64,
        pub align_in_bits: u64,
        pub offset_in_bits: u64,
        pub flags: u64,
        pub extra_data: Option<Arc<MetadataRecord>>,
        /// DWARF address space (in bitcode as value+1; the struct has the real value)
        pub dwarf_address_space: Option<u64>,
        pub annotations: Option<Arc<MetadataRecord>>,
        /// Raw pointer‐authentication data
        pub ptr_auth_data: Option<NonZero<u64>>,
    }

    /// Record ID: METADATA_COMPOSITE_TYPE = 18
    /// This record encodes a DICompositeType.
    /// Fields:
    ///   [ distinct, tag, raw_name, file, line, scope, base_type,
    ///     size_in_bits, align_in_bits, offset_in_bits, flags, elements,
    ///     runtime_lang, vtable_holder, template_params, raw_identifier,
    ///     discriminator, raw_data_location, raw_associated, raw_allocated,
    ///     raw_rank, annotations ]
    #[derive(Debug)]
    pub struct DICompositeType {
        pub distinct: bool,
        pub tag: u32,
        pub name: Option<Arc<MetadataRecord>>,
        pub file: Option<Arc<MetadataRecord>>,
        pub line: u32,
        pub scope: Option<Arc<MetadataRecord>>,
        pub base_type: Option<Arc<MetadataRecord>>,
        pub size_in_bits: u64,
        pub align_in_bits: u64,
        pub offset_in_bits: u64,
        pub flags: u8,
        pub elements: Option<Arc<MetadataRecord>>,
        pub runtime_lang: u64,
        pub vtable_holder: Option<Arc<MetadataRecord>>,
        pub template_params: Option<Arc<MetadataRecord>>,
        pub raw_identifier: Option<Arc<MetadataRecord>>,
        pub discriminator: Option<Arc<MetadataRecord>>,
        pub raw_data_location: Option<Arc<MetadataRecord>>,
        pub raw_associated: Option<Arc<MetadataRecord>>,
        pub raw_allocated: Option<Arc<MetadataRecord>>,
        pub raw_rank: Option<Arc<MetadataRecord>>,
        pub annotations: Option<Arc<MetadataRecord>>,
        pub num_extra_inhabitants: u64,
        pub raw_specification: Option<Arc<MetadataRecord>>,
    }

    /// Record ID: `METADATA_SUBROUTINE_TYPE` = 19
    /// This record encodes a `DISubroutineType`.
    /// Fields:
    ///   [ distinct, flags, type_array, cc ]
    #[derive(Debug)]
    pub struct DISubroutineType {
        pub distinct: bool,
        pub flags: u64,
        /// Metadata ID for the type-array (i.e. a list of type IDs).
        pub type_array: Option<Arc<MetadataRecord>>,
        pub cc: u64, // calling convention
    }

    impl fmt::Debug for DILocation {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            self.loc.fmt(f)
        }
    }

    impl fmt::Debug for DILexicalBlock {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let mut s = f.debug_struct("Block");
            s.field("line", &format_args!("{}:{}", self.line, self.column));
            if let Some(f) = &self.file { s.field("file", f); }
            if let Some(f) = &self.scope { s.field("scope", f); }
            s.finish()
        }
    }

    impl fmt::Debug for DINamespace {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let mut s = f.debug_struct("Ns");
            if self.export_symbols { s.field("export", &true); }
            if let Some(f) = &self.name { s.field("name", f); }
            if let Some(f) = &self.scope { s.field("scope", f); }
            s.finish()
        }
    }

    impl fmt::Debug for DIFile {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            if let Some(d) = &self.directory {
                d.fmt(f)?;
                f.write_char('/')?;
            }
            if let Some(d) = &self.filename {
                d.fmt(f)?;
            }
            Ok(())
        }
    }

    /// Record ID: METADATA_FILE = 16
    /// This record encodes a DIFile.
    /// Fields:
    ///   [ distinct, raw_filename, raw_directory, checksum_kind, raw_checksum, raw_source? ]
    pub struct DIFile {
        pub distinct: bool,
        pub filename: Option<Arc<MetadataRecord>>,
        pub directory: Option<Arc<MetadataRecord>>,
        pub checksum_kind: Option<NonZero<u64>>, // 0 if none
        pub raw_checksum: Option<Arc<MetadataRecord>>,
        pub raw_source: Option<Arc<MetadataRecord>>,
    }

    /// Record ID: METADATA_COMPILE_UNIT = 20
    /// This record encodes a DICompileUnit.
    /// Fields:
    ///   [ distinct, source_language, file, raw_producer, is_optimized,
    ///     raw_flags, runtime_version, raw_split_debug_filename, emission_kind,
    ///     enum_types, retained_types, subprograms, global_variables,
    ///     imported_entities, dwo_id, macros, split_debug_inlining,
    ///     debug_info_for_profiling, name_table_kind, ranges_base_address,
    ///     raw_sysroot, raw_sdk ]
    #[derive(Debug)]
    pub struct DICompileUnit {
        pub distinct: bool, // always true
        pub source_language: u32,
        pub file: Option<Arc<MetadataRecord>>,
        pub producer: Option<Arc<MetadataRecord>>,
        pub is_optimized: bool,
        pub raw_flags: Option<Arc<MetadataRecord>>,
        pub runtime_version: u32,
        pub split_debug_filename: Option<Arc<MetadataRecord>>,
        pub emission_kind: u32,
        pub enum_types: Option<Arc<MetadataRecord>>,
        pub retained_types: Option<Arc<MetadataRecord>>,
        pub subprograms: u64, // always 0 in this encoding
        pub global_variables: Option<Arc<MetadataRecord>>,
        pub imported_entities: Option<Arc<MetadataRecord>>,
        pub dwo_id: u64,
        pub macros: Option<Arc<MetadataRecord>>,
        pub split_debug_inlining: u32,
        pub debug_info_for_profiling: u32,
        pub name_table_kind: u32,
        pub ranges_base_address: u64,
        pub raw_sysroot: Option<Arc<MetadataRecord>>,
        pub raw_sdk: Option<Arc<MetadataRecord>>,
    }

    /// Record ID: `METADATA_SUBPROGRAM` = 21
    /// This record encodes a `DISubprogram`.
    /// Fields:
    ///   [ (distinct|unit_flag|sp_flags), scope, raw_name, raw_linkage_name, file,
    ///     line, type, scope_line, containing_type, sp_flags, virtual_index,
    ///     flags, raw_unit, template_params, declaration, retained_nodes,
    ///     this_adjustment, thrown_types, annotations, raw_target_func_name ]
    #[derive(Debug)]
    pub struct DISubprogram {
        pub distinct: bool,
        // (unit flag and sp_flags are packed into the first field in the bitcode)
        pub scope: Option<Arc<MetadataRecord>>,
        pub name: Option<Arc<MetadataRecord>>,
        pub linkage_name: Option<Arc<MetadataRecord>>,
        pub file: Option<Arc<MetadataRecord>>,
        pub line: u32,
        pub type_id: Option<Arc<MetadataRecord>>,
        pub scope_line: u32,
        pub containing_type: Option<Arc<MetadataRecord>>,
        pub sp_flags: u64,
        pub virtual_index: u64,
        pub flags: u64,
        pub raw_unit: Option<Arc<MetadataRecord>>,
        pub template_params: Option<Arc<MetadataRecord>>,
        pub declaration: Option<Arc<MetadataRecord>>,
        pub retained_nodes: Option<Arc<MetadataRecord>>,
        pub this_adjustment: u64,
        pub thrown_types: Option<Arc<MetadataRecord>>,
        pub annotations: Option<Arc<MetadataRecord>>,
        pub raw_target_func_name: Option<Arc<MetadataRecord>>,
    }

    /// Record ID: `METADATA_LEXICAL_BLOCK` = 22
    /// This record encodes a `DILexicalBlock`.
    /// Fields:
    ///   [ distinct, scope, file, line, column ]
    pub struct DILexicalBlock {
        pub distinct: bool,
        pub scope: Option<Arc<MetadataRecord>>,
        pub file: Option<Arc<MetadataRecord>>,
        pub line: u32,
        pub column: u32,
    }

    /// Record ID: `METADATA_LEXICAL_BLOCK_FILE` = 23
    /// This record encodes a `DILexicalBlockFile`.
    /// Fields:
    ///   [ distinct, scope, file, discriminator ]
    #[derive(Debug)]
    pub struct DILexicalBlockFile {
        pub distinct: bool,
        pub scope: Option<Arc<MetadataRecord>>,
        pub file: Option<Arc<MetadataRecord>>,
        pub discriminator: u64,
    }

    /// (No numeric id provided by our snippet; assume "`METADATA_COMMON_BLOCK`")
    /// This record encodes a `DICommonBlock`.
    /// Fields:
    ///   [ distinct, scope, decl, raw_name, file, line_no ]
    #[derive(Debug)]
    pub struct DICommonBlock {
        pub distinct: bool,
        pub scope: Option<Arc<MetadataRecord>>,
        pub decl: Option<u64>,
        pub name: Option<Arc<MetadataRecord>>,
        pub file: Option<Arc<MetadataRecord>>,
        pub line_no: u32,
    }

    /// Record ID: METADATA_NAMESPACE = 24
    /// This record encodes a DINamespace.
    /// The first field packs the "distinct" flag and an "export_symbols" bit.
    pub struct DINamespace {
        pub distinct: bool,
        pub export_symbols: bool,
        pub scope: Option<Arc<MetadataRecord>>,
        pub name: Option<Arc<MetadataRecord>>,
    }

    /// Record ID: `METADATA_MACRO` = 33
    /// This record encodes a `DIMacro`.
    /// Fields:
    ///   [ distinct, macinfo_type, line, raw_name, raw_value ]
    #[derive(Debug)]
    pub struct DIMacro {
        pub distinct: bool,
        pub macinfo_type: u32,
        pub line: u32,
        pub name: Option<Arc<MetadataRecord>>,
        pub raw_value: Option<u64>,
    }

    /// Record ID: `METADATA_MACRO_FILE` = 34
    /// This record encodes a `DIMacroFile`.
    /// Fields:
    ///   [ distinct, macinfo_type, line, file, elements ]
    #[derive(Debug)]
    pub struct DIMacroFile {
        pub distinct: bool,
        pub macinfo_type: u32,
        pub line: u32,
        pub file: Option<Arc<MetadataRecord>>,
        pub elements: Option<u64>,
    }

    /// Record ID: `METADATA_ARG_LIST` = 35
    /// This record encodes a `DIArgList`.
    /// It is simply a list of metadata IDs (one per argument).
    #[derive(Debug)]
    pub struct DIArgList {
        pub args: Vec<u64>,
    }

    /// Record ID: `METADATA_MODULE` = (see bitcode constant)
    /// This record encodes a `DIModule`.
    /// Fields:
    ///   [ distinct, operands..., line_no, is_decl ]
    #[derive(Debug)]
    pub struct DIModule {
        pub distinct: bool,
        pub operands: Vec<u64>,
        pub line_no: u32,
        pub is_decl: bool,
    }

    /// Record ID: `METADATA_ASSIGN_ID` = 36
    /// This record encodes a `DIAssignID`. (It has no operands other than the distinct flag.)
    #[derive(Debug)]
    pub struct DIAssignID {
        pub distinct: bool,
    }

    /// Record ID: `METADATA_TEMPLATE_TYPE` = 25
    /// This record encodes a `DITemplateTypeParameter`.
    /// Fields:
    ///   [ distinct, raw_name, type, is_default ]
    #[derive(Debug)]
    pub struct DITemplateTypeParameter {
        pub distinct: bool,
        pub name: Option<Arc<MetadataRecord>>,
        pub type_id: Option<Arc<MetadataRecord>>,
        pub is_default: bool,
    }

    /// Record ID: `METADATA_TEMPLATE_VALUE` = 26
    /// This record encodes a `DITemplateValueParameter`.
    /// Fields:
    ///   [ distinct, tag, raw_name, type, is_default, raw_value ]
    #[derive(Debug)]
    pub struct DITemplateValueParameter {
        pub distinct: bool,
        pub tag: u32,
        pub name: Option<Arc<MetadataRecord>>,
        pub type_id: Option<u64>,
        pub is_default: bool,
        pub raw_value: Option<u64>,
    }

    /// Record ID: `METADATA_GLOBAL_VAR` = 27
    /// This record encodes a `DIGlobalVariable`.
    /// Fields:
    ///   [ version (distinct|…), scope, raw_name, raw_linkage_name, file, line,
    ///     type, is_local_to_unit, is_definition, static_data_member_declaration,
    ///     template_params, align_in_bits, annotations ]
    #[derive(Debug)]
    pub struct DIGlobalVariable {
        pub distinct: bool,
        pub scope: Option<Arc<MetadataRecord>>,
        pub name: Option<Arc<MetadataRecord>>,
        pub linkage_name: Option<Arc<MetadataRecord>>,
        pub file: Option<Arc<MetadataRecord>>,
        pub line: u32,
        pub type_id: Option<u64>,
        pub is_local_to_unit: bool,
        pub is_definition: bool,
        pub static_data_member_declaration: Option<u64>,
        pub template_params: Option<u64>,
        pub align_in_bits: u64,
        pub annotations: Option<u64>,
    }

    /// Record ID: `METADATA_LOCAL_VAR` = 28
    /// This record encodes a `DILocalVariable`.
    /// Fields:
    ///   [ (distinct|has_alignment), scope, raw_name, file, line, type, arg,
    ///     flags, align_in_bits, annotations ]
    #[derive(Debug)]
    pub struct DILocalVariable {
        pub distinct: bool,
        pub scope: Option<Arc<MetadataRecord>>,
        pub name: Option<Arc<MetadataRecord>>,
        pub file: Option<Arc<MetadataRecord>>,
        pub line: u32,
        pub type_id: Option<u64>,
        pub arg: u64,
        pub flags: u64,
        pub align_in_bits: u64,
        pub annotations: Option<u64>,
    }

    /// Record ID: METADATA_LABEL (e.g.  ?)
    /// This record encodes a DILabel.
    /// Fields:
    ///   [ distinct, scope, raw_name, file, line ]
    #[derive(Debug)]
    pub struct DILabel {
        pub distinct: bool,
        pub scope: Option<Arc<MetadataRecord>>,
        pub name: Option<Arc<MetadataRecord>>,
        pub file: Option<Arc<MetadataRecord>>,
        pub line: u32,
    }

    /// Record ID: `METADATA_EXPRESSION` = 29
    /// This record encodes a `DIExpression`.
    /// Fields:
    ///   [ (distinct|version), elements... ]
    #[derive(Debug)]
    pub struct DIExpression {
        pub distinct: bool,
        /// A vector of signed integers that make up the expression.
        pub elements: Vec<i64>,
    }

    /// Record ID: `METADATA_GLOBAL_VAR_EXPR` = 30
    /// This record encodes a `DIGlobalVariableExpression`.
    /// Fields:
    ///   [ distinct, variable, expression ]
    #[derive(Debug)]
    pub struct DIGlobalVariableExpression {
        pub distinct: bool,
        pub variable: u64,
        pub expression: u64,
    }

    /// Record ID: `METADATA_OBJC_PROPERTY` = 31
    /// This record encodes a `DIObjCProperty`.
    /// Fields:
    ///   [ distinct, raw_name, file, line, raw_setter_name, raw_getter_name,
    ///     attributes, type ]
    #[derive(Debug)]
    pub struct DIObjCProperty {
        pub distinct: bool,
        pub name: Option<Arc<MetadataRecord>>,
        pub file: Option<Arc<MetadataRecord>>,
        pub line: u32,
        pub raw_setter_name: Option<u64>,
        pub raw_getter_name: Option<u64>,
        pub attributes: u64,
        pub type_id: Option<u64>,
    }

    /// Record ID: `METADATA_IMPORTED_ENTITY` = 32
    /// This record encodes a `DIImportedEntity`.
    /// Fields:
    ///   [ distinct, tag, scope, entity, line, raw_name, raw_file, elements ]
    #[derive(Debug)]
    pub struct DIImportedEntity {
        pub distinct: bool,
        pub tag: u32,
        pub scope: Option<Arc<MetadataRecord>>,
        pub entity: Option<u64>,
        pub line: u32,
        pub name: Option<Arc<MetadataRecord>>,
        pub file: Option<Arc<MetadataRecord>>,
        pub elements: Option<u64>,
    }

    /// Record ID: `METADATA_NAMED_NODE` = 10
    ///
    /// The name is from `METADATA_NAME` before it.
    /// This record encodes a named metadata node’s operands.
    /// Fields:
    ///   [ mdnode IDs ]
    #[derive(Debug)]
    pub struct MetadataNamedNode {
        pub name: String,
        pub mdnodes: Vec<u64>, // IDs of the MDNodes in this named node
    }
}

use rustc_demangle::try_demangle;

/// Not a record: `CST_CODE_SETTYPE` = 1.
#[derive(Debug)]
pub enum ConstantRecord {
    ConstantInteger(ConstantInteger),
    ConstantWideInteger(ConstantWideInteger),
    ConstantFloat(ConstantFloat),
    ConstantAggregate(ConstantAggregate),
    ConstantString(ConstantString),
    ConstantCString(ConstantCString),
    ConstantBinaryOp(ConstantBinaryOp),
    ConstantCast(ConstantCast),
    ConstantGEP(ConstantGEP),
    ConstantSelect(ConstantSelect),
    ConstantExtractElement(ConstantExtractElement),
    ConstantInsertElement(ConstantInsertElement),
    ConstantShuffleVector(ConstantShuffleVector),
    ConstantCompare(ConstantCompare),
    ConstantBlockAddress(ConstantBlockAddress),
    ConstantInlineASM(ConstantInlineASM),

    /// Represents a poison value.
    /// Corresponds to `CST_CODE_POISON` = 26.
    ConstantPoison,
    ConstantDSOLocalEquivalent(ConstantDSOLocalEquivalent),
    ConstantNoCFI(ConstantNoCFI),
    ConstantPtrAuth(ConstantPtrAuth),

    /// Represents a null constant.
    /// Corresponds to `CST_CODE_NULL` = 2.
    ConstantNull(u32),

    /// Represents an undefined constant.
    /// Corresponds to `CST_CODE_UNDEF` = 3.
    ConstantUndef,
}

impl ConstantRecord {
    /// Returns the type ID of this constant if available.
    #[must_use]
    pub fn get_type_id(&self) -> Option<u32> {
        Some(match self {
            Self::ConstantInteger(c) => c.ty,
            Self::ConstantWideInteger(c) => c.ty,
            Self::ConstantFloat(c) => c.ty,
            Self::ConstantAggregate(c) => c.ty,
            Self::ConstantString(c) => c.ty,
            Self::ConstantCString(c) => c.ty,
            Self::ConstantBinaryOp(c) => c.ty,
            Self::ConstantCast(c) => c.ty,
            Self::ConstantGEP(c) => c.ty,
            Self::ConstantSelect(c) => c.ty,
            Self::ConstantExtractElement(c) => c.operand_ty,
            Self::ConstantInsertElement(c) => c.ty,
            Self::ConstantShuffleVector(c) => c.ty,
            Self::ConstantCompare(c) => c.ty,
            Self::ConstantBlockAddress(c) => c.ty,
            Self::ConstantInlineASM(c) => c.ty,
            Self::ConstantPoison => return None,
            Self::ConstantDSOLocalEquivalent(c) => c.ty,
            Self::ConstantNoCFI(c) => c.ty,
            Self::ConstantPtrAuth(c) => c.ty,
            Self::ConstantNull(ty) => *ty,
            Self::ConstantUndef => return None,
        })
    }
}

/// Represents an integer constant.
/// Corresponds to `CST_CODE_INTEGER` = 4.
#[derive(Debug)]
pub struct ConstantInteger {
    pub ty: TypeId,
    /// The value of the integer constant.
    pub value: i64,
}

/// Represents a wide integer constant.
/// Corresponds to `CST_CODE_WIDE_INTEGER` = 5.
#[derive(Debug)]
pub struct ConstantWideInteger {
    pub ty: TypeId,
    /// The values of the wide integer constant (multiple words).
    pub values: Vec<u64>,
}

/// Represents a floating-point constant.
/// Corresponds to `CST_CODE_FLOAT` = 6.
#[derive(Debug)]
pub struct ConstantFloat {
    pub ty: TypeId,
    /// The IEEE floating-point representation.
    pub value: f64,
}

/// Represents an aggregate constant (array or struct).
/// Corresponds to `CST_CODE_AGGREGATE` = 7.
#[derive(Debug)]
pub struct ConstantAggregate {
    pub ty: TypeId,
    /// The values of the aggregate.
    pub values: Vec<u64>, // References to other constants
}

/// Represents a string constant.
/// Corresponds to `CST_CODE_STRING` = 8 or `CST_CODE_DATA`.
#[derive(Debug)]
pub struct ConstantString {
    pub ty: TypeId,
    /// The value of the string.
    pub value: Vec<u8>,
}

/// Represents a C-style string constant.
/// Corresponds to `CST_CODE_CSTRING` = 9.
#[derive(Debug)]
pub struct ConstantCString {
    pub ty: TypeId,
    /// The value of the C string.
    pub value: Vec<u8>,
}

/// Represents a binary operator constant expression.
/// Corresponds to `CST_CODE_CE_BINOP` = 10.
#[derive(Debug)]
pub struct ConstantBinaryOp {
    pub ty: TypeId,
    /// The opcode of the binary operation.
    pub opcode: BinOpcode,
    /// Left operand.
    pub lhs: ValueId,
    /// Right operand.
    pub rhs: ValueId,
    /// optimization flags
    pub flags: u8,
}

/// Represents a cast constant expression.
/// Corresponds to `CST_CODE_CE_CAST` = 11.
/// [opcode, opty, opval]
#[derive(Debug)]
pub struct ConstantCast {
    /// The opcode of the cast.
    pub opcode: CastOpcode,
    /// The type ID of the operand.
    pub ty: TypeId,
    /// The operand value.
    pub operand: ValueId,
}

/// Represents a getelementptr constant expression.
/// Corresponds to `CST_CODE_CE_GEP` = 32 and `CST_CODE_CE_GEP_WITH_INRANGE` = 31.
#[derive(Debug)]
pub struct ConstantGEP {
    pub ty: TypeId,
    /// The type of the base pointer.
    pub base_type: u32,
    /// Flags (e.g., inbounds).
    pub flags: u8,
    /// Optional inrange index.
    pub inrange: Option<Range<i64>>,
    /// List of operands (offsets).
    pub operands: Vec<(TypeId, ValueId)>,
}

/// Represents a select constant expression.
/// Corresponds to `CST_CODE_CE_SELECT` = 13.
#[derive(Debug)]
pub struct ConstantSelect {
    pub ty: TypeId,
    /// The condition value.
    pub condition: u64,
    /// The value if true.
    pub true_value: u64,
    /// The value if false.
    pub false_value: u64,
}

/// Represents an extractelement constant expression.
/// Corresponds to `CST_CODE_CE_EXTRACTELT` = 14.
#[derive(Debug)]
pub struct ConstantExtractElement {
    pub operand_ty: TypeId,
    pub operand_val: ValueId,
    pub index_ty: TypeId,
    pub index_val: ValueId,
}

/// Represents an insertelement constant expression.
/// Corresponds to `CST_CODE_CE_INSERTELT` = 15.
#[derive(Debug)]
pub struct ConstantInsertElement {
    pub ty: TypeId,
    /// The type ID of the operand.
    pub operand_type: u32,
    /// The vector operand.
    pub vector: u64,
    /// The element to insert.
    pub element: u64,
    /// The index to insert at.
    pub index: u64,
}

/// Represents a shufflevector constant expression.
/// Corresponds to `CST_CODE_CE_SHUFFLEVEC` = 16.
#[derive(Debug)]
pub struct ConstantShuffleVector {
    pub ty: TypeId,
    /// First input vector.
    pub vector1: u64,
    /// Second input vector.
    pub vector2: u64,
    /// Shuffle mask.
    pub mask: u64,
}

/// Represents a comparison constant expression.
/// Corresponds to `CST_CODE_CE_CMP` = 17.
#[derive(Debug)]
pub struct ConstantCompare {
    pub ty: TypeId,
    /// The type ID of the operands.
    pub operand_type: u32,
    /// Left operand.
    pub lhs: u64,
    /// Right operand.
    pub rhs: u64,
    /// The predicate.
    pub predicate: u8,
}

/// Represents a block address constant.
/// Corresponds to `CST_CODE_BLOCKADDRESS` = 21.
#[derive(Debug)]
pub struct ConstantBlockAddress {
    pub ty: TypeId,
    /// The function containing the block.
    pub function: ValueId,
    /// The basic block index.
    pub block: u64,
}

/// Represents an inline assembly constant.
/// Corresponds to `CST_CODE_INLINEASM` = 30.
#[derive(Debug)]
pub struct ConstantInlineASM {
    pub ty: TypeId,
    /// The function type.
    pub function_type: u32,
    /// Flags (side effects, dialect, unwind).
    pub flags: u8,
    /// The assembly string.
    pub asm: String,
    /// The constraint string.
    pub constraints: String,
}

/// Represents a DSO local equivalent value.
/// Corresponds to `CST_CODE_DSO_LOCAL_EQUIVALENT` = 27.
#[derive(Debug)]
pub struct ConstantDSOLocalEquivalent {
    pub ty: TypeId,
    /// The global variable type.
    pub gv_type: u32,
    /// The global variable.
    pub gv: u64,
}

/// Represents a `no_cfi` value.
/// Corresponds to `CST_CODE_NO_CFI_VALUE` = 29.
#[derive(Debug)]
pub struct ConstantNoCFI {
    pub ty: TypeId,
    /// The function type.
    pub function_type: u32,
    /// The function.
    pub function: u64,
}

/// Represents a pointer authentication constant.
/// Corresponds to `CST_CODE_PTRAUTH` = 33.
#[derive(Debug)]
pub struct ConstantPtrAuth {
    pub ty: TypeId,
    /// The pointer value.
    pub pointer: u64,
    /// The key.
    pub key: u64,
    /// The discriminator.
    pub discriminator: u64,
    /// The address discriminator.
    pub address_discriminator: u64,
}

#[derive(Debug, Clone, Copy, TryFromPrimitive)]
#[repr(u8)]
pub enum ParamAttrGroupCodes {
    /// Enum attribute (e.g., `AlwaysInline`, `NoInline`)
    EnumAttr = 0,
    /// Integer attribute (e.g., Alignment, `StackAlignment`)
    IntAttr = 1,
    /// String attribute
    StringAttr = 3,
    /// Key + Value
    StringAttrWithValue = 4,
    /// Type attribute (e.g., `ByVal`)
    TypeAttr = 5,
    TypeAttrTypeId = 6,
    ConstantRange = 7,
    /// Constant range list attribute
    ConstantRangeList = 8,
}

/// Represents an attribute group entry in the parameter attribute group block.
/// Corresponds to `PARAMATTR_GRP_CODE_ENTRY` = 3.
#[derive(Debug)]
pub struct AttributeGroupEntry {
    /// Unique attribute group ID.
    pub group_id: u64,
    /// Index into the function signature where the attributes apply (0 for return, 1-N for parameters).
    pub index: u64,
    /// List of attributes.
    pub attributes: Vec<Attribute>,
}

/// Represents a single attribute in the attribute group entry.
pub enum Attribute {
    AttrKind(AttrKind),
    Int {
        kind: AttrKind,
        value: u64,
    },
    String {
        key: String,
        value: Option<String>,
    },
    Type {
        kind: AttrKind,
        type_id: Option<u64>,
    },
    ConstantRange {
        kind: AttrKind,
        bit_width: u32,
        range: Range<i64>,
    },
    ConstantRangeList {
        kind: AttrKind,
        bit_width: u32,
        ranges: Vec<Range<i64>>,
    },
}

impl fmt::Debug for Attribute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AttrKind(attr_kind) => attr_kind.fmt(f),
            Self::Int { kind, value } => write!(f, "{kind:?}={value:?}"),
            Self::String { key, value } => {
                write!(f, "{key}={:?}", value.as_deref().unwrap_or_default())
            }
            Self::Type { kind, type_id } => write!(f, "{kind:?}={type_id:?}"),
            Self::ConstantRange { kind, bit_width, range } => write!(f, "{kind:?}={bit_width}={range:?}"),
            Self::ConstantRangeList { kind, bit_width, ranges } => write!(f, "{kind:?}={bit_width}={ranges:?}"),
        }
    }
}

#[derive(Debug)]
pub struct Types {
    pub types: Vec<Type>,
}

pub enum LLVMTypeID {
    // PrimitiveTypes
    Half = 0,
    BFloat,
    Float,
    Double,
    X86Fp80,
    FP128,
    PpcFp128,
    Void,
    Label,
    Metadata,
    X86Amx,
    Token,

    // Derived types... see DerivedTypes.h file.
    Integer,
    Function,
    Pointer,
    Struct,
    Array,
    FixedVector,
    ScalableVector,
    TypedPointer,
    TargetExt,
}

#[derive(Debug, Clone, Copy, TryFromPrimitive, Eq, PartialEq, Hash)]
#[repr(u8)]
pub enum MetadataKind {
    /// dbg
    Dbg = 0,
    /// tbaa
    Tbaa = 1,
    /// prof
    Prof = 2,
    /// fpmath
    Fpmath = 3,
    /// range
    Range = 4,
    /// tbaa.struct
    TbaaStruct = 5,
    /// invariant.load
    InvariantLoad = 6,
    /// alias.scope
    AliasScope = 7,
    /// noalias
    Noalias = 8,
    /// nontemporal
    Nontemporal = 9,
    /// `llvm.mem.parallel_loop_access`
    MemParallelLoopAccess = 10,
    /// nonnull
    Nonnull = 11,
    /// dereferenceable
    Dereferenceable = 12,
    /// `dereferenceable_or_null`
    DereferenceableOrNull = 13,
    /// make.implicit
    MakeImplicit = 14,
    /// unpredictable
    Unpredictable = 15,
    /// invariant.group
    InvariantGroup = 16,
    /// align
    Align = 17,
    /// llvm.loop
    Loop = 18,
    /// type
    Type = 19,
    /// `section_prefix`
    SectionPrefix = 20,
    /// `absolute_symbol`
    AbsoluteSymbol = 21,
    /// associated
    Associated = 22,
    /// callees
    Callees = 23,
    /// `irr_loop`
    IrrLoop = 24,
    /// llvm.access.group
    AccessGroup = 25,
    /// callback
    Callback = 26,
    /// llvm.preserve.access.index
    PreserveAccessIndex = 27,
    /// `vcall_visibility`
    VcallVisibility = 28,
    /// noundef
    Noundef = 29,
    /// annotation
    Annotation = 30,
    /// nosanitize
    Nosanitize = 31,
    /// `func_sanitize`
    FuncSanitize = 32,
    /// exclude
    Exclude = 33,
    /// memprof
    Memprof = 34,
    /// callsite
    Callsite = 35,
    /// `kcfi_type`
    KcfiType = 36,
    /// pcsections
    Pcsections = 37,
    /// `DIAssignID`
    DIAssignID = 38,
    /// coro.outside.frame
    CoroOutsideFrame = 39,
    /// mmra
    Mmra = 40,
    /// noalias.addrspace
    NoaliasAddrspace = 41,
    /// srcloc
    Srcloc = 42,
    /// Not supported by this implementation
    Unknown = 255,
}

impl Types {
    #[must_use]
    pub fn get_fn(&self, fn_ty_id: u32) -> Option<&TypeFunctionRecord> {
        match self.get(fn_ty_id) {
            Some(Type::Function(fty)) => Some(fty),
            _ => None,
        }
    }

    #[must_use]
    pub fn get(&self, ty_id: u32) -> Option<&Type> {
        self.types.get(ty_id as usize)
    }

    #[must_use]
    pub fn llvm_basic_type_id(&self, ty_id: u32) -> Option<LLVMTypeID> {
        Some(match &self.types[ty_id as usize] {
            Type::Void => LLVMTypeID::Void,
            Type::Half => LLVMTypeID::Half,
            Type::BFloat => LLVMTypeID::BFloat,
            Type::Float => LLVMTypeID::Float,
            Type::Double => LLVMTypeID::Double,
            Type::Label => LLVMTypeID::Label,
            Type::Opaque => LLVMTypeID::Struct,
            Type::Integer { width: _ } => LLVMTypeID::Integer,
            Type::Array(_type_array_record) => LLVMTypeID::Array,
            Type::Vector(_type_vector_record) => LLVMTypeID::FixedVector,
            Type::X86Fp80 => LLVMTypeID::X86Fp80,
            Type::Fp128 => LLVMTypeID::FP128,
            Type::PpcFp128 => LLVMTypeID::PpcFp128,
            Type::Metadata => LLVMTypeID::Metadata,
            Type::X86Mmx => return None,
            Type::Struct(_type_struct_record) => LLVMTypeID::Struct,
            Type::Function(_type_function_record) => LLVMTypeID::Function,
            Type::X86Amx => LLVMTypeID::X86Amx,
            Type::OpaquePointer(_type_opaque_pointer_record) => LLVMTypeID::Pointer,
            Type::TargetType(_type_target_type_record) => LLVMTypeID::TargetExt,
            Type::Token => LLVMTypeID::Token,
        })
    }
}
