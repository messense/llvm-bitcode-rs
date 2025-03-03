use crate::bitstream::{PayloadOperand, ScalarOperand};
use std::sync::Arc;
use std::{collections::HashMap, convert::TryFrom, error, fmt};

use crate::bitcode::{BlockInfo, RecordIter};
use crate::bits::{self, Cursor};
use crate::bitstream::{Abbreviation, BlockInfoCode, BuiltinAbbreviationId, Operand};
use crate::visitor::BitStreamVisitor;

/// Bitstream reader errors
#[derive(Debug, Clone)]
pub enum Error {
    EndOfRecord,
    ValueOverflow,
    UnexpectedOperand(Option<Operand>),
    InvalidSignature(u32),
    InvalidAbbrev,
    NestedBlockInBlockInfo,
    MissingSetBid,
    InvalidBlockInfoRecord(u64),
    NoSuchAbbrev { block_id: u32, abbrev_id: u32 },
    MissingEndBlock(u32),
    AbbrevWidthTooSmall(usize),
    ReadBits(bits::Error),
    Other(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EndOfRecord => write!(f, "read past end of record"),
            Self::ValueOverflow => write!(f, "read integer too big"),
            Self::UnexpectedOperand(op) => write!(f, "Unexpected operand {op:?}"),
            Self::InvalidSignature(sig) => {
                write!(f, "invalid signature (magic number): 0x{sig:x}")
            }
            Self::InvalidAbbrev => write!(f, "invalid abbreviation"),
            Self::NestedBlockInBlockInfo => write!(f, "nested block in block info"),
            Self::MissingSetBid => write!(f, "missing SETBID"),
            Self::InvalidBlockInfoRecord(record_id) => {
                write!(f, "invalid block info record `{record_id}`")
            }
            Self::AbbrevWidthTooSmall(width) => {
                write!(f, "abbreviation width `{width}` is too small")
            }
            Self::NoSuchAbbrev {
                block_id,
                abbrev_id,
            } => write!(
                f,
                "no such abbreviation `{abbrev_id}` in block `{block_id}`"
            ),
            Self::MissingEndBlock(block_id) => write!(f, "missing end block for `{block_id}`"),
            Self::ReadBits(err) => err.fmt(f),
            Self::Other(err) => err.fmt(f),
        }
    }
}

impl error::Error for Error {}

impl From<bits::Error> for Error {
    fn from(err: bits::Error) -> Self {
        Self::ReadBits(err)
    }
}

/// A block can contain either nested blocks or records.
/// LLVM writes blocks first, but the format allows them to be mixed freely.
#[derive(Debug)]
pub enum BlockItem<'cursor, 'input> {
    /// Recurse
    Block(BlockIter<'cursor, 'input>),
    /// Read a record from the current block
    Record(RecordIter<'cursor, 'input>),
}

/// Iterator content directly in a block
#[derive(Debug)]
pub struct BlockIter<'global_state, 'input> {
    /// ID of the block being iterated
    pub id: u32,
    cursor: Cursor<'input>,
    abbrev_width: u8,
    /// Abbreviations defined in this block
    block_local_abbrevs: Vec<Arc<Abbreviation>>,
    /// Global abbreviations and names
    reader: &'global_state mut BitStreamReader,
}

/// Bitstream reader
#[derive(Debug, Clone)]
pub struct BitStreamReader {
    /// Block information
    pub(crate) block_info: HashMap<u32, BlockInfo>,
    global_abbrevs: HashMap<u32, Vec<Arc<Abbreviation>>>,
}

impl BitStreamReader {
    /// Top level fake block ID
    pub const TOP_LEVEL_BLOCK_ID: u32 = u32::MAX;

    #[must_use]
    pub fn new() -> Self {
        Self {
            block_info: HashMap::new(),
            global_abbrevs: HashMap::new(),
        }
    }

