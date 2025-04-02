use num_enum::TryFromPrimitive;

/// Enumeration of block identifiers in LLVM bitcode format.
#[derive(Hash, Eq, PartialEq, Debug, Clone, Copy, TryFromPrimitive)]
#[repr(u8)]
#[non_exhaustive]
pub enum BlockId {
    /// `MODULE` block identifier
    Module = 8,

    /// `PARAMATTR` block identifier
    ParamAttr,

    /// `PARAMATTR_GROUP` block identifier
    ParamAttrGroup,

    /// `CONSTANTS_BLOCK_ID = 11`
    Constants,

    /// `FUNCTION_BLOCK_ID = 12`
    Function,

    /// Obsolete.
    ///
    /// Block intended to contain information on the bitcode versioning. Can be
    /// used to provide better error messages when we fail to parse a bitcode file.
    Identification,

    /// `VALUE_SYMTAB_BLOCK_ID`
    ValueSymtab,

    /// `METADATA_BLOCK_ID`
    Metadata,

    /// `METADATA_ATTACHMENT_ID`
    MetadataAttachment,

    /// `TYPE_BLOCK_ID_NEW = 17`
    Type = 17,

    /// `USELIST_BLOCK_ID`
    Uselist,

    /// `MODULE_STRTAB_BLOCK_ID`
    ModuleStrtab,

    /// Obsolete
    /// `GLOBALVAL_SUMMARY_BLOCK_ID`
    GlobalvalSummary,

    /// `OPERAND_BUNDLE_TAGS_BLOCK_ID`
    OperandBundleTags,

    /// `METADATA_KIND_BLOCK_ID`
    MetadataKind,

    /// `STRTAB_BLOCK_ID`
    Strtab,

    /// `FULL_LTO_GLOBALVAL_SUMMARY_BLOCK_ID`
    FullLtoGlobalvalSummary,

    /// `SYMTAB_BLOCK_ID`
    Symtab,

    /// `SYNC_SCOPE_NAMES_BLOCK_ID`
    SyncScopeNames,
}

/// OperandBundle tag codes
#[derive(Debug, Clone, Copy, TryFromPrimitive)]
#[repr(u8)]
#[non_exhaustive]
pub enum OperandBundleTagCode {
    /// `TAG`
    ///
    /// [strchr x N]
    Tag = 1,
}

/// Sync scope name codes
#[derive(Debug, Clone, Copy, TryFromPrimitive)]
#[repr(u8)]
#[non_exhaustive]
pub enum SyncScopeNameCode {
    /// `SYNC_SCOPE_NAME`
    Name = 1,
}

/// STRTAB block codes
#[derive(Debug, Clone, Copy, TryFromPrimitive)]
#[repr(u8)]
#[non_exhaustive]
pub enum StrtabCode {
    /// `STRTAB_BLOB`
    Blob = 1,
}

/// SYMTAB block codes
#[derive(Debug, Clone, Copy, TryFromPrimitive)]
#[repr(u8)]
#[non_exhaustive]
pub enum SymtabCode {
    /// `SYMTAB_BLOB`
    Blob = 1,
}

/// `MODULE` blocks have a number of optional fields and subblocks.
#[derive(Debug, Clone, Copy, TryFromPrimitive)]
#[repr(u8)]
#[non_exhaustive]
pub enum ModuleCode {
    /// `VERSION`
    ///
    /// [version#]
    Version = 1,

    /// `TRIPLE`
    ///
    /// [strchr x N]
    Triple = 2,

    /// `DATALAYOUT`
    ///
    /// [strchr x N]
    Datalayout = 3,

    /// `ASM`
    ///
    /// [strchr x N]
    Asm = 4,

    /// `SECTIONNAME`
    ///
    /// [strchr x N]
    SectionName = 5,

    /// Obsolete.
    ///
    /// `DEPLIB`
    ///
    /// [strchr x N]
    Deplib = 6,

    /// `GLOBALVAR`
    ///
    /// [pointer type, isconst, initid, linkage, alignment, section, visibility, threadlocal]
    GlobalVar = 7,

    /// `FUNCTION`
    ///
    /// [type, callingconv, isproto, linkage, paramattrs, alignment, section, visibility, gc, unnamed_addr]
    Function = 8,

