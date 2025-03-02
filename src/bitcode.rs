use crate::bits::Cursor;
use crate::bitstream::{Abbreviation, Operand};
use crate::bitstream::{PayloadOperand, ScalarOperand};
use std::collections::HashMap;
use std::num::NonZero;
use std::ops::Range;
use std::sync::Arc;

use crate::read::{BitStreamReader, Error};
use crate::visitor::{BitStreamVisitor, CollectingVisitor};

const LLVM_BITCODE_WRAPPER_MAGIC: u32 = 0x0B17C0DE;

/// Represents the contents of a file encoded using the
/// [LLVM bitstream container format](https://llvm.org/docs/BitCodeFormat.html#bitstream-container-format)
#[derive(Debug, Clone)]
pub struct Bitcode {
    pub signature: Signature,
    pub elements: Vec<BitcodeElement>,
    pub block_info: HashMap<u32, BlockInfo>,
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
    pub id: u32,
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
    #[must_use]
    pub fn fields(&self) -> &[u64] {
        &self.fields
    }

    pub fn take_payload(&mut self) -> Option<Payload> {
        self.payload.take()
    }
}

#[derive(Debug)]
enum Ops {
    Abbrev {
        /// If under `abbrev.fields.len()`, then it's the next op to read
        /// If equals `abbrev.fields.len()`, then payload is next
        /// If greater than `abbrev.fields.len()`, then payload has been read
        state: usize,
        abbrev: Arc<Abbreviation>,
    },
    /// Num ops left
    Full(usize),
}

/// Data records consist of a record code and a number of (up to) 64-bit integer values
///
/// The interpretation of the code and values is application specific and may vary between different block types.
#[derive(Debug)]
pub struct RecordIter<'cursor, 'input> {
    /// Record code
    pub id: u64,
    cursor: &'cursor mut Cursor<'input>,
    ops: Ops,
}

impl<'cursor, 'input> RecordIter<'cursor, 'input> {
    pub(crate) fn into_record(mut self) -> Result<Record, Error> {
        let mut fields = Vec::with_capacity(self.len());
        while let Some(f) = self.next()? {
            fields.push(f);
        }
        Ok(Record {
            id: self.id,
            fields,
            payload: self.payload().ok().flatten(),
        })
    }

    fn read_scalar_operand(cursor: &mut Cursor<'_>, operand: ScalarOperand) -> Result<u64, Error> {
        match operand {
            ScalarOperand::Char6 => {
                let value = cursor.read(6)? as u8;
                Ok(u64::from(match value {
                    0..=25 => value + b'a',
                    26..=51 => value + (b'A' - 26),
                    52..=61 => value - (52 - b'0'),
                    62 => b'.',
                    63 => b'_',
                    _ => return Err(Error::InvalidAbbrev),
                }))
            }
            ScalarOperand::Literal(value) => Ok(value),
            ScalarOperand::Fixed(width) => Ok(cursor.read(width)?),
            ScalarOperand::Vbr(width) => Ok(cursor.read_vbr(width)?),
        }
    }

    pub(crate) fn from_cursor_abbrev(
        cursor: &'cursor mut Cursor<'input>,
        abbrev: Arc<Abbreviation>,
    ) -> Result<Self, Error> {
        let id =
            Self::read_scalar_operand(cursor, *abbrev.fields.first().ok_or(Error::InvalidAbbrev)?)?;
        Ok(Self {
            id,
            cursor,
            ops: Ops::Abbrev { state: 1, abbrev },
        })
    }

    pub(crate) fn from_cursor(cursor: &'cursor mut Cursor<'input>) -> Result<Self, Error> {
        let id = cursor.read_vbr(6)?;
        let num_ops = cursor.read_vbr(6)? as usize;
        Ok(Self {
            id,
            cursor,
            ops: Ops::Full(num_ops),
        })
    }

