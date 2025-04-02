use num_enum::TryFromPrimitive;

#[derive(Debug, Copy, Clone, PartialEq, TryFromPrimitive)]
#[repr(u8)]
pub enum AttrKind {
    // = 0 is unused
    Alignment = 1,
    AlwaysInline = 2,
    ByVal = 3,
    InlineHint = 4,
    InReg = 5,
    MinSize = 6,
    Naked = 7,
    Nest = 8,
    NoAlias = 9,
    NoBuiltin = 10,
    NoCapture = 11,
    NoDuplicate = 12,
    NoImplicitFloat = 13,
    NoInline = 14,
    NonLazyBind = 15,
    NoRedZone = 16,
    NoReturn = 17,
    NoUnwind = 18,
    OptimizeForSize = 19,
    ReadNone = 20,
    ReadOnly = 21,
    Returned = 22,
    ReturnsTwice = 23,
    SExt = 24,
    StackAlignment = 25,
    StackProtect = 26,
    StackProtectReq = 27,
    StackProtectStrong = 28,
    StructRet = 29,
    SanitizeAddress = 30,
    SanitizeThread = 31,
    SanitizeMemory = 32,
    UwTable = 33,
    ZExt = 34,
    Builtin = 35,
    Cold = 36,
    OptimizeNone = 37,
    InAlloca = 38,
    NonNull = 39,
    JumpTable = 40,
    Dereferenceable = 41,
    DereferenceableOrNull = 42,
    Convergent = 43,
    Safestack = 44,
    /// Unused
    ArgMemOnly = 45,
    SwiftSelf = 46,
    SwiftError = 47,
    NoRecurse = 48,
    /// Unused
    InaccessibleMemOnly = 49,
    /// Unused
    InaccessiblememOrArgMemOnly = 50,
    AllocSize = 51,
    Writeonly = 52,
    Speculatable = 53,
    StrictFp = 54,
    SanitizeHwaddress = 55,
    NocfCheck = 56,
    OptForFuzzing = 57,
    Shadowcallstack = 58,
    SpeculativeLoadHardening = 59,
    Immarg = 60,
    Willreturn = 61,
    Nofree = 62,
    Nosync = 63,
    SanitizeMemtag = 64,
    Preallocated = 65,
    NoMerge = 66,
    NullPointerIsValid = 67,
    Noundef = 68,
    Byref = 69,
    Mustprogress = 70,
    NoCallback = 71,
    Hot = 72,
    NoProfile = 73,
    VscaleRange = 74,
    SwiftAsync = 75,
    NoSanitizeCoverage = 76,
    Elementtype = 77,
    DisableSanitizerInstrumentation = 78,
    NoSanitizeBounds = 79,
    AllocAlign = 80,
    AllocatedPointer = 81,
    AllocKind = 82,
    PresplitCoroutine = 83,
    FnretthunkExtern = 84,
    SkipProfile = 85,
    Memory = 86,
    Nofpclass = 87,
    OptimizeForDebugging = 88,
    Writable = 89,
    CoroOnlyDestroyWhenComplete = 90,
    DeadOnUnwind = 91,
    Range = 92,
    SanitizeNumericalStability = 93,
    Initializes = 94,
    HybridPatchable = 95,
}

/// CastOpcodes - These are values used in the bitcode files to encode which
/// cast a CST_CODE_CE_CAST or a XXX refers to.  The values of these enums
/// have no fixed relation to the LLVM IR enum values.  Changing these will
/// break compatibility with old files.
#[derive(Debug, TryFromPrimitive)]
#[repr(u8)]
pub enum CastOpcode {
    Trunc = 0,
    ZExt = 1,
    SExt = 2,
    FpToUi = 3,
    FpToSi = 4,
    UiToFp = 5,
    SiToFp = 6,
    FpTrunc = 7,
    FpExt = 8,
    PtrToInt = 9,
    IntToPtr = 10,
    Bitcast = 11,
    Addrspace = 12,
}

#[derive(Debug, TryFromPrimitive)]
#[repr(u8)]
pub enum Linkage {
    External = 0,
    Weak = 1,
    Appending = 2,
    Internal = 3,
    Linkonce = 4,
    Dllimport = 5,
    Dllexport = 6,
    ExternWeak = 7,
    Common = 8,
    Private = 9,
    WeakOdr = 10,
    LinkonceOdr = 11,
    AvailableExternally = 12,
    Deprecated1 = 13,
    Deprecated2 = 14,
}

#[derive(Debug, TryFromPrimitive)]
#[repr(u8)]
pub enum DllStorageClass {
    Default = 0,
    Import = 1,
    Export = 2,
}

