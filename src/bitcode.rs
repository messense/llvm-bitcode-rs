use crate::bits::Cursor;
use crate::bitstream::{Abbreviation, Operand};
use std::collections::HashMap;

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
    fields: Vec<u64>,
    /// Array and Blob encoding has payload
    payload: Option<Payload>,
}

impl Record {
    pub fn fields(&self) -> &[u64] {
        &self.fields
    }

    pub fn take_payload(&mut self) -> Option<Payload> {
        self.payload.take()
    }

    fn read_single_abbreviated_record_operand(
        cursor: &mut Cursor<'_>,
        operand: &Operand,
    ) -> Result<u64, Error> {
        match operand {
            Operand::Char6 => {
                let value = cursor.read(6)?;
                match value {
                    0..=25 => Ok(value + u64::from('a' as u32)),
                    26..=51 => Ok(value + u64::from('A' as u32) - 26),
                    52..=61 => Ok(value + u64::from('0' as u32) - 52),
                    62 => Ok(u64::from('.' as u32)),
                    63 => Ok(u64::from('_' as u32)),
                    _ => Err(Error::InvalidAbbrev),
                }
            }
            Operand::Literal(value) => Ok(*value),
            Operand::Fixed(width) => Ok(cursor.read(*width as usize)?),
            Operand::Vbr(width) => Ok(cursor.read_vbr(*width as usize)?),
            Operand::Array(_) | Operand::Blob => Err(Error::InvalidAbbrev),
        }
    }

    pub(crate) fn from_cursor_abbrev(
        cursor: &mut Cursor<'_>,
        abbrev: &Abbreviation,
    ) -> Result<Record, Error> {
        let code =
            Self::read_single_abbreviated_record_operand(cursor, abbrev.operands.first().unwrap())?;
        let last_operand = abbrev.operands.last().unwrap();
        let last_regular_operand_index =
            abbrev.operands.len() - (if last_operand.is_payload() { 1 } else { 0 });
        let mut fields = Vec::new();
        for op in &abbrev.operands[1..last_regular_operand_index] {
            fields.push(Self::read_single_abbreviated_record_operand(cursor, op)?);
        }
        let payload = if last_operand.is_payload() {
            match last_operand {
                Operand::Array(element) => {
                    let length = cursor.read_vbr(6)? as usize;
                    if matches!(**element, Operand::Char6) {
                        let mut s = String::with_capacity(length);
                        for _ in 0..length {
                            s.push(
                                u32::try_from(Self::read_single_abbreviated_record_operand(
                                    cursor, element,
                                )?)
                                .ok()
                                .and_then(char::from_u32)
                                .unwrap_or('\u{fffd}'),
                            );
                        }

                        Some(Payload::Char6String(s))
                    } else {
                        let mut elements = Vec::with_capacity(length);
                        for _ in 0..length {
                            elements.push(Self::read_single_abbreviated_record_operand(
                                cursor, element,
                            )?);
                        }
                        Some(Payload::Array(elements))
                    }
                }
                Operand::Blob => {
                    let length = cursor.read_vbr(6)? as usize;
                    cursor.advance(32)?;
                    let data = cursor.read_bytes(length)?.to_vec();
                    cursor.advance(32)?;
                    Some(Payload::Blob(data))
                }
                _ => unreachable!(),
            }
        } else {
            None
        };
        Ok(Self {
            id: code,
            fields,
            payload,
        })
    }