    /// Obsolete alias record; replaced by `MODULE_CODE_ALIAS`
    ///
    /// `ALIAS`
    ///
    /// [alias type, aliasee val#, linkage, visibility]
    AliasOld = 9,

    /// `GCNAME`
    ///
    /// [strchr x N]
    GCName = 11,

    /// `COMDAT`
    ///
    /// [selection_kind, name]
    Comdat = 12,

    /// `VSTOFFSET`
    ///
    /// [offset]
    VstOffset = 13,

    /// `ALIAS`
    ///
    /// [alias value type, addrspace, aliasee val#, linkage, visibility]
    Alias = 14,

    /// Defined in the MODULE block but never emitted (Obsolete)
    MetadataValuesUnused = 15,

    /// `SOURCE_FILENAME`
    ///
    /// [namechar x N]
    SourceFilename = 16,

    /// `HASH`
    ///
    /// [5*i32]
    Hash = 17,

    /// `IFUNC`
    ///
    /// [ifunc value type, addrspace, resolver val#, linkage, visibility]
    Ifunc = 18,
}

/// The global value summary block contains codes for defining the global value summary information.
#[derive(Debug, Clone, Copy, TryFromPrimitive)]
#[repr(u8)]
#[non_exhaustive]
pub enum GlobalValueSummaryCode {
    /// `PERMODULE`
    ///
    /// [valueid, flags, instcount, numrefs, numrefs x valueid, n x (valueid)]
    PerModule = 1,

    /// `PERMODULE_PROFILE`
    ///
    /// [valueid, flags, instcount, numrefs, numrefs x valueid, n x (valueid, hotness+tailcall)]
    PerModuleProfile = 2,

    /// `PERMODULE_GLOBALVAR_INIT_REFS`
    ///
    /// [valueid, flags, n x valueid]
    PerModuleGlobalvarInitRefs = 3,

    /// `COMBINED`
    ///
    /// [valueid, modid, flags, instcount, numrefs, numrefs x valueid, n x (valueid)]
    Combined = 4,

    /// `COMBINED_PROFILE`
    ///
    /// [valueid, modid, flags, instcount, numrefs, numrefs x valueid, n x (valueid, hotness+tailcall)]
    CombinedProfile = 5,

    /// `COMBINED_GLOBALVAR_INIT_REFS`
    ///
    /// [valueid, modid, flags, n x valueid]
    CombinedGlobalvarInitRefs = 6,

    /// `ALIAS`
    ///
    /// [valueid, flags, valueid]
    Alias = 7,

    /// `COMBINED_ALIAS`
    ///
    /// [valueid, modid, flags, valueid]
    CombinedAlias = 8,

    /// `COMBINED_ORIGINAL_NAME`
    ///
    /// [original_name_hash]
    CombinedOriginalName = 9,

    /// `VERSION` of the summary, bumped when adding flags for instance.
    Version = 10,

    /// The list of `llvm.type.test` type identifiers used by the following function that are used
    /// other than by an `llvm.assume`.
    ///
    /// [n x typeid]
    TypeTests = 11,

    /// The list of virtual calls made by this function using `llvm.assume(llvm.type.test)` intrinsics
    /// that do not have all constant integer arguments.
    ///
    /// [n x (typeid, offset)]
    TypeTestAssumeVCalls = 12,

    /// The list of virtual calls made by this function using `llvm.type.checked.load` intrinsics
    /// that do not have all constant integer arguments.
    ///
    /// [n x (typeid, offset)]
    TypeCheckedLoadVCalls = 13,

    /// Identifies a virtual call made by this function using an `llvm.assume(llvm.type.test)`
    /// intrinsic with all constant integer arguments.
    ///
    /// [typeid, offset, n x arg]
    TypeTestAssumeConstVCall = 14,

    /// Identifies a virtual call made by this function using an `llvm.type.checked.load` intrinsic
    /// with all constant integer arguments.
    ///
    /// [typeid, offset, n x arg]
    TypeCheckedLoadConstVCall = 15,

    /// Assigns a GUID to a value ID. This normally appears only in combined summaries,

    /// but it can also appear in per-module summaries for PGO data.
    ///
    /// [valueid, guid]
    ValueGuid = 16,