#[derive(Debug, TryFromPrimitive)]
#[repr(u8)]
pub enum CallConv {
    C = 0,
    Fast = 8,
    Cold = 9,
    GHC = 10,
    HiPE = 11,
    AnyReg = 13,
    PreserveMost = 14,
    PreserveAll = 15,
    Swift = 16,
    /// CXX_FAST_TLS
    CxxFastTls = 17,
    Tail = 18,
    /// CFGuard_Check
    CFGuardCheck = 19,
    SwiftTail = 20,
    PreserveNone = 21,
    /// X86_StdCall (first target cc)
    X86StdCall = 64,
    /// X86_FastCall
    X86FastCall = 65,
    /// ARM_APCS
    ArmApcs = 66,
    /// ARM_AAPCS
    ArmAapcs = 67,
    /// ARM_AAPCS_VFP
    ArmAapcsVfp = 68,
    /// MSP430_INTR
    Msp430Intr = 69,
    /// X86_ThisCall
    X86ThisCall = 70,
    /// PTX_Kernel
    PTXKernel = 71,
    /// PTX_Device
    PTXDevice = 72,
    /// SPIR_FUNC
    SpirFunc = 75,
    /// SPIR_KERNEL
    SpirKernel = 76,
    /// Intel_OCL_BI
    IntelOclBi = 77,
    /// X86_64_SysV
    X8664SysV = 78,
    /// Win64
    Win64 = 79,
    /// X86_VectorCall
    X86VectorCall = 80,
    /// DUMMY_HHVM
    DummyHhvm = 81,
    /// DUMMY_HHVM_C
    DummyHhvmC = 82,
    /// X86_INTR
    X86Intr = 83,
    /// AVR_INTR
    AvrIntr = 84,
    /// AVR_SIGNAL
    AvrSignal = 85,
    /// AVR_BUILTIN
    AvrBuiltin = 86,
    /// AMDGPU_VS
    AmdGpuVs = 87,
    /// AMDGPU_GS
    AmdGpuGs = 88,
    /// AMDGPU_PS
    AmdGpuPs = 89,
    /// AMDGPU_CS
    AmdGpuCs = 90,
    /// AMDGPU_KERNEL
    AmdGpuKernel = 91,
    /// X86_RegCall
    X86RegCall = 92,
    /// AMDGPU_HS
    AmdGpuHs = 93,
    /// MSP430_BUILTIN
    Msp430Builtin = 94,
    /// AMDGPU_LS
    AmdGpuLs = 95,
    /// AMDGPU_ES
    AmdGpuEs = 96,
    /// AArch64_VectorCall
    AArch64VectorCall = 97,
    /// AArch64_SVE_VectorCall
    AArch64SVEVectorCall = 98,
    /// WASM_EmscriptenInvoke
    WasmEmscriptenInvoke = 99,
    /// AMDGPU_Gfx
    AmdGpuGfx = 100,
    /// M68k_INTR
    M68kIntr = 101,
    AArch64SmeAbiSupportRoutinesPreserveMostFromX0 = 102,
    AArch64SmeAbiSupportRoutinesPreserveMostFromX2 = 103,
    AmdGpuCSChain = 104,
    AmdGpuCSChainPreserve = 105,
    M68kRTD = 106,
    Graal = 107,
    Arm64ECThunkX64 = 108,
    Arm64ECThunkNative = 109,
    RiscVVectorCall = 110,
    AArch64SmeAbiSupportRoutinesPreserveMostFromX1 = 111,
}

/// call conv field in bitcode is often mixed with flags
impl CallConv {
    pub fn from_flags(ccinfo_flags: u64) -> Result<Self, String> {
        // static_cast<CallingConv::ID>((0x7ff & CCInfo) >> bitc::CALL_CCONV));
        let id = u8::try_from((ccinfo_flags & 0x7ff) >> 1).map_err(|e| e.to_string())?;
        Self::try_from_primitive(id).map_err(|e| e.to_string())
    }
}

/// BinaryOpcodes - These are values used in the bitcode files to encode which
/// binop a CST_CODE_CE_BINOP or a XXX refers to.  The values of these enums
/// have no fixed relation to the LLVM IR enum values.  Changing these will
/// break compatibility with old files.
#[derive(Debug, TryFromPrimitive)]
#[repr(u8)]
pub enum BinOpcode {
    Add = 0,
    Sub = 1,
    Mul = 2,
    Udiv = 3,
    Sdiv = 4, // overloaded for FP
    Urem = 5,
    Srem = 6, // overloaded for FP
    Shl = 7,
    Lshr = 8,
    Ashr = 9,
    And = 10,
    Or = 11,
    Xor = 12,
}

/// Encoded AtomicOrdering values.
#[derive(Debug, TryFromPrimitive, Default)]
#[repr(u8)]
pub enum AtomicOrdering {
    #[default]
    Notatomic = 0,
    Unordered = 1,
    Monotonic = 2,
    Acquire = 3,
    Release = 4,
    AcqRel = 5,
    SeqCst = 6,
}

/// COMDATSELECTIONKIND enumerates the possible selection mechanisms for
/// COMDAT sections.
#[derive(Debug, Clone, Copy, TryFromPrimitive)]
#[repr(u8)]
pub enum ComdatSelectionKind {
    Any = 1,
    ExactMatch = 2,
    Largest = 3,
    NoDuplicates = 4,
    SameSize = 5,
}

/// Atomic read-modify-write operations
#[derive(Debug, Clone, Copy, TryFromPrimitive)]
#[repr(u8)]
#[non_exhaustive]
pub enum RmwOperation {
    /// `XCHG`
    Xchg = 0,

    /// `ADD`
    Add = 1,

    /// `SUB`
    Sub = 2,

    /// `AND`
    And = 3,

    /// `NAND`
    Nand = 4,

    /// `OR`
    Or = 5,

    /// `XOR`
    Xor = 6,

    /// `MAX`
    Max = 7,

    /// `MIN`
    Min = 8,

    /// `UMAX`
    Umax = 9,

    /// `UMIN`
    Umin = 10,

    /// `FADD`
    Fadd = 11,

    /// `FSUB`
    Fsub = 12,

    /// `FMAX`
    Fmax = 13,

    /// `FMIN`
    Fmin = 14,

    /// `UINC_WRAP`
    UincWrap = 15,

    /// `UDEC_WRAP`
    UdecWrap = 16,

    /// `USUB_COND`
    UsSubCond = 17,

    /// `USUB_SAT`
    UsSubSat = 18,
}

/// Unary Opcodes
#[derive(Debug, Clone, Copy, TryFromPrimitive)]
#[repr(u8)]
#[non_exhaustive]
pub enum UnaryOpcode {
    /// `UNOP_FNEG`
    Fneg = 0,
}
