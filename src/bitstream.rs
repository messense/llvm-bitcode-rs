use num_enum::TryFromPrimitive;

/// An `Abbreviation` represents the encoding definition for a user-defined
/// record. An `Abbreviation` is the primary form of compression available in
/// a bitstream file.
#[derive(Debug, Clone)]
pub struct Abbreviation {
    /// All operands except the last that may be an array/string/blob
    pub fields: Vec<ScalarOperand>,
    pub payload: Option<PayloadOperand>,
    pub id: u32,
}

/// Abbreviation operand
#[derive(Debug, Copy, Clone)]
pub enum ScalarOperand {
    /// A literal value (emitted as a VBR8 field)
    Literal(u64),
    /// A fixed-width field
    Fixed(u8),
    /// A VBR-encoded value with the provided chunk width
    Vbr(u8),
    /// A char6-encoded ASCII character
    Char6,
}

/// Abbreviation operand
#[derive(Debug, Copy, Clone)]
pub enum PayloadOperand {
    /// Emitted as a vbr6 value, padded to a 32-bit boundary and then
    /// an array of 8-bit objects
    Blob,
    /// An array of values. This expects another operand encoded
    /// directly after indicating the element type.
    /// The array will begin with a vbr6 value indicating the length of
    /// the following array.
    Array(ScalarOperand),
}

#[derive(Debug, Copy, Clone)]
pub enum Operand {
    Scalar(ScalarOperand),
    Payload(PayloadOperand),
}

impl Operand {
    /// Whether this case is payload
    #[must_use]
    pub fn is_payload(&self) -> bool {
        matches!(self, Self::Payload(_))
    }

    /// Whether this case is the `literal` case
    #[must_use]
    pub fn is_literal(&self) -> bool {
        matches!(self, Self::Scalar(ScalarOperand::Literal(_)))
    }

    #[must_use]
    pub fn is_array(&self) -> bool {
        matches!(self, Self::Payload(PayloadOperand::Array(_)))
    }

    #[must_use]
    pub fn is_blob(&self) -> bool {
        matches!(self, Self::Payload(PayloadOperand::Blob))
    }

    /// The `llvm::BitCodeAbbrevOp::Encoding` value this
    /// enum case represents.
    /// - note: Must match the encoding in
    ///         <http://llvm.org/docs/BitCodeFormat.html#define-abbrev-encoding>
    #[must_use]
    pub fn encoded_kind(&self) -> u8 {
        match self {
            Self::Scalar(ScalarOperand::Literal(_)) => 0,
            Self::Scalar(ScalarOperand::Fixed(_)) => 1,
            Self::Scalar(ScalarOperand::Vbr(_)) => 2,
            Self::Payload(PayloadOperand::Array(_)) => 3,
            Self::Scalar(ScalarOperand::Char6) => 4,
            Self::Payload(PayloadOperand::Blob) => 5,
        }
    }
}

/// A `BlockInfoCode` enumerates the bits that occur in the metadata for
/// a block or record. Of these bits, only `SetBid` is required. If
/// a name is given to a block or record with `BlockName` or
/// `SetRecordName`, debugging tools like `llvm-bcanalyzer` can be used to
/// introspect the structure of blocks and records in the bitstream file.
#[derive(Debug, Clone, Copy, TryFromPrimitive)]
#[repr(u8)]
pub enum BlockInfoCode {
    /// Indicates which block ID is being described.
    SetBid = 1,
    /// An optional element that records which bytes of the record are the
    /// name of the block.
    BlockName = 2,
    /// An optional element that records the record ID number and the bytes
    /// for the name of the corresponding record.
    SetRecordName = 3,
}

/// An abbreviation id is a fixed-width field that occurs at the start of
/// abbreviated data records and inside block definitions.
///
/// Bitstream reserves 4 special abbreviation IDs for its own bookkeeping.
#[derive(Debug, Clone, Copy, TryFromPrimitive)]
#[repr(u32)]
pub enum BuiltinAbbreviationId {
    /// Marks the end of the current block.
    EndBlock = 0,
    /// Marks the beginning of a new block.
    EnterSubBlock = 1,
    /// Marks the definition of a new abbreviation.
    DefineAbbreviation = 2,
    /// Marks the definition of a new unabbreviated record.
    UnabbreviatedRecord = 3,
}
