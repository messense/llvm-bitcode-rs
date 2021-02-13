use std::collections::HashMap;

use crate::bits::Bits;
use crate::read::{BitStreamReader, Error};
use crate::visitor::{BitStreamVisitor, CollectingVisitor};

const LLVM_BITCODE_WRAPPER_MAGIC: u32 = 0x0B17C0DE;

/// Represents the contents of a file encoded using the
/// [LLVM bitstream container format](https://llvm.org/docs/BitCodeFormat.html#bitstream-container-format)
#[derive(Debug, Clone)]
pub struct Bitcode {
    pub signature: Signature,
    pub elements: Vec<BitcodeElement>,
    pub block_info: HashMap<u64, BlockInfo>,
}

/// Blocks in a bitstream denote nested regions of the stream,
/// and are identified by a content-specific id number
///
/// Block IDs 0-7 are reserved for [standard blocks](https://llvm.org/docs/BitCodeFormat.html#standard-blocks)
/// whose meaning is defined by Bitcode;
/// block IDs 8 and greater are application specific.
#[derive(Debug, Clone)]
pub struct Block {
    /// Block ID
    pub id: u64,
    /// Block elements
    pub elements: Vec<BitcodeElement>,
}

#[derive(Debug, Clone)]
pub enum Payload {
    Array(Vec<u64>),
    Char6String(String),
    Blob(Vec<u8>),
}

/// Data records consist of a record code and a number of (up to) 64-bit integer values
///
/// The interpretation of the code and values is application specific and may vary between different block types.
#[derive(Debug, Clone)]
pub struct Record {
    /// Record code
    pub id: u64,
    /// An abbreviated record has a abbreviation id followed by a set of fields
    pub fields: Vec<u64>,
    /// Array and Blob encoding has payload
    pub payload: Option<Payload>,
}

/// Bitcode element
#[derive(Debug, Clone)]
pub enum BitcodeElement {
    /// Block
    Block(Block),
    /// Data record
    Record(Record),
}

impl BitcodeElement {
    /// Returns true if it is a `Block`
    pub fn is_block(&self) -> bool {
        matches!(self, BitcodeElement::Block(_))
    }

    /// If it is a `Block`, returns the associated block. Returns `None` otherwise.
    pub fn as_block(&self) -> Option<&Block> {
        match self {
            BitcodeElement::Block(block) => Some(block),
            BitcodeElement::Record(_) => None,
        }
    }

    /// If it is a `Block`, returns the associated mutable block. Returns `None` otherwise.
    pub fn as_block_mut(&mut self) -> Option<&mut Block> {
        match self {
            BitcodeElement::Block(block) => Some(block),
            BitcodeElement::Record(_) => None,
        }
    }

    /// Returns true if it is a `Record`
    pub fn is_record(&self) -> bool {
        matches!(self, BitcodeElement::Record(_))
    }

    /// If it is a `Record`, returns the associated record. Returns `None` otherwise.
    pub fn as_record(&self) -> Option<&Record> {
        match self {
            BitcodeElement::Block(_) => None,
            BitcodeElement::Record(record) => Some(record),
        }
    }

    /// If it is a `Record`, returns the associated mutable record. Returns `None` otherwise.
    pub fn as_record_mut(&mut self) -> Option<&mut Record> {
        match self {
            BitcodeElement::Block(_) => None,
            BitcodeElement::Record(record) => Some(record),
        }
    }
}

/// Block information
#[derive(Debug, Clone, Default)]
pub struct BlockInfo {
    /// Block name
    pub name: String,
    /// Data record names
    pub record_names: HashMap<u64, String>,
}

/// aka. Magic number
#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct Signature(u32);

impl Signature {
    pub fn new(val: u32) -> Self {
        Self(val)
    }

    pub fn into_inner(self) -> u32 {
        self.0
    }
}

impl Bitcode {
    fn clean(data: &[u8]) -> (Signature, &[u8]) {
        assert!(data.len() > 4);
        let signature = Bits::new(data).read_bits(0, 32) as u32;
        if signature == LLVM_BITCODE_WRAPPER_MAGIC {
            // It is a LLVM Bitcode wrapper, remove wrapper header
            assert!(data.len() > 20);
            let offset = u32::from_le_bytes([data[8], data[9], data[10], data[11]]) as usize;
            let size = u32::from_le_bytes([data[12], data[13], data[14], data[15]]) as usize;
            let data = &data[offset..offset + size];
            let signature = Bits::new(data).read_bits(0, 32) as u32;
            (Signature(signature), &data[4..])
        } else {
            (Signature(signature), &data[4..])
        }
    }

    /// Parse bitcode from bytes
    ///
    /// Accepts both LLVM bitcode and bitcode wrapper formats
    pub fn new(data: &[u8]) -> Result<Self, Error> {
        let (signature, stream) = Self::clean(data);
        let mut reader = BitStreamReader::new(stream);
        let mut visitor = CollectingVisitor::new();
        reader.read_block(BitStreamReader::TOP_LEVEL_BLOCK_ID, 2, &mut visitor)?;
        Ok(Self {
            signature,
            elements: visitor.finalize_top_level_elements(),
            block_info: reader.block_info,
        })
    }

    /// Read bitcode from bytes with a visitor
    ///
    /// Accepts both LLVM bitcode and bitcode wrapper formats
    pub fn read<V>(data: &[u8], visitor: &mut V) -> Result<(), Error>
    where
        V: BitStreamVisitor,
    {
        let (signature, stream) = Self::clean(data);
        visitor.validate(signature);
        let mut reader = BitStreamReader::new(stream);
        reader.read_block(BitStreamReader::TOP_LEVEL_BLOCK_ID, 2, visitor)
    }
}