    fn payload(&mut self) -> Result<Option<Payload>, Error> {
        match &mut self.ops {
            Ops::Abbrev { state, abbrev } => {
                if *state > abbrev.fields.len() {
                    return Ok(None);
                }
                Ok(match abbrev.payload {
                    Some(PayloadOperand::Blob) => Some(Payload::Blob(self.blob()?.to_vec())),
                    Some(PayloadOperand::Array(ScalarOperand::Char6)) => {
                        Some(Payload::Char6String(
                            String::from_utf8(self.string()?).map_err(|_| Error::InvalidAbbrev)?,
                        ))
                    }
                    Some(PayloadOperand::Array(_)) => Some(Payload::Array(self.array()?)),
                    None => None,
                })
            }
            Ops::Full(_) => Ok(None),
        }
    }

    /// Number of unread fields, excludes string/array/blob payload
    #[must_use]
    pub fn len(&self) -> usize {
        match &self.ops {
            Ops::Abbrev { state, abbrev } => abbrev.fields.len().saturating_sub(*state),
            Ops::Full(num_ops) => *num_ops,
        }
    }

    /// Matches len, excludes string/array/blob payload
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn next(&mut self) -> Result<Option<u64>, Error> {
        match &mut self.ops {
            Ops::Abbrev { state, abbrev } => {
                let Some(&op) = abbrev.fields.get(*state) else {
                    return Ok(None);
                };
                *state += 1;
                Ok(Some(Self::read_scalar_operand(self.cursor, op)?))
            }
            Ops::Full(num_ops) => {
                if *num_ops == 0 {
                    return Ok(None);
                }
                *num_ops -= 1;
                Ok(Some(self.cursor.read_vbr(6)?))
            }
        }
    }

    pub fn u64(&mut self) -> Result<u64, Error> {
        self.next()?.ok_or(Error::EndOfRecord)
    }

    pub fn nzu64(&mut self) -> Result<Option<NonZero<u64>>, Error> {
        self.u64().map(NonZero::new)
    }

    pub fn i64(&mut self) -> Result<i64, Error> {
        let v = self.u64()?;
        let shifted = (v >> 1) as i64;
        Ok(if (v & 1) == 0 {
            shifted
        } else if v != 1 {
            -shifted
        } else {
            1 << 63
        })
    }

    pub fn u32(&mut self) -> Result<u32, Error> {
        self.u64()?.try_into().map_err(|_| Error::ValueOverflow)
    }

    pub fn nzu32(&mut self) -> Result<Option<NonZero<u32>>, Error> {
        self.u32().map(NonZero::new)
    }

    pub fn u8(&mut self) -> Result<u8, Error> {
        self.u64()?.try_into().map_err(|_| Error::ValueOverflow)
    }

    pub fn try_from<U: TryFrom<u64>, T: TryFrom<U>>(&mut self) -> Result<T, Error> {
        T::try_from(self.u64()?.try_into().map_err(|_| Error::ValueOverflow)?)
            .map_err(|_| Error::ValueOverflow)
    }

    pub fn nzu8(&mut self) -> Result<Option<NonZero<u8>>, Error> {
        self.u8().map(NonZero::new)
    }