    pub(crate) fn from_cursor<'input>(cursor: &mut Cursor<'input>) -> Result<Record, Error> {
        let code = cursor.read_vbr(6)?;
        let num_ops = cursor.read_vbr(6)? as usize;
        let mut operands = Vec::with_capacity(num_ops);
        for _ in 0..num_ops {
            operands.push(cursor.read_vbr(6)?);
        }
        let record = Record {
            id: code,
            fields: operands,
            payload: None,
        };
        Ok(record)
    }

    pub fn string(&self, start_at: usize) -> String {
        self.fields
            .iter()
            .skip(start_at)
            .map(|&x| {
                u32::try_from(x)
                    .ok()
                    .and_then(char::from_u32)
                    .unwrap_or('\u{fffd}')
            })
            .collect()
    }
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
    #[must_use]
    pub fn is_block(&self) -> bool {
        matches!(self, Self::Block(_))
    }

    /// If it is a `Block`, returns the associated block. Returns `None` otherwise.
    #[must_use]
    pub fn as_block(&self) -> Option<&Block> {
        match self {
            Self::Block(block) => Some(block),
            Self::Record(_) => None,
        }
    }

    /// If it is a `Block`, returns the associated mutable block. Returns `None` otherwise.
    pub fn as_block_mut(&mut self) -> Option<&mut Block> {
        match self {
            Self::Block(block) => Some(block),
            Self::Record(_) => None,
        }
    }

    /// Returns true if it is a `Record`
    #[must_use]
    pub fn is_record(&self) -> bool {
        matches!(self, Self::Record(_))
    }

    /// If it is a `Record`, returns the associated record. Returns `None` otherwise.
    #[must_use]
    pub fn as_record(&self) -> Option<&Record> {
        match self {
            Self::Block(_) => None,
            Self::Record(record) => Some(record),
        }
    }

    /// If it is a `Record`, returns the associated mutable record. Returns `None` otherwise.
    pub fn as_record_mut(&mut self) -> Option<&mut Record> {
        match self {
            Self::Block(_) => None,
            Self::Record(record) => Some(record),
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
#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq)]
pub struct Signature {
    pub magic: u32,
    pub magic2: u32,
    pub version: u32,
    pub offset: u32,
    pub size: u32,
    pub cpu_type: u32,
}

impl Signature {
    #[must_use]
    pub fn parse(data: &[u8]) -> Option<(Self, &[u8])> {
        let (signature, remaining_data) = data.split_first_chunk::<4>()?;
        let magic = u32::from_le_bytes(*signature);
        if magic != LLVM_BITCODE_WRAPPER_MAGIC {
            Some((
                Signature {
                    version: 0,
                    magic,
                    magic2: 0,
                    offset: 4,
                    size: remaining_data.len() as _,
                    cpu_type: 0,
                },
                remaining_data,
            ))
        } else {
            // It is a LLVM Bitcode wrapper, remove wrapper header
            if data.len() < 20 {
                return None;
            }
            let mut words = data
                .chunks_exact(4)
                .skip(1)
                .map(|w| u32::from_le_bytes(w.try_into().unwrap()));
            let version = words.next()?;
            let offset = words.next()?;
            let size = words.next()?;
            let cpu_id = words.next()?;
            let data = data.get(offset as usize..offset as usize + size as usize)?;
            let (magic2, remaining_data) = data.split_first_chunk::<4>()?;
            let magic2 = u32::from_le_bytes(*magic2);
            Some((
                Signature {
                    version,
                    magic,
                    magic2,
                    offset,
                    size,
                    cpu_type: cpu_id,
                },
                remaining_data,
            ))
        }
    }
}

impl Bitcode {
    /// Parse bitcode from bytes
    ///
    /// Accepts both LLVM bitcode and bitcode wrapper formats
    pub fn new(data: &[u8]) -> Result<Self, Error> {
        let (signature, stream) = Signature::parse(data).ok_or(Error::InvalidSignature(0))?;
        let mut reader = BitStreamReader::new();
        let mut visitor = CollectingVisitor::new();
        reader.read_block(
            &mut Cursor::new(stream),
            BitStreamReader::TOP_LEVEL_BLOCK_ID,
            2,
            &mut visitor,
        )?;
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
        let (header, stream) = Signature::parse(data).ok_or(Error::InvalidSignature(0))?;
        if !visitor.validate(header) {
            return Err(Error::InvalidSignature(header.magic));
        }
        let mut reader = BitStreamReader::new();
        reader.read_block(
            &mut Cursor::new(stream),
            BitStreamReader::TOP_LEVEL_BLOCK_ID,
            2,
            visitor,
        )
    }
}