    /// The list of local functions with CFI jump tables. Function names are strings in `strtab`.
    ///
    /// [n * name]
    CfiFunctionDefs = 17,

    /// The list of external functions with CFI jump tables. Function names are strings in `strtab`.
    ///
    /// [n * name]
    CfiFunctionDecls = 18,

    /// Per-module summary that also adds relative block frequency to callee info.
    ///
    /// `PERMODULE_RELBF`
    ///
    /// [valueid, flags, instcount, numrefs, numrefs x valueid, n x (valueid, relblockfreq+tailcall)]
    PerModuleRelBf = 19,

    /// Index-wide flags
    Flags = 20,

    /// Maps type identifier to summary information for that type identifier. Produced by the thin link
    /// (only lives in combined index).
    ///
    /// `TYPE_ID`
    ///
    /// [typeid, kind, bitwidth, align, size, bitmask, inlinebits, n x (typeid, kind, name, numrba, numrba x (numarg, numarg x arg, kind, info, byte, bit)]
    TypeId = 21,

    /// Maps type identifier to summary information for that type identifier computed from type metadata:
    /// the valueid of each vtable definition decorated with a type metadata for that identifier,

    /// and the offset from the corresponding type metadata.
    /// Exists in the per-module summary to provide information to thin link for index-based whole
    /// program devirtualization.
    ///
    /// `TYPE_ID_METADATA`
    ///
    /// [typeid, n x (valueid, offset)]
    TypeIdMetadata = 22,

    /// Summarizes vtable definition for use in index-based whole program devirtualization during the thin link.
    ///
    /// `PERMODULE_VTABLE_GLOBALVAR_INIT_REFS`
    ///
    /// [valueid, flags, varflags, numrefs, numrefs x valueid, n x (valueid, offset)]
    PerModuleVtableGlobalvarInitRefs = 23,

    /// The total number of basic blocks in the module.
    BlockCount = 24,

    /// Range information for accessed offsets for every argument.
    ///
    /// [n x (paramno, range, numcalls, numcalls x (callee_guid, paramno, range))]
    ParamAccess = 25,

    /// Summary of per-module memprof callsite metadata.
    ///
    /// [valueid, n x stackidindex]
    PerModuleCallsiteInfo = 26,

    /// Summary of per-module allocation memprof metadata.
    ///
    /// [nummib, nummib x (alloc type, context radix tree index), [nummib x (numcontext x total size)]?]
    PerModuleAllocInfo = 27,

    /// Summary of combined index memprof callsite metadata.
    ///
    /// [valueid, context radix tree index, numver, numver x version]
    CombinedCallsiteInfo = 28,

    /// Summary of combined index allocation memprof metadata.
    ///
    /// [nummib, numver, nummib x (alloc type, numstackids, numstackids x stackidindex), numver x version]
    CombinedAllocInfo = 29,

    /// List of all stack ids referenced by index in the callsite and alloc infos.
    ///
    /// [n x stack id]
    StackIds = 30,

    /// List of all full stack id pairs corresponding to the total sizes recorded at the end of the alloc info
    /// when reporting of hinted bytes is enabled.
    ///
    /// [nummib x (numcontext x full stack id)]
    AllocContextIds = 31,

    /// Linearized radix tree of allocation contexts.
    ///
    /// [n x entry]
    ContextRadixTreeArray = 32,
}

/// `METADATA` block codes
#[derive(Debug, Clone, Copy, TryFromPrimitive)]
#[repr(u8)]
#[non_exhaustive]
pub enum MetadataCode {
    /// `MDSTRING`
    ///
    /// [values]
    StringOld = 1,

    /// `VALUE`
    ///
    /// [type num, value num]
    Value = 2,

    /// `NODE`
    ///
    /// [n x md num]
    Node = 3,

    /// `STRING`
    ///
    /// [values]
    Name = 4,

    /// `DISTINCT_NODE`
    ///
    /// [n x md num]
    DistinctNode = 5,

    /// `KIND`
    ///
    /// [n x [id, name]]
    Kind = 6,

    /// `LOCATION`
    ///
    /// [distinct, line, col, scope, inlined-at?]
    Location = 7,

    /// `OLD_NODE`
    ///
    /// [n x (type num, value num)]
    OldNode = 8,

