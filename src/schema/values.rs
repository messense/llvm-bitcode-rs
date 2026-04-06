//! From the LLVM Project, under the [Apache License v2.0 with LLVM Exceptions](https://llvm.org/LICENSE.txt)

use num_enum::TryFromPrimitive;

#[derive(Debug, Copy, Clone, PartialEq, TryFromPrimitive)]
#[repr(u8)]
pub enum AttrKind {
    /// Alignment of parameter (5 bits) stored as log2 of alignment with +1 bias.
    /// 0 means unaligned (different from align(1)).
    Alignment = 1,
    /// inline=always.
    AlwaysInline = 2,
    /// Pass structure by value.
    ByVal = 3,
    /// Source said inlining was desirable.
    InlineHint = 4,
    /// Force argument to be passed in register.
    InReg = 5,
    /// Function must be optimized for size first.
    MinSize = 6,
    /// Naked function.
    Naked = 7,
    /// Nested function static chain.
    Nest = 8,
    /// Considered to not alias after call.
    NoAlias = 9,
    /// Callee isn't recognized as a builtin.
    NoBuiltin = 10,
    NoCapture = 11,
    /// Call cannot be duplicated.
    NoDuplicate = 12,
    /// Disable implicit floating point insts.
    NoImplicitFloat = 13,
    /// inline=never.
    NoInline = 14,
    /// Function is called early and/or often, so lazy binding isn't worthwhile.
    NonLazyBind = 15,
    /// Disable redzone.
    NoRedZone = 16,
    /// Mark the function as not returning.
    NoReturn = 17,
    /// Function doesn't unwind stack.
    NoUnwind = 18,
    /// opt_size.
    OptimizeForSize = 19,
    /// Function does not access memory.
    ReadNone = 20,
    /// Function only reads from memory.
    ReadOnly = 21,
    /// Return value is always equal to this argument.
    Returned = 22,
    /// Function can return twice.
    ReturnsTwice = 23,
    /// Sign extended before/after call.
    SExt = 24,
    /// Alignment of stack for function (3 bits)  stored as log2 of alignment with
    /// +1 bias 0 means unaligned (different from alignstack=(1)).
    StackAlignment = 25,
    /// Stack protection.
    StackProtect = 26,
    /// Stack protection required.
    StackProtectReq = 27,
    /// Strong Stack protection.
    StackProtectStrong = 28,
    /// Hidden pointer to structure to return.
    StructRet = 29,
    /// AddressSanitizer is on.
    SanitizeAddress = 30,
    /// ThreadSanitizer is on.
    SanitizeThread = 31,
    /// MemorySanitizer is on.
    SanitizeMemory = 32,
    /// Function must be in a unwind table.
    UwTable = 33,
    /// Zero extended before/after call.
    ZExt = 34,
    /// Callee is recognized as a builtin, despite nobuiltin attribute on its
    /// declaration.
    Builtin = 35,
    /// Marks function as being in a cold path.
    Cold = 36,
    /// Function must not be optimized.
    OptimizeNone = 37,
    /// Pass structure in an alloca.
    InAlloca = 38,
    /// Pointer is known to be not null.
    NonNull = 39,
    /// Build jump-instruction tables and replace refs.
    JumpTable = 40,
    /// Pointer is known to be dereferenceable.
    Dereferenceable = 41,
    /// Pointer is either null or dereferenceable.
    DereferenceableOrNull = 42,
    /// Can only be moved to control-equivalent blocks.
    /// NB: Could be IntersectCustom with "or" handling.
    Convergent = 43,
    /// Safe Stack protection.
    Safestack = 44,
    /// Unused
    ArgMemOnly = 45,
    /// Argument is swift self/context.
    SwiftSelf = 46,
    /// Argument is swift error.
    SwiftError = 47,
    /// The function does not recurse.
    NoRecurse = 48,
    /// Unused
    InaccessibleMemOnly = 49,
    /// Unused
    InaccessibleMemOrArgMemOnly = 50,
    /// The result of the function is guaranteed to point to a number of bytes that
    /// we can determine if we know the value of the function's arguments.
    AllocSize = 51,
    /// Function only writes to memory.
    WriteOnly = 52,
    /// Function can be speculated.
    Speculatable = 53,
    /// Function was called in a scope requiring strict floating point semantics.
    StrictFp = 54,
    /// HWAddressSanitizer is on.
    SanitizeHwAddress = 55,
    /// Disable Indirect Branch Tracking.
    NoCfCheck = 56,
    /// Select optimizations for best fuzzing signal.
    OptForFuzzing = 57,
    /// Shadow Call Stack protection.
    ShadowCallStack = 58,
    /// Speculative Load Hardening is enabled.
    ///
    /// Note that this uses the default compatibility (always compatible during
    /// inlining) and a conservative merge strategy where inlining an attributed
    /// body will add the attribute to the caller. This ensures that code carrying
    /// this attribute will always be lowered with hardening enabled.
    SpeculativeLoadHardening = 59,
    /// Parameter is required to be a trivial constant.
    ImmArg = 60,
    /// Function always comes back to callsite.
    WillReturn = 61,
    /// Function does not deallocate memory.
    Nofree = 62,
    /// Function does not synchronize.
    Nosync = 63,
    /// MemTagSanitizer is on.
    SanitizeMemtag = 64,
    /// Similar to byval but without a copy.
    Preallocated = 65,
    /// Disable merging for specified functions or call sites.
    NoMerge = 66,
    /// Null pointer in address space zero is valid.
    NullPointerIsValid = 67,
    /// Parameter or return value may not contain uninitialized or poison bits.
    NoUndef = 68,
    /// Mark in-memory ABI type.
    ByRef = 69,
    /// Function is required to make Forward Progress.
    MustProgress = 70,
    /// Function cannot enter into caller's translation unit.
    NoCallback = 71,
    /// Marks function as being in a hot path and frequently called.
    Hot = 72,
    /// Function should not be instrumented.
    NoProfile = 73,
    /// Minimum/Maximum vscale value for function.
    VscaleRange = 74,
    /// Argument is swift async context.
    SwiftAsync = 75,
    /// No SanitizeCoverage instrumentation.
    NoSanitizeCoverage = 76,
    /// Provide pointer element type to intrinsic.
    Elementtype = 77,
    /// Do not instrument function with sanitizers.
    DisableSanitizerInstrumentation = 78,
    /// No SanitizeBounds instrumentation.
    NoSanitizeBounds = 79,
    /// Parameter of a function that tells us the alignment of an allocation, as in
    /// aligned_alloc and aligned ::operator::new.
    AllocAlign = 80,
    /// Parameter is the pointer to be manipulated by the allocator function.
    AllocatedPointer = 81,
    /// Describes behavior of an allocator function in terms of known properties.
    AllocKind = 82,
    /// Function is a presplit coroutine.
    PresplitCoroutine = 83,
    /// Whether to keep return instructions, or replace with a jump to an external
    /// symbol.
    FnRetThunkExtern = 84,
    SkipProfile = 85,
    /// Memory effects of the function.
    Memory = 86,
    /// Forbidden floating-point classes.
    NoFpClass = 87,
    /// Select optimizations that give decent debug info.
    OptimizeForDebugging = 88,
    /// Pointer argument is writable.
    Writable = 89,
    CoroOnlyDestroyWhenComplete = 90,
    /// Argument is dead if the call unwinds.
    DeadOnUnwind = 91,
    /// Parameter or return value is within the specified range.
    Range = 92,
    /// NumericalStabilitySanitizer is on.
    SanitizeNumericalStability = 93,
    /// Pointer argument memory is initialized.
    Initializes = 94,
    /// Function has a hybrid patchable thunk.
    HybridPatchable = 95,
    /// RealtimeSanitizer is on.
    SanitizeRealtime = 96,
    /// RealtimeSanitizer should error if a real-time unsafe function is invoked
    /// during a real-time sanitized function (see `sanitize_realtime`).
    SanitizeRealtimeBlocking = 97,
    /// The coroutine call meets the elide requirement. Hint the optimization
    /// pipeline to perform elide on the call or invoke instruction.
    CoroElideSafe = 98,
    /// No extension needed before/after call (high bits are undefined).
    NoExt = 99,
    /// Function is not a source of divergence.
    NoDivergenceSource = 100,
    /// TypeSanitizer is on.
    SanitizeType = 101,
    /// Specify how the pointer may be captured.
    Captures = 102,
    /// Argument is dead upon function return.
    DeadOnReturn = 103,
    /// Allocation token instrumentation is on.
    SanitizeAllocToken = 104,
    /// Result will not be undef or poison if all arguments are not undef and not
    /// poison.
    NoCreateUndefOrPoison = 105,
    /// Indicate the denormal handling of the default floating-point
    /// environment.
    DenormalFpEnv = 106,
    NoOutline = 107,
}

