use num_enum::TryFromPrimitive;

const FIRST_APPLICATION_BLOCK_ID: u8 = 8;

/// LLVM Bitcode block IDs
#[derive(Debug, Clone, Copy, TryFromPrimitive)]
#[repr(u8)]
pub enum BlockId {
    Module = FIRST_APPLICATION_BLOCK_ID,
    // Module sub-block
    ParamAttr,
    ParamAttrGroup,

    Constants,
    Function,
    // Block intended to contains information on the bitcode versioning.
    // Can be used to provide better error messages when we fail to parse a
    // bitcode file.
    Identification,
    ValueSymbolTable,
    Metadata,
    MetadataAttachment,
    Type,
    UseList,
    ModuleStringTable,
    GlobalValSummary,
    OperandBundleTags,
    MetadataKind,
    StringTable,
    FullLtoGlobalValSummary,
    SymbolTable,
    SyncScopeNames,
}

/// MODULE blocks have a number of optional fields and subblocks.
#[derive(Debug, Clone, Copy, TryFromPrimitive)]
#[repr(u8)]
pub enum ModuleCode {
    Version = 1,     // [version#]
    Triple = 2,      // [strchr x N]
    DataLayout = 3,  // [strchr x N]
    Asm = 4,         // [strchr x N]
    SectionName = 5, // [strchr x N]
    /// Deprecated, but still needed to read old bitcode files.
    DepLib = 6, // [strchr x N]
    /// GLOBALVAR: [pointer type, isconst, initid,
    ///             linkage, alignment, section, visibility, threadlocal]
    GlobalVar = 7,
    /// FUNCTION:  [type, callingconv, isproto, linkage, paramattrs, alignment,
    ///             section, visibility, gc, unnamed_addr]
    Function = 8,
    /// ALIAS: `[alias type, aliasee val#, linkage, visibility]`
    AliasOld = 9,
    GcName = 11,    // [strchr x N]
    ComDat = 12,    // [selection_kind, name]
    VstOffset = 13, // [offset]
    /// ALIAS: `[alias value type, addrspace, aliasee val#, linkage, visibility]`
    Alias = 14,
    MetadataValuesUnused = 15,
    /// SOURCE_FILENAME: `[namechar x N]`
    SourceFileName = 16,
    /// HASH: `[5 * i32]`
    Hash = 17,
    /// IFUNC: `[ifunc value type, addrspace, resolver val#, linkage, visibility]`
    IFunc = 18,
}