    /// `OLD_FN_NODE`
    ///
    /// [n x (type num, value num)]
    OldFnNode = 9,

    /// `NAMED_NODE`
    ///
    /// [n x mdnodes]
    NamedNode = 10,

    /// `ATTACHMENT`
    ///
    /// [m x [value, [n x [id, mdnode]]]
    Attachment = 11,

    /// `GENERIC_DEBUG`
    ///
    /// [distinct, tag, vers, header, n x md num]
    GenericDebug = 12,

    /// `SUBRANGE`
    ///
    /// [distinct, count, lo]
    Subrange = 13,

    /// `ENUMERATOR`
    ///
    /// [isUnsigned|distinct, value, name]
    Enumerator = 14,

    /// `BASIC_TYPE`
    ///
    /// [distinct, tag, name, size, align, enc]
    BasicType = 15,

    /// `FILE`
    ///
    /// [distinct, filename, directory, checksumkind, checksum]
    File = 16,

    /// `DERIVED_TYPE`
    ///
    /// [distinct, ...]
    DerivedType = 17,

    /// `COMPOSITE_TYPE`
    ///
    /// [distinct, ...]
    CompositeType = 18,

    /// `SUBROUTINE_TYPE`
    ///
    /// [distinct, flags, types, cc]
    SubroutineType = 19,

    /// `COMPILE_UNIT`
    ///
    /// [distinct, ...]
    CompileUnit = 20,

    /// `SUBPROGRAM`
    ///
    /// [distinct, ...]
    Subprogram = 21,

    /// `LEXICAL_BLOCK`
    ///
    /// [distinct, scope, file, line, column]
    LexicalBlock = 22,

    /// `LEXICAL_BLOCK_FILE`
    ///
    /// [distinct, scope, file, discriminator]
    LexicalBlockFile = 23,

    /// `NAMESPACE`
    ///
    /// [distinct, scope, file, name, line, exportSymbols]
    Namespace = 24,

    /// `TEMPLATE_TYPE`
    ///
    /// [distinct, scope, name, type, ...]
    TemplateType = 25,

    /// `TEMPLATE_VALUE`
    ///
    /// [distinct, scope, name, type, value, ...]
    TemplateValue = 26,

    /// `GLOBAL_VAR`
    ///
    /// [distinct, ...]
    GlobalVar = 27,

    /// `LOCAL_VAR`
    ///
    /// [distinct, ...]
    LocalVar = 28,

    /// `EXPRESSION`
    ///
    /// [distinct, n x element]
    Expression = 29,

    /// `OBJC_PROPERTY`
    ///
    /// [distinct, name, file, line, ...]
    ObjcProperty = 30,

    /// `IMPORTED_ENTITY`
    ///
    /// [distinct, tag, scope, entity, line, name]
    ImportedEntity = 31,

    /// `MODULE`
    ///
    /// [distinct, scope, name, ...]
    Module = 32,

    /// `MACRO`
    ///
    /// [distinct, macinfo, line, name, value]
    Macro = 33,

    /// `MACRO_FILE`
    ///
    /// [distinct, macinfo, line, file, ...]
    MacroFile = 34,

    /// `STRINGS`
    ///
    /// [count, offset] blob([lengths][chars])
    Strings = 35,

    /// `GLOBAL_DECL_ATTACHMENT`
    ///
    /// [valueid, n x [id, mdnode]]
    GlobalDeclAttachment = 36,

    /// `GLOBAL_VAR_EXPR`
    ///
    /// [distinct, var, expr]
    GlobalVarExpr = 37,

    /// `INDEX_OFFSET`
    ///
    /// [offset]
    IndexOffset = 38,

    /// `INDEX`
    ///
    /// [bitpos]
    Index = 39,

    /// `LABEL`
    ///
    /// [distinct, scope, name, file, line]
    Label = 40,

    /// `STRING_TYPE`
    ///
    /// [distinct, name, size, align, ..]
    StringType = 41,

    /// `COMMON_BLOCK`
    ///
    /// [distinct, scope, name, variable, ..]
    CommonBlock = 44,

    /// `GENERIC_SUBRANGE`
    ///
    /// [distinct, count, lo, up, stride]
    GenericSubrange = 45,

    /// `ARG_LIST`
    ///
    /// [n x [type num, value num]]
    ArgList = 46,

