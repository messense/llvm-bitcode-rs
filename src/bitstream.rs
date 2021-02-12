use crate::bitcode::{BitcodeElement, Block, Record, Signature};
use crate::BitStreamReader;

/// An `Abbreviation` represents the encoding definition for a user-defined
/// record. An `Abbreviation` is the primary form of compression available in
/// a bitstream file.
#[derive(Debug, Clone)]
pub struct Abbreviation {
    pub operands: Vec<Operand>,
}

#[derive(Debug, Clone)]
pub enum Operand {
    /// A literal value (emitted as a VBR8 field)
    Literal(u64),
    /// A fixed-width field
    Fixed(u8),
    /// A VBR-encoded value with the provided chunk width
    Vbr(u8),
    /// An array of values. This expects another operand encoded
    /// directly after indicating the element type.
    /// The array will begin with a vbr6 value indicating the length of
    /// the following array.
    Array(Box<Operand>),
    /// A char6-encoded ASCII character
    Char6,
    /// Emitted as a vbr6 value, padded to a 32-bit boundary and then
    /// an array of 8-bit objects
    Blob,
}

impl Operand {
    /// Whether this case is payload
    pub fn is_payload(&self) -> bool {
        use Operand::*;

        match self {
            Array(_) | Blob => true,
            Literal(_) | Fixed(_) | Vbr(_) | Char6 => false,
        }
    }

    /// Whether this case is the `literal` case
    pub fn is_literal(&self) -> bool {
        matches!(self, Operand::Literal(_))
    }

    pub fn is_array(&self) -> bool {
        matches!(self, Operand::Array(_))
    }

    pub fn is_blob(&self) -> bool {
        matches!(self, Operand::Blob)
    }

    /// The llvm::BitCodeAbbrevOp::Encoding value this
    /// enum case represents.
    /// - note: Must match the encoding in
    ///         http://llvm.org/docs/BitCodeFormat.html#define-abbrev-encoding
    pub fn encoded_kind(&self) -> u8 {
        use Operand::*;

        match self {
            Literal(_) => 0,
            Fixed(_) => 1,
            Vbr(_) => 2,
            Array(_) => 3,
            Char6 => 4,
            Blob => 5,
        }
    }
}

/// A `BlockInfoCode` enumerates the bits that occur in the metadata for
/// a block or record. Of these bits, only `SetBid` is required. If
/// a name is given to a block or record with `BlockName` or
/// `SetRecordName`, debugging tools like `llvm-bcanalyzer` can be used to
/// introspect the structure of blocks and records in the bitstream file.
#[repr(u8)]
#[derive(Debug, Clone, Copy)]
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

/// A `BlockId` is a fixed-width field that occurs at the start of all blocks.
///
/// Bitstream reserves the first 7 block IDs for its own bookkeeping.
#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq)]
pub struct BlockId(u8);

impl BlockId {
    pub const BLOCK_INFO: Self = Self(0);
    pub const FIRST_APPLICATION_ID: Self = Self(8);

    #[inline]
    pub fn id(&self) -> u8 {
        self.0
    }
}

/// An `AbbreviationId` is a fixed-width field that occurs at the start of
/// abbreviated data records and inside block definitions.
///
/// Bitstream reserves 4 special abbreviation IDs for its own bookkeeping.
#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq)]
pub struct AbbreviationId(u64);

impl AbbreviationId {
    /// Marks the end of the current block.
    pub const END_BLOCK: Self = Self(0);
    /// Marks the beginning of a new block.
    pub const ENTER_SUB_BLOCK: Self = Self(1);
    /// Marks the definition of a new abbreviation.
    pub const DEFINE_ABBREVIATION: Self = Self(2);
    /// Marks the definition of a new unabbreviated record.
    pub const UNABBREVIATED_RECORD: Self = Self(3);
    /// The first application-defined abbreviation ID.
    pub const FIRST_APPLICATION_ID: Self = Self(4);

    #[inline]
    pub fn id(&self) -> u64 {
        self.0
    }
}

/// A visitor which receives callbacks while reading a bitstream.
pub trait BitStreamVisitor {
    /// Validate a bitstream's signature or "magic number".
    fn validate(&self, _signature: Signature) {}
    /// Called when a new block is encountered. Return `true` to enter the block
    /// and read its contents, or `false` to skip it.
    fn should_enter_block(&mut self, id: u64) -> bool;
    /// Called when a block is exited.
    fn did_exit_block(&mut self);
    /// Called whenever a record is encountered.
    fn visit(&mut self, record: Record);
}

pub struct CollectingVisitor {
    stack: Vec<(u64, Vec<BitcodeElement>)>,
}

/// A basic visitor that collects all the blocks and records in a stream.
impl CollectingVisitor {
    pub fn new() -> Self {
        Self {
            stack: vec![(BitStreamReader::TOP_LEVEL_BLOCK_ID, Vec::new())],
        }
    }

    pub fn finalize_top_level_elements(mut self) -> Vec<BitcodeElement> {
        assert_eq!(self.stack.len(), 1);
        self.stack.pop().unwrap().1
    }
}

impl BitStreamVisitor for CollectingVisitor {
    fn should_enter_block(&mut self, id: u64) -> bool {
        self.stack.push((id, Vec::new()));
        true
    }

    fn did_exit_block(&mut self) {
        if let Some((id, elements)) = self.stack.pop() {
            let block = Block { id, elements };
            let last = self.stack.last_mut().unwrap();
            last.1.push(BitcodeElement::Block(block));
        }
    }

    fn visit(&mut self, record: Record) {
        let last = self.stack.last_mut().unwrap();
        last.1.push(BitcodeElement::Record(record));
    }
}