/// These are values used in the bitcode files to encode which
/// cast a `CST_CODE_CE_CAST` refers to.
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

/// These are bitcode-specific values, different from C++ enum
#[derive(Debug, TryFromPrimitive)]
#[repr(u8)]
#[non_exhaustive]
pub enum Linkage {
    /// Externally visible function
    External = 0,
    /// Keep one copy of named function when linking (weak)
    /// Old value with implicit comdat.
    #[deprecated]
    WeakAnyOld = 1,
    /// Special purpose, only applies to global arrays
    Appending = 2,
    /// Rename collisions when linking (static functions).
    Internal = 3,
    /// Keep one copy of function when linking (inline)
    /// Old value with implicit comdat.
    #[deprecated]
    LinkOnceAnyOld = 4,
    /// Externally visible function
    /// Obsolete DLLImportLinkage
    #[deprecated]
    DllImport = 5,
    /// Externally visible function
    /// Obsolete DLLExportLinkage
    #[deprecated]
    DllExport = 6,
    /// ExternalWeak linkage
    ExternWeak = 7,
    /// Tentative definitions.
    Common = 8,
    /// Like Internal, but omit from symbol table.
    Private = 9,
    /// Same, but only replaced by something equivalent.
    /// Old value with implicit comdat.
    #[deprecated]
    WeakOdrOld = 10,
    /// Same, but only replaced by something equivalent.
    /// Old value with implicit comdat.
    #[deprecated]
    LinkOnceOdrOld = 11,
    /// Available for inspection, not emission.
    AvailableExternally = 12,
    /// Like Internal, but omit from symbol table.
    /// Obsolete LinkerPrivateLinkage
    #[deprecated]
    LinkerPrivate = 13,
    /// Like Internal, but omit from symbol table.
    /// Obsolete LinkerPrivateWeakLinkage
    #[deprecated]
    LinkerPrivateWeak = 14,
    /// Externally visible function
    /// Obsolete LinkOnceODRAutoHideLinkage
    #[deprecated]
    LinkOnceOdrAutoHide = 15,
    /// Keep one copy of named function when linking (weak)
    WeakAny = 16,
    /// Same, but only replaced by something equivalent.
    WeakOdr = 17,
    /// Keep one copy of function when linking (inline)
    LinkOnceAny = 18,
    /// Same, but only replaced by something equivalent.
    LinkOnceOdr = 19,
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
#[non_exhaustive]
pub enum CallConv {
    /// The default llvm calling convention, compatible with C. This convention
    /// is the only one that supports varargs calls. As with typical C calling
    /// conventions, the callee/caller have to tolerate certain amounts of
    /// prototype mismatch.
    C = 0,
    /// Attempts to make calls as fast as possible (e.g. by passing things in
    /// registers).
    Fast = 8,
    /// Attempts to make code in the caller as efficient as possible under the
    /// assumption that the call is not commonly executed. As such, these calls
    /// often preserve all registers so that the call does not break any live
    /// ranges in the caller side.
    Cold = 9,
    /// Used by the Glasgow Haskell Compiler (GHC).
    Ghc = 10,
    /// Used by the High-Performance Erlang Compiler (HiPE).
    HiPE = 11,
    /// Used for dynamic register based calls (e.g. stackmap and patchpoint
    /// intrinsics).
    AnyReg = 13,
    /// Used for runtime calls that preserves most registers.
    PreserveMost = 14,
    /// Used for runtime calls that preserves (almost) all registers.
    PreserveAll = 15,
    /// Calling convention for Swift.
    Swift = 16,
    /// Used for access functions.
    CxxFastTls = 17,
    /// Attemps to make calls as fast as possible while guaranteeing that tail
    /// call optimization can always be performed.
    Tail = 18,
    /// Special calling convention on Windows for calling the Control Guard
    /// Check ICall funtion. The function takes exactly one argument (address of
    /// the target function) passed in the first argument register, and has no
    /// return value. All register values are preserved.
    CfGuardCheck = 19,
    /// This follows the Swift calling convention in how arguments are passed
    /// but guarantees tail calls will be made by making the callee clean up
    /// their stack.
    SwiftTail = 20,
    /// Used for runtime calls that preserves none general registers.
    PreserveNone = 21,
    /// stdcall is mostly used by the Win32 API. It is basically the same as the
    /// C convention with the difference in that the callee is responsible for
    /// popping the arguments from the stack.
    X86StdCall = 64,
    /// 'fast' analog of X86_StdCall. Passes first two arguments in ECX:EDX
    /// registers, others - via stack. Callee is responsible for stack cleaning.
    X86FastCall = 65,
    /// ARM Procedure Calling Standard (obsolete, but still used on some
    /// targets).
    ArmApcs = 66,
    /// ARM Architecture Procedure Calling Standard calling convention (aka
    /// EABI). Soft float variant.
    ArmAapcs = 67,
    /// Same as ARM_AAPCS, but uses hard floating point ABI.
    ArmAapcsVfp = 68,
    /// Used for MSP430 interrupt routines.
    Msp430Intr = 69,
    /// Similar to X86_StdCall. Passes first argument in ECX, others via stack.
    /// Callee is responsible for stack cleaning. MSVC uses this by default for
    /// methods in its ABI.
    X86ThisCall = 70,
    /// Call to a PTX kernel. Passes all arguments in parameter space.
    PtxKernel = 71,
    /// Call to a PTX device function. Passes all arguments in register or
    /// parameter space.
    PtxDevice = 72,
    /// Used for SPIR non-kernel device functions. No lowering or expansion of
    /// arguments. Structures are passed as a pointer to a struct with the
    /// byval attribute. Functions can only call SPIR_FUNC and SPIR_KERNEL
    /// functions. Functions can only have zero or one return values. Variable
    /// arguments are not allowed, except for printf. How arguments/return
    /// values are lowered are not specified. Functions are only visible to the
    /// devices.
    SpirFunc = 75,
    /// Used for SPIR kernel functions. Inherits the restrictions of SPIR_FUNC,
    /// except it cannot have non-void return values, it cannot have variable
    /// arguments, it can also be called by the host or it is externally
    /// visible.
    SpirKernel = 76,
    /// Used for Intel OpenCL built-ins.
    IntelOclBi = 77,
    /// The C convention as specified in the x86-64 supplement to the System V
    /// ABI, used on most non-Windows systems.
    X8664SysV = 78,
    /// The C convention as implemented on Windows/x86-64 and AArch64. It
    /// differs from the more common \c X86_64_SysV convention in a number of
    /// ways, most notably in that XMM registers used to pass arguments are
    /// shadowed by GPRs, and vice versa. On AArch64, this is identical to the
    /// normal C (AAPCS) calling convention for normal functions, but floats are
    /// passed in integer registers to variadic functions.
    Win64 = 79,
    /// MSVC calling convention that passes vectors and vector aggregates in SSE
    /// registers.
    X86VectorCall = 80,
    /// Placeholders for HHVM calling conventions (deprecated, removed).
    #[deprecated]
    DummyHhvm = 81,
    DummyHhvmC = 82,
    /// x86 hardware interrupt context. Callee may take one or two parameters,
    /// where the 1st represents a pointer to hardware context frame and the 2nd
    /// represents hardware error code, the presence of the later depends on the
    /// interrupt vector taken. Valid for both 32- and 64-bit subtargets.
    X86Intr = 83,
    /// Used for AVR interrupt routines.
    AvrIntr = 84,
    /// Used for AVR signal routines.
    AvrSignal = 85,
    /// Used for special AVR rtlib functions which have an "optimized"
    /// convention to preserve registers.
    AvrBuiltin = 86,
    /// Used for Mesa vertex shaders, or AMDPAL last shader stage before
    /// rasterization (vertex shader if tessellation and geometry are not in
    /// use, or otherwise copy shader if one is needed).
    AmdGpuVs = 87,
    /// Used for Mesa/AMDPAL geometry shaders.
    AmdGpuGs = 88,
    /// Used for Mesa/AMDPAL pixel shaders.
    AmdGpuPs = 89,
    /// Used for Mesa/AMDPAL compute shaders.
    AmdGpuCs = 90,
    /// Used for AMDGPU code object kernels.
    AmdGpuKernel = 91,
    /// Register calling convention used for parameters transfer optimization
    X86RegCall = 92,
    /// Used for Mesa/AMDPAL hull shaders (= tessellation control shaders).
    AmdGpuHs = 93,
    /// Used for special MSP430 rtlib functions which have an "optimized"
    /// convention using additional registers.
    Msp430Builtin = 94,
    /// Used for AMDPAL vertex shader if tessellation is in use.
    AmdGpuLs = 95,
    /// Used for AMDPAL shader stage before geometry shader if geometry is in
    /// use. So either the domain (= tessellation evaluation) shader if
    /// tessellation is in use, or otherwise the vertex shader.
    AmdGpuEs = 96,
    /// Used between AArch64 Advanced SIMD functions
    AArch64VectorCall = 97,
    /// Used between AArch64 SVE functions
    AArch64SveVectorCall = 98,
    /// For emscripten __invoke_* functions. The first argument is required to
    /// be the function ptr being indirectly called. The remainder matches the
    /// regular calling convention.
    WasmEmscriptenInvoke = 99,
    /// Used for AMD graphics targets.
    AmdGpuGfx = 100,
    /// Used for M68k interrupt routines.
    M68kIntr = 101,
    /// Preserve X0-X13, X19-X29, SP, Z0-Z31, P0-P15.
    AArch64SmeAbiSupportRoutinesPreserveMostFromX0 = 102,
    /// Preserve X2-X15, X19-X29, SP, Z0-Z31, P0-P15.
    AArch64SmeAbiSupportRoutinesPreserveMostFromX2 = 103,
    /// Used on AMDGPUs to give the middle-end more control over argument
    /// placement.
    AmdGpuCsChain = 104,
    /// Used on AMDGPUs to give the middle-end more control over argument
    /// placement. Preserves active lane values for input VGPRs.
    AmdGpuCsChainPreserve = 105,
    /// Used for M68k rtd-based CC (similar to X86's stdcall).
    M68kRtd = 106,
    /// Used by GraalVM. Two additional registers are reserved.
    Graal = 107,
    /// Calling convention used in the ARM64EC ABI to implement calls between
    /// x64 code and thunks. This is basically the x64 calling convention using
    /// ARM64 register names. The first parameter is mapped to x9.
    Arm64ecThunkX64 = 108,
    /// Calling convention used in the ARM64EC ABI to implement calls between
    /// ARM64 code and thunks. This is just the ARM64 calling convention,
    /// except that the first parameter is mapped to x9.
    Arm64ecThunkNative = 109,
    /// Calling convention used for RISC-V V-extension.
    RiscVVectorCall = 110,
    /// Preserve X1-X15, X19-X29, SP, Z0-Z31, P0-P15.
    AArch64SmeAbiSupportRoutinesPreserveMostFromX1 = 111,
    /// Calling convention used for RISC-V V-extension fixed vectors.
    RiscVVlsCall32 = 112,
    RiscVVlsCall64 = 113,
    RiscVVlsCall128 = 114,
    RiscVVlsCall256 = 115,
    RiscVVlsCall512 = 116,
    RiscVVlsCall1024 = 117,
    RiscVVlsCall2048 = 118,
    RiscVVlsCall4096 = 119,
    RiscVVlsCall8192 = 120,
    RiscVVlsCall16384 = 121,
    RiscVVlsCall32768 = 122,
    RiscVVlsCall65536 = 123,
    AmdGpuGfxWholeWave = 124,
    /// Calling convention used for CHERIoT when crossing a protection boundary.
    CHERIoTCompartmentCall = 125,
    /// Calling convention used for the callee of CHERIoT_CompartmentCall.
    /// Ignores the first two capability arguments and the first integer
    /// argument, zeroes all unused return registers on return.
    CHERIoTCompartmentCallee = 126,
    /// Calling convention used for CHERIoT for cross-library calls to a
    /// stateless compartment.
    CHERIoTLibraryCall = 127,
}

/// call conv field in bitcode is often mixed with flags
impl CallConv {
    #[doc(hidden)]
    #[deprecated]
    pub fn from_flags(ccinfo_flags: u64) -> Result<Self, String> {
        Self::from_call_flags(ccinfo_flags).ok_or_else(|| "out of range".into())
    }