    /// `ASSIGN_ID`
    ///
    /// [distinct, ...]
    AssignId = 47,
}

/// `USELISTBLOCK` encoded values for a value's use-list.
#[derive(Debug, Clone, Copy, TryFromPrimitive)]
#[repr(u8)]
#[non_exhaustive]
pub enum UselistCode {
    /// `DEFAULT`
    ///
    /// [index..., value-id]
    Default = 1,

    /// `BB`
    ///
    /// [index..., bb-id]
    BB = 2,
}

/// Identification block contains a string that describes the producer details,
/// and an epoch that defines the auto-upgrade capability.
#[derive(Debug, Clone, Copy, TryFromPrimitive)]
#[repr(u8)]
#[non_exhaustive]
pub enum IdentificationCode {
    /// `IDENTIFICATION`
    ///
    /// [strchr x N]
    String = 1,

    /// `EPOCH`
    ///
    /// [epoch#]
    Epoch = 2,
}

/// `PARAMATTR` blocks have code for defining a parameter attribute set.
#[derive(Debug, Clone, Copy, TryFromPrimitive)]
#[repr(u8)]
#[non_exhaustive]
pub enum AttributeCode {
    /// `ENTRY`
    ///
    /// [paramidx0, attr0, paramidx1, attr1...]
    EntryOld = 1,

    /// `ENTRY`
    ///
    /// [attrgrp0, attrgrp1, ...]
    Entry = 2,

    /// `ENTRY`
    ///
    /// [grpid, idx, attr0, attr1, ...]
    GrpCodeEntry = 3,
}

/// Value symbol table codes.
#[derive(Debug, Clone, Copy, TryFromPrimitive)]
#[repr(u8)]
#[non_exhaustive]
pub enum ValueSymtabCode {
    /// `VST_ENTRY`
    ///
    /// [valueid, namechar x N]
    Entry = 1,

    /// `VST_BBENTRY`
    ///
    /// [bbid, namechar x N]
    BbEntry = 2,

    /// `VST_FNENTRY`
    ///
    /// Unused when strtab is present
    ///
    /// [valueid, offset, namechar x N]
    FnEntry = 3,

    /// Obsolete.
    ///
    /// `VST_COMBINED_ENTRY`
    ///
    /// [valueid, refguid]
    CombinedEntry = 5,
}

/// `TYPE` blocks have codes for each type primitive they use.
#[derive(Debug, Clone, Copy, TryFromPrimitive)]
#[repr(u8)]
#[non_exhaustive]
pub enum TypeCode {
    /// `NUMENTRY`
    ///
    /// [numentries]
    NumEntry = 1,

    /// `VOID`
    Void = 2,

    /// `FLOAT`
    Float = 3,

    /// `DOUBLE`
    Double = 4,

    /// `LABEL`
    Label = 5,

    /// `OPAQUE`
    Opaque = 6,

    /// `INTEGER`
    ///
    /// [width]
    Integer = 7,

    /// Typed pointers are obsolete.
    ///
    /// [pointee type]
    Pointer = 8,

    /// Obsolete
    ///
    /// [vararg, attrid, retty, paramty x N]
    FunctionOld = 9,

    /// `HALF`
    Half = 10,

    /// `ARRAY`
    ///
    /// [num_elements, elements_type]
    Array = 11,

    /// `VECTOR`
    ///
    /// [num_elements, elements_type]
    Vector = 12,

    /// `X86 LONG DOUBLE`
    X86Fp80 = 13,

    /// `LONG DOUBLE` (112 bit mantissa)
    Fp128 = 14,

    /// `PPC LONG DOUBLE` (2 doubles)
    PpcFp128 = 15,

    /// `METADATA`
    Metadata = 16,

    /// Unused
    ///
    /// `X86 MMX`
    X86Mmx = 17,

    /// `STRUCT_ANON`
    ///
    /// [ispacked, elements_type x N]
    StructAnon = 18,

    /// `STRUCT_NAME`
    ///
    /// [strchr x N]
    StructName = 19,

    /// `STRUCT_NAMED`
    ///
    /// [ispacked, elements_type x N]
    StructNamed = 20,

    /// `FUNCTION`
    ///
    /// [vararg, retty, paramty x N]
    Function = 21,