    pub fn bool(&mut self) -> Result<bool, Error> {
        match self.u64()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(Error::ValueOverflow),
        }
    }

    pub fn range(&mut self) -> Result<Range<usize>, Error> {
        let start = self.u64()? as usize;
        Ok(Range {
            start,
            end: start + self.u64()? as usize,
        })
    }

    pub fn blob(&mut self) -> Result<&'input [u8], Error> {
        match &mut self.ops {
            Ops::Abbrev { state, abbrev } => match Self::take_payload_operand(state, abbrev)? {
                Some(PayloadOperand::Blob) => {
                    let length = self.cursor.read_vbr(6)? as usize;
                    self.cursor.align32()?;
                    let data = self.cursor.read_bytes(length)?;
                    self.cursor.align32()?;
                    Ok(data)
                }
                other => Err(Error::UnexpectedOperand(other.map(Operand::Payload))),
            },
            Ops::Full(_) => Err(Error::UnexpectedOperand(None)),
        }
    }

    pub fn array(&mut self) -> Result<Vec<u64>, Error> {
        match &mut self.ops {
            Ops::Abbrev { state, abbrev } => match Self::take_payload_operand(state, abbrev)? {
                Some(PayloadOperand::Array(op)) => {
                    let len = self.cursor.read_vbr(6)? as usize;
                    let mut out = Vec::with_capacity(len);
                    for _ in 0..len {
                        if out.len() == out.capacity() {
                            debug_assert!(false);
                            break;
                        }
                        out.push(Self::read_scalar_operand(self.cursor, op)?);
                    }
                    Ok(out)
                }
                other => Err(Error::UnexpectedOperand(other.map(Operand::Payload))),
            },
            // Not a proper array payload, but this fallback pattern is used by LLVM
            Ops::Full(num_ops) => {
                let len = *num_ops;
                *num_ops = 0;
                let mut out = Vec::with_capacity(len);
                for _ in 0..len {
                    if out.len() == out.capacity() {
                        debug_assert!(false);
                        break;
                    }
                    out.push(self.cursor.read_vbr(6)?);
                }
                Ok(out)
            }
        }
    }

    /// Mark payload as read, if there is one
    fn take_payload_operand(
        state: &mut usize,
        abbrev: &Abbreviation,
    ) -> Result<Option<PayloadOperand>, Error> {
        if *state == abbrev.fields.len() {
            if abbrev.payload.is_some() {
                *state += 1;
            }
            Ok(abbrev.payload)
        } else {
            Err(Error::UnexpectedOperand(
                abbrev.fields.get(*state).copied().map(Operand::Scalar),
            ))
        }
    }

    /// Read remainder of the fields as string chars
    /// The strings are just binary blobs. LLVM doesn't guarantee any encoding.
    pub fn string(&mut self) -> Result<Vec<u8>, Error> {
        match &mut self.ops {
            Ops::Abbrev { state, abbrev } => match Self::take_payload_operand(state, abbrev)? {
                Some(PayloadOperand::Array(el)) => {
                    *state += 1;
                    let len = self.cursor.read_vbr(6)? as usize;
                    let mut out = Vec::with_capacity(len);

                    match el {
                        ScalarOperand::Char6 => {
                            for _ in 0..len {
                                if out.len() == out.capacity() {
                                    debug_assert!(false);
                                    break;
                                }
                                let ch = match self.cursor.read(6)? as u8 {
                                    value @ 0..=25 => value + b'a',
                                    value @ 26..=51 => value + (b'A' - 26),
                                    value @ 52..=61 => value - (52 - b'0'),
                                    62 => b'.',
                                    63 => b'_',
                                    _ => return Err(Error::InvalidAbbrev),
                                };
                                out.push(ch);
                            }
                        }
                        ScalarOperand::Fixed(width @ 6..=8) => {
                            for _ in 0..len {
                                if out.len() == out.capacity() {
                                    debug_assert!(false);
                                    break;
                                }
                                out.push(self.cursor.read(width)? as u8);
                            }
                        }
                        other => {
                            return Err(Error::UnexpectedOperand(Some(Operand::Scalar(other))));
                        }
                    }
                    Ok(out)
                }
                other => Err(Error::UnexpectedOperand(other.map(Operand::Payload))),
            },
            Ops::Full(num_ops) => {
                let len = std::mem::replace(num_ops, 0);
                let mut out = Vec::with_capacity(len);
                for _ in 0..len {
                    let ch = self.cursor.read_vbr(6)?;
                    out.push(u8::try_from(ch).map_err(|_| Error::ValueOverflow)?);
                }
                Ok(out)
            }
        }
    }

    /// Zero-terminated string, assumes latin1 encoding
    pub fn zstring(&mut self) -> Result<String, Error> {
        let mut s = String::new();
        while let Some(b) = self.nzu8()? {
            s.push(b.get() as char);
        }
        Ok(s)
    }
}

impl Iterator for RecordIter<'_, '_> {
    type Item = Result<u64, Error>;
    fn next(&mut self) -> Option<Self::Item> {
        self.next().transpose()
    }
}

impl Drop for RecordIter<'_, '_> {
    /// Must drain the remaining records to advance the cursor to the next record
    fn drop(&mut self) {
        while let Ok(Some(_)) = self.next() {}
        if let Ops::Abbrev { abbrev, .. } = &self.ops {
            if abbrev.payload.is_some() {
                let _ = self.payload();
            }
        }
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
            Cursor::new(stream),
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
            Cursor::new(stream),
            BitStreamReader::TOP_LEVEL_BLOCK_ID,
            2,
            visitor,
        )
    }
}