    /// Extract calling convention from CALL/CALLBR CCInfo flags.
    #[must_use]
    pub fn from_call_flags(ccinfo_flags: u64) -> Option<Self> {
        // static_cast<CallingConv::ID>((0x7ff & CCInfo) >> bitc::CALL_CCONV));
        let id = u8::try_from((ccinfo_flags & 0x7ff) >> 1).ok()?;
        Self::try_from_primitive(id).ok()
    }

    /// Extract calling convention from INVOKE CCInfo flags.
    #[must_use]
    pub fn from_invoke_flags(ccinfo_flags: u64) -> Option<Self> {
        let id = u8::try_from(ccinfo_flags & 0x3ff).ok()?;
        Self::try_from_primitive(id).ok()
    }
}

/// These are values used in the bitcode files to encode which
/// binop a `CST_CODE_CE_BINOP` refers to.
#[derive(Debug, TryFromPrimitive)]
#[repr(u8)]
pub enum BinOpcode {
    Add = 0,
    Sub = 1,
    Mul = 2,
    UDiv = 3,
    /// overloaded for FP
    SDiv = 4,
    URem = 5,
    /// overloaded for FP
    SRem = 6,
    Shl = 7,
    LShr = 8,
    AShr = 9,
    And = 10,
    Or = 11,
    Xor = 12,
}

/// Encoded `AtomicOrdering` values.
#[derive(Debug, TryFromPrimitive, Default)]
#[repr(u8)]
pub enum AtomicOrdering {
    #[default]
    NotAtomic = 0,
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
    UMax = 9,