    /// `TOKEN`
    Token = 22,

    /// `BRAIN FLOATING POINT`
    BFloat = 23,

    /// `X86 AMX`
    X86Amx = 24,

    /// `OPAQUE_POINTER`
    ///
    /// [addrspace]
    OpaquePointer = 25,

    /// `TARGET_TYPE`
    TargetType = 26,
}

// The constants block (`CONSTANTS_BLOCK_ID` describes emission for each
// constant and maintains an implicit current type value.
#[derive(PartialEq, Debug, Clone, Copy, TryFromPrimitive)]
#[repr(u8)]
#[non_exhaustive]
pub enum ConstantsCodes {
    /// `SETTYPE`
    ///
    /// [typeid]
    Settype = 1,

    /// `NULL`
    Null = 2,

    /// `UNDEF`
    Undef = 3,

    /// `INTEGER`
    ///
    /// [intval]
    Integer = 4,

    /// `WIDE_INTEGER`
    ///
    /// [n x intval]
    WideInteger = 5,

    /// `FLOAT`
    ///
    /// [fpval]
    Float = 6,

    /// `AGGREGATE`
    ///
    /// [n x value number]
    Aggregate = 7,

    /// `STRING`
    ///
    /// [values]
    String = 8,

    /// `CSTRING`
    ///
    /// [values]
    CString = 9,

    /// `CE_BINOP`
    ///
    /// [opcode, opval, opval]
    BinOp = 10,

    /// `CE_CAST`
    ///
    /// [opcode, opty, opval]
    Cast = 11,

    /// Obsolete “constant expression” GEP record; replaced by `CST_CODE_CE_GEP`
    ///
    /// `CE_GEP`
    ///
    /// [n x operands]
    GepOld = 12,

    /// Unused
    ///
    /// `CE_SELECT`
    ///
    /// [opval, opval, opval]
    Select = 13,

    /// `CE_EXTRACTELT`
    ///
    /// [opty, opval, opval]
    ExtractElt = 14,

    /// `CE_INSERTELT`
    ///
    /// [opval, opval, opval]
    InsertElt = 15,

    /// `CE_SHUFFLEVEC`
    ///
    /// [opval, opval, opval]
    ShuffleVec = 16,

    /// Unused.
    ///
    /// `CE_CMP`
    ///
    /// [opty, opval, opval, pred]
    Cmp = 17,

    /// Obsolete inline asm record variant
    ///
    /// `INLINEASM`
    ///
    /// [sideeffect|alignstack, asmstr, onststr]
    InlineasmOld = 18,

    /// `SHUFVEC_EX`
    ///
    /// [opty, opval, opval, opval]
    ShufVecEx = 19,

    /// Obsolete.
    ///
    /// `INBOUNDS_GEP`
    ///
    /// [n x operands]
    InboundsGep = 20,

    /// `BLOCKADDRESS`
    ///
    /// [fnty, fnval, bb#]
    BlockAddress = 21,

    /// `DATA`
    ///
    /// [n x elements]
    Data = 22,

    /// Obsolete inline asm encoding variant
    ///
    /// `INLINEASM`
    ///
    /// [sideeffect|alignstack|asmdialect, smstr, onststr]
    InlineAsmOld2 = 23,

    /// [opty, flags, n x operands]
    GepWithInrangeIndexOld = 24,

    /// `CST_CODE_CE_UNOP`
    ///
    /// [opcode, opval]
    UnOp = 25,

    /// `POISON`
    Poison = 26,

    /// `DSO_LOCAL_EQUIVALENT`
    ///
    /// [gvty, gv]
    DsoLocalEquivalent = 27,

    /// Obsolete variant for inline asm
    ///
    /// `INLINEASM`
    ///
    /// [sideeffect|alignstack|asmdialect|unwind, asmstr, onststr]
    InlineAsmOld3 = 28,

    /// `NO_CFI`
    ///
    /// [fty, f]
    NoCfiValue = 29,

    /// `INLINEASM`
    ///
    /// [fnty, sideeffect|alignstack|asmdialect|unwind, asmstr, onststr]
    InlineAsm = 30,

    /// CST_CODE_CE_GEP_WITH_INRANGE
    /// [opty, flags, range, n x operands]
    GepWithInrange = 31,