    /// Skip `Signature` first
    pub fn iter_bitcode<'input>(&mut self, bitcode_data: &'input [u8]) -> BlockIter<'_, 'input> {
        BlockIter::new(self, Cursor::new(bitcode_data), Self::TOP_LEVEL_BLOCK_ID, 2)
    }

    fn visit_block<V: BitStreamVisitor>(
        mut block: BlockIter<'_, '_>,
        visitor: &mut V,
    ) -> Result<(), Error> {
        let block_id = block.id;
        while let Some(item) = block.next()? {
            match item {
                BlockItem::Block(new_block) => {
                    let new_id = new_block.id;
                    if visitor.should_enter_block(new_id) {
                        Self::visit_block(new_block, visitor)?;
                        visitor.did_exit_block(new_id);
                    }
                }
                BlockItem::Record(record) => {
                    visitor.visit(block_id, record.into_record()?);
                }
            }
        }
        Ok(())
    }

    /// Read abbreviated operand
    #[inline(never)]
    fn read_abbrev_op(cursor: &mut Cursor<'_>, num_ops_left: &mut usize) -> Result<Operand, Error> {
        if *num_ops_left == 0 {
            return Err(Error::InvalidAbbrev);
        }
        *num_ops_left -= 1;

        let is_literal = cursor.read(1)?;
        if is_literal == 1 {
            return Ok(Operand::Scalar(ScalarOperand::Literal(cursor.read_vbr(8)?)));
        }
        let op_type = cursor.read(3)?;
        Ok(match op_type {
            1 => Operand::Scalar(ScalarOperand::Fixed(cursor.read_vbr(5)? as u8)),
            2 => Operand::Scalar(ScalarOperand::Vbr(cursor.read_vbr(5)? as u8)),
            3 if *num_ops_left == 1 => {
                let op = Self::read_abbrev_op(cursor, num_ops_left)?;
                if let Operand::Scalar(op) = op {
                    Operand::Payload(PayloadOperand::Array(op))
                } else {
                    return Err(Error::UnexpectedOperand(Some(op)));
                }
            }
            4 => Operand::Scalar(ScalarOperand::Char6),
            5 if *num_ops_left == 0 => Operand::Payload(PayloadOperand::Blob),
            _ => return Err(Error::InvalidAbbrev),
        })
    }

    /// Read abbreviation
    fn define_abbrev(
        cursor: &mut Cursor<'_>,
        abbrevs: &mut Vec<Arc<Abbreviation>>,
    ) -> Result<(), Error> {
        let mut num_ops = cursor.read_vbr(5)? as usize;

        let mut fields = Vec::with_capacity(num_ops);
        let mut payload = None;
        while num_ops > 0 && fields.len() != fields.capacity() {
            match Self::read_abbrev_op(cursor, &mut num_ops)? {
                Operand::Scalar(op) => {
                    fields.push(op);
                }
                Operand::Payload(op) if num_ops == 0 => {
                    payload = Some(op);
                }
                op => return Err(Error::UnexpectedOperand(Some(op))),
            }
        }
        let id = abbrevs.len() as u32;
        let abbrev = Arc::new(Abbreviation {
            id,
            fields,
            payload,
        });
        abbrevs.push(abbrev);
        Ok(())
    }

    /// Read block info block
    fn read_block_info_block(
        &mut self,
        cursor: &mut Cursor<'_>,
        abbrev_width: u8,
    ) -> Result<(), Error> {
        use BuiltinAbbreviationId::*;

        let mut current_block_id: Option<u32> = None;
        loop {
            let abbrev_id = cursor.read(abbrev_width)? as u32;
            match BuiltinAbbreviationId::try_from(abbrev_id).map_err(|_| Error::NoSuchAbbrev {
                block_id: 0,
                abbrev_id,
            })? {
                EndBlock => {
                    cursor.align32()?;
                    return Ok(());
                }
                EnterSubBlock => {
                    return Err(Error::NestedBlockInBlockInfo);
                }
                DefineAbbreviation => {
                    let block_id = current_block_id.ok_or(Error::MissingSetBid)? as u32;
                    Self::define_abbrev(cursor, self.global_abbrevs.entry(block_id).or_default())?;
                }
                UnabbreviatedRecord => {
                    let mut record = RecordIter::from_cursor(cursor)?;
                    let block = u8::try_from(record.id)
                        .ok()
                        .and_then(|c| BlockInfoCode::try_from(c).ok())
                        .ok_or(Error::InvalidBlockInfoRecord(record.id))?;
                    match block {
                        BlockInfoCode::SetBid => {
                            let id = record
                                .u32()
                                .ok()
                                .filter(|_| record.is_empty())
                                .ok_or(Error::InvalidBlockInfoRecord(record.id))?;
                            current_block_id = Some(id);
                        }
                        BlockInfoCode::BlockName => {
                            let block_id = current_block_id.ok_or(Error::MissingSetBid)?;
                            let block_info = self.block_info.entry(block_id).or_default();
                            if let Ok(name) = String::from_utf8(record.string()?) {
                                block_info.name = name;
                            }
                        }
                        BlockInfoCode::SetRecordName => {
                            let block_id = current_block_id.ok_or(Error::MissingSetBid)?;
                            let record_id = record
                                .u64()
                                .map_err(|_| Error::InvalidBlockInfoRecord(record.id))?;
                            let block_info = self.block_info.entry(block_id).or_default();
                            if let Ok(name) = String::from_utf8(record.string()?) {
                                block_info.record_names.insert(record_id, name);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Read block with visitor
    pub fn read_block<V: BitStreamVisitor>(
        &mut self,
        cursor: Cursor<'_>,
        block_id: u32,
        abbrev_width: u8,
        visitor: &mut V,
    ) -> Result<(), Error> {
        Self::visit_block(
            BlockIter::new(self, cursor, block_id, abbrev_width),
            visitor,
        )
    }
}

impl<'global_state, 'input> BlockIter<'global_state, 'input> {
    /// Returns the next item (block or record) in this block
    pub fn next<'parent>(&'parent mut self) -> Result<Option<BlockItem<'parent, 'input>>, Error> {
        if self.cursor.is_at_end() {
            return if self.id == BitStreamReader::TOP_LEVEL_BLOCK_ID {
                Ok(None)
            } else {
                Err(Error::MissingEndBlock(self.id))
            };
        }

        let abbrev_id = self.cursor.read(self.abbrev_width)? as u32;

        if let Ok(builtin_abbrev) = BuiltinAbbreviationId::try_from(abbrev_id) {
            use BuiltinAbbreviationId::*;
            match builtin_abbrev {
                EndBlock => {
                    self.cursor.align32()?;
                    Ok(None)
                }
                EnterSubBlock => {
                    let block_id = self.cursor.read_vbr(8)? as u32;
                    let new_abbrev_width = self.cursor.read_vbr(4)? as u8;
                    self.cursor.align32()?;
                    let block_length = self.cursor.read(32)? as usize * 4;
                    let mut cursor = self.cursor.take_slice(block_length)?;

                    if block_id == 0 {
                        self.reader
                            .read_block_info_block(&mut cursor, new_abbrev_width)?;
                        return self.next();
                    }

                    // Create new block iterator
                    let block_iter =
                        BlockIter::new(self.reader, cursor, block_id, new_abbrev_width);
                    Ok(Some(BlockItem::Block(block_iter)))
                }
                DefineAbbreviation => {
                    BitStreamReader::define_abbrev(
                        &mut self.cursor,
                        &mut self.block_local_abbrevs,
                    )?;
                    self.next()
                }
                UnabbreviatedRecord => {
                    let record_iter = RecordIter::from_cursor(&mut self.cursor)?;
                    Ok(Some(BlockItem::Record(record_iter)))
                }
            }
        } else {
            let abbrev_index = abbrev_id as usize - 4;
            let global_abbrevs = self
                .reader
                .global_abbrevs
                .get(&self.id)
                .map(|v| v.as_slice())
                .unwrap_or_default();

            // > Any abbreviations defined in a BLOCKINFO record for the particular block type receive IDs first,
            // > followed by any abbreviations defined within the block itself.
            let abbrev = if let Some(local_index) = abbrev_index.checked_sub(global_abbrevs.len()) {
                self.block_local_abbrevs.get(local_index).cloned()
            } else {
                global_abbrevs.get(abbrev_index).cloned()
            };

            let abbrev = abbrev.ok_or(Error::NoSuchAbbrev {
                block_id: self.id,
                abbrev_id,
            })?;

            Ok(Some(BlockItem::Record(RecordIter::from_cursor_abbrev(
                &mut self.cursor,
                abbrev,
            )?)))
        }
    }

    /// Bit width of abbreviation IDs in this block.
    ///
    /// This is an implementation detail,
    /// intended only for debugging or data dumps.
    #[must_use]
    pub fn debug_abbrev_width(&self) -> u8 {
        self.abbrev_width
    }

    /// Valid only before any record or subblock has been read. This is the block size in bytes.
    ///
    /// This is an implementation detail,
    /// intended only for debugging or data dumps.
    #[must_use]
    pub fn debug_data_len(&self) -> Option<usize> {
        let bits = self.cursor.unconsumed_bit_len();
        (bits & 31 != 0).then_some(bits >> 3)
    }

    fn new(
        reader: &'global_state mut BitStreamReader,
        cursor: Cursor<'input>,
        block_id: u32,
        abbrev_width: u8,
    ) -> Self {
        Self {
            id: block_id,
            cursor,
            abbrev_width,
            block_local_abbrevs: Vec::new(),
            reader,
        }
    }
}