    /// `UMIN`
    UMin = 10,

    /// `FADD`
    FAdd = 11,

    /// `FSUB`
    FSub = 12,

    /// `FMAX`
    FMax = 13,

    /// `FMIN`
    FMin = 14,

    /// `UINC_WRAP`
    UIncWrap = 15,

    /// `UDEC_WRAP`
    UDecWrap = 16,

    /// `USUB_COND`
    USubCond = 17,

    /// `USUB_SAT`
    USubSat = 18,
}

/// Unary Opcodes
#[derive(Debug, Clone, Copy, TryFromPrimitive)]
#[repr(u8)]
#[non_exhaustive]
pub enum UnaryOpcode {
    /// `UNOP_FNEG`
    Fneg = 0,
}

/// Flags for serializing
/// OverflowingBinaryOperator's SubclassOptionalData contents.
#[derive(Debug, Clone, Copy, TryFromPrimitive)]
#[repr(u8)]
#[non_exhaustive]
pub enum OverflowingBinaryOperatorOptionalFlags {
    NoUnsignedWrap = 0,
    NoSignedWrap = 1,
}

/// Flags for serializing
/// TruncInstOptionalFlags's SubclassOptionalData contents.
#[derive(Debug, Clone, Copy, TryFromPrimitive)]
#[repr(u8)]
#[non_exhaustive]
pub enum TruncInstOptionalFlags {
    NoUnsignedWrap = 0,
    NoSignedWrap = 1,
}

/// FastMath Flags
/// This is a fixed layout derived from the bitcode emitted by LLVM 5.0
/// intended to decouple the in-memory representation from the serialization.
#[derive(Debug, Clone, Copy, TryFromPrimitive)]
#[repr(u8)]
pub enum FastMathMap {
    UnsafeAlgebra = 1 << 0, // Legacy
    NoNaNs = 1 << 1,
    NoInfs = 1 << 2,
    NoSignedZeros = 1 << 3,
    AllowReciprocal = 1 << 4,
    AllowContract = 1 << 5,
    ApproxFunc = 1 << 6,
    AllowReassoc = 1 << 7,
}

bitflags::bitflags! {
    /// `GetElementPtrOptionalFlags`
    #[derive(Debug, Copy, Clone, Default)]
    pub struct GEPFlags: u8 {
        /// GEP_INBOUNDS = Index is guaranteed within bounds (enables optimizations)
        const Inbounds = 1 << 0;
        /// GEP_NUSW = No unsigned/signed wrap
        const Nusw = 1 << 1;
        /// GEP_NUW = No unsigned wrap
        const Nuw = 1 << 2;
    }
}

bitflags::bitflags! {
    /// Markers and flags for call instruction
    #[derive(Debug, Copy, Clone, Default)]
    pub struct CallMarkersFlags: u32 {
        const Tail = 1 << 0;
        const Cconv = 1 << 1;
        const MustTail = 1 << 14;
        const ExplicitType = 1 << 15;
        const NoTail = 1 << 16;
        /// Call has optional fast-math-flags
        const Fmf = 1 << 17;
    }
}