    /// CST_CODE_CE_GEP
    /// [opty, flags, n x operands]
    Gep = 32,

    /// CST_CODE_PTRAUTH
    /// [ptr, key, disc, addrdisc]
    PtrAuth = 33,
}

// The function body block (`FUNCTION_BLOCK_ID`) describes function bodies. It
// can contain a constant block (`CONSTANTS_BLOCK_ID`).
#[derive(Debug, Copy, Clone, PartialEq, TryFromPrimitive)]
#[repr(u8)]
#[non_exhaustive]
pub enum FunctionCode {
    /// `DECLAREBLOCKS`
    ///
    /// [n]
    DeclareBlocks = 1,

    /// `BINOP`
    ///
    /// [opcode, ty, opval, opval]
    BinOp = 2,

    /// `CAST`
    ///
    /// [opcode, ty, opty, opval]
    Cast = 3,

    /// Old GEP instruction record; superseded by `FUNC_CODE_INST_GEP`
    ///
    /// `GEP`
    ///
    /// [n x operands]
    GepOld = 4,

    /// Unused.
    ///
    /// `SELECT`
    ///
    /// [ty, opval, opval, opval]
    SelectOld = 5,

    /// `EXTRACTELT`
    ///
    /// [opty, opval, opval]
    ExtractElt = 6,

    /// `INSERTELT`
    ///
    /// [ty, opval, opval, opval]
    InsertElt = 7,

    /// `SHUFFLEVEC`
    ///
    /// [ty, opval, opval, opval]
    ShuffleVec = 8,

    /// `CMP`
    ///
    /// [opty, opval, opval, pred]
    Cmp = 9,

    /// `RET`
    ///
    /// [opty, pval<both optional>]
    Ret = 10,

    /// `BR`
    ///
    /// [bb#, bb#, cond] or [bb#]
    Br = 11,

    /// `SWITCH`
    ///
    /// [opty, op0, op1, ...]
    Switch = 12,

    /// `INVOKE`
    ///
    /// [attr, fnty, op0, op1, ...]
    Invoke = 13,

    /// `UNREACHABLE`
    Unreachable = 15,

    /// `PHI`
    ///
    /// [ty, val0, b0, ...]
    Phi = 16,

    /// `ALLOCA`
    ///
    /// [instty, opty, op, align]
    Alloca = 19,

    /// `LOAD`
    ///
    /// [opty, op, align, vol]
    Load = 20,

    /// `VAARG`
    ///
    /// [valistty, valist, instty]
    VaArg = 23,

    // This store code encodes the pointer type, rather than the value type
    // this is so information only available in the pointer type (e.g. address
    // spaces) is retained.
    /// Obsolete store record; replaced by `FUNC_CODE_INST_STORE`
    ///
    /// `STORE`
    ///
    /// [ptrty, tr, al, align, vol]
    StoreOld = 24,

    /// `EXTRACTVAL`
    ///
    /// [n x operands]
    ExtractVal = 26,

    /// `INSERTVAL`
    ///
    /// [n x operands]
    InsertVal = 27,

    /// `CMP2`
    ///
    /// fcmp/icmp returning Int1TY or vector of Int1Ty. Same as `CMP`, exists to
    /// support legacy vicmp/vfcmp instructions.
    ///
    /// [opty, opval, opval, pred]
    Cmp2 = 28,

    /// `VSELECT`
    ///
    /// new select on i1 or [N x i1]
    ///
    /// [ty, pval, pval, redty, red]
    Vselect = 29,

    /// Obsolete inbounds GEP record; replaced by the newer `FUNC_CODE_INST_GEP`
    ///
    /// `INBOUNDS_GEP`
    ///
    /// [n x operands]
    InboundsGepOld = 30,

    /// `INDIRECTBR`
    ///
    /// [opty, op0, op1, ...]
    IndirectBr = 31,

    /// `DEBUG_LOC_AGAIN`
    DebugLocAgain = 33,

    /// `CALL`
    ///
    /// [attr, cc, fnty, fnid, args...]
    Call = 34,

    /// `DEBUG_LOC`
    ///
    /// [Line, ol, copeVal, IAVal]
    DebugLoc = 35,

    /// `FENCE`
    ///
    /// [ordering, synchscope]
    Fence = 36,

    /// Old cmpxchg record; replaced by `FUNC_CODE_INST_CMPXCHG`
    ///
    /// `CMPXCHG`
    ///
    /// [ptrty, ptr, cmp, val, vol, ordering, synchscope, failure_ordering?, weak?]
    CmpxchgOld = 37,

    /// Obsolete atomicrmw record; replaced by `FUNC_CODE_INST_ATOMICRMW`
    ///
    /// `ATOMICRMW`
    ///
    /// [ptrty, tr, al, operation, align, vol, ordering, synchscope]
    AtomicRmwOld = 38,

    /// `RESUME`
    ///
    /// [opval]
    Resume = 39,

    /// Obsolete landingpad record; replaced by `FUNC_CODE_INST_LANDINGPAD`
    ///
    /// `LANDINGPAD`
    ///
    /// [ty, al, al, um, d0, al0...]
    LandingPadOld = 40,

    /// `LOAD`
    ///
    /// [opty, op, align, vol, ordering, synchscope]
    LoadAtomic = 41,

    /// Obsolete store-atomic record; replaced by `FUNC_CODE_INST_STOREATOMIC`
    ///
    /// `STORE`
    ///
    /// [ptrty, tr, al, align, vol ordering, synchscope]
    StoreAtomicOld = 42,

    /// `GEP`
    ///
    /// [inbounds, n x operands]
    Gep = 43,

    /// `STORE`
    ///
    /// [ptrty, tr, alty, al, align, vol]
    Store = 44,

    /// `STORE`
    ///
    /// [ptrty, tr, al, align, vol]
    StoreAtomic = 45,

    /// `CMPXCHG`
    ///
    /// [ptrty, ptr, cmp, val, vol, success_ordering, synchscope, failure_ordering, weak]
    Cmpxchg = 46,

    /// `LANDINGPAD`
    ///
    /// [ty, al, um, d0, al0...]
    LandingPad = 47,

    /// `CLEANUPRET`
    ///
    /// [val] or [val, b#]
    CleanupRet = 48,

    /// `CATCHRET`
    ///
    /// [val, b#]
    CatchRet = 49,

    /// `CATCHPAD`
    ///
    /// [bb#, b#, um, rgs...]
    CatchPad = 50,

    /// `CLEANUPPAD`
    ///
    /// [num, rgs...]
    CleanupPad = 51,

    /// `CATCHSWITCH`
    ///
    /// [num, rgs...] or [num, rgs..., b]
    CatchSwitch = 52,

    /// `OPERAND_BUNDLE`
    ///
    /// [tag#, value...]
    OperandBundle = 55,

    /// `UNOP`
    ///
    /// [opcode, ty, opval]
    UnOp = 56,

    /// `CALLBR`
    ///
    /// [attr, cc, norm, transfs, fnty, fnid, args...]
    CallBr = 57,

    /// `FREEZE`
    ///
    /// [opty, opval]
    Freeze = 58,

    /// `ATOMICRMW`
    ///
    /// [ptrty, ptr, valty, val, operation, align, vol, ordering, synchscope]
    AtomicRmw = 59,

    /// `BLOCKADDR_USERS`
    ///
    /// [value...]
    BlockaddrUsers = 60,

    /// [DILocation, DILocalVariable, DIExpression, ValueAsMetadata]
    DebugRecordValue = 61,

    /// [DILocation, DILocalVariable, DIExpression, ValueAsMetadata]
    DebugRecordDeclare = 62,

    /// [DILocation, DILocalVariable, DIExpression, ValueAsMetadata, DIAssignID, DIExpression (addr), ValueAsMetadata (addr)]
    DebugRecordAssign = 63,

    /// [DILocation, DILocalVariable, DIExpression, Value]
    DebugRecordValueSimple = 64,

    /// [DILocation, DILabel]
    DebugRecordLabel = 65,
}

/// `MODULEPATH_SYMTAB` block codes
#[derive(Debug, Clone, Copy, TryFromPrimitive)]
#[repr(u8)]
#[non_exhaustive]
pub enum ModulePathSymtabCode {
    /// `MST_ENTRY`
    ///
    /// [modid, namechar x N]
    Entry = 1,

    /// `MST_HASH`
    ///
    /// [5*i32]
    Hash = 2,
}
