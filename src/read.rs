use std::{collections::HashMap, convert::TryFrom, error, fmt};

use crate::bitcode::{BlockInfo, Record};
use crate::bits::{self, Cursor};
use crate::bitstream::{Abbreviation, BlockInfoCode, BuiltinAbbreviationId, Operand};
use crate::visitor::BitStreamVisitor;

/// Bitstream reader errors
#[derive(Debug, Clone)]
pub enum Error {
    InvalidSignature(u32),
    InvalidAbbrev,
    NestedBlockInBlockInfo,
    MissingSetBid,
    InvalidBlockInfoRecord(u64),
    AbbrevWidthTooSmall(usize),
    NoSuchAbbrev { block_id: u64, abbrev_id: usize },
    MissingEndBlock(u64),
    ReadBits(bits::Error),
    Other(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
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

/// Bitstream reader
#[derive(Debug, Clone)]
pub struct BitStreamReader {
    /// Block information
    pub(crate) block_info: HashMap<u64, BlockInfo>,
    global_abbrevs: HashMap<u64, Vec<Abbreviation>>,
}

impl BitStreamReader {
    /// Top level fake block ID
    pub const TOP_LEVEL_BLOCK_ID: u64 = u64::MAX;

    #[must_use]
    pub fn new() -> Self {
        Self {
            block_info: HashMap::new(),
            global_abbrevs: HashMap::new(),
        }
    }

    /// Read abbreviated operand
    fn read_abbrev_op(cursor: &mut Cursor<'_>, num_ops_left: &mut usize) -> Result<Operand, Error> {
        if *num_ops_left == 0 {
            return Err(Error::InvalidAbbrev);
        }
        *num_ops_left -= 1;

        let is_literal = cursor.read(1)?;
        if is_literal == 1 {
            return Ok(Operand::Literal(cursor.read_vbr(8)?));
        }
        let op_type = cursor.read(3)?;
        Ok(match op_type {
            1 => Operand::Fixed(cursor.read_vbr(5)? as u8),
            2 => Operand::Vbr(cursor.read_vbr(5)? as u8),
            3 if *num_ops_left == 1 => {
                Operand::Array(Box::new(Self::read_abbrev_op(cursor, num_ops_left)?))
            }
            4 => Operand::Char6,
            5 if *num_ops_left == 0 => Operand::Blob,
            _ => return Err(Error::InvalidAbbrev),
        })
    }

    /// Read abbreviation
    fn define_abbrev(
        cursor: &mut Cursor<'_>,
        abbrevs: &mut Vec<Abbreviation>,
    ) -> Result<(), Error> {
        let mut num_ops = cursor.read_vbr(5)? as usize;

        let mut operands = Vec::with_capacity(num_ops);
        while num_ops > 0 && operands.len() != operands.capacity() {
            operands.push(Self::read_abbrev_op(cursor, &mut num_ops)?);
        }
        abbrevs.push(Abbreviation { operands });
        Ok(())
    }

    /// Read block info block
    pub fn read_block_info_block(
        &mut self,
        cursor: &mut Cursor<'_>,
        abbrev_width: usize,
    ) -> Result<(), Error> {
        use BuiltinAbbreviationId::*;

        let mut current_block_id = None;
        loop {
            let abbrev_id = cursor.read(abbrev_width)?;
            match BuiltinAbbreviationId::try_from(abbrev_id).map_err(|_| Error::NoSuchAbbrev {
                block_id: 0,
                abbrev_id: abbrev_id as usize,
            })? {
                EndBlock => {
                    cursor.advance(32)?;
                    return Ok(());
                }
                EnterSubBlock => {
                    return Err(Error::NestedBlockInBlockInfo);
                }
                DefineAbbreviation => {
                    let block_id = current_block_id.ok_or(Error::MissingSetBid)?;
                    Self::define_abbrev(cursor, self.global_abbrevs.entry(block_id).or_default())?;
                }
                UnabbreviatedRecord => {
                    let record = Record::from_cursor(cursor)?;
                    let block = u8::try_from(record.id)
                        .ok()
                        .and_then(|c| BlockInfoCode::try_from(c).ok())
                        .ok_or(Error::InvalidBlockInfoRecord(record.id))?;
                    match block {
                        BlockInfoCode::SetBid => {
                            let [id] = record.fields()[..] else {
                                return Err(Error::InvalidBlockInfoRecord(record.id));
                            };
                            current_block_id = Some(id);
                        }
                        BlockInfoCode::BlockName => {
                            let block_id = current_block_id.ok_or(Error::MissingSetBid)?;
                            let block_info = self.block_info.entry(block_id).or_default();
                            block_info.name = record.string(0);
                        }
                        BlockInfoCode::SetRecordName => {
                            let block_id = current_block_id.ok_or(Error::MissingSetBid)?;
                            let id = record.id;
                            let record_id = record
                                .fields()
                                .get(0)
                                .copied()
                                .ok_or(Error::InvalidBlockInfoRecord(id))?;
                            let block_info = self.block_info.entry(block_id).or_default();
                            let name = record.string(1);
                            block_info.record_names.insert(record_id, name);
                        }
                    }
                }
            }
        }
    }

    /// Read block with visitor
    pub fn read_block<V: BitStreamVisitor>(
        &mut self,
        cursor: &mut Cursor<'_>,
        block_id: u64,
        abbrev_width: usize,
        visitor: &mut V,
    ) -> Result<(), Error> {
        use BuiltinAbbreviationId::*;
        let mut block_local_abbrevs = Vec::new();

        while !cursor.is_at_end() {
            let abbrev_id = cursor.read(abbrev_width)?;
            if let Ok(abbrev_id) = BuiltinAbbreviationId::try_from(abbrev_id) {
                match abbrev_id {
                    EndBlock => {
                        cursor.advance(32)?;
                        visitor.did_exit_block(block_id);
                        return Ok(());
                    }
                    EnterSubBlock => {
                        let block_id = cursor.read_vbr(8)?;
                        let new_abbrev_width = cursor.read_vbr(4)? as usize;
                        cursor.advance(32)?;
                        let block_length = cursor.read(32)? as usize * 4;
                        let cursor = &mut cursor.take_slice(block_length)?;
                        if block_id == 0 {
                            self.read_block_info_block(cursor, new_abbrev_width)?;
                        } else {
                            if !visitor.should_enter_block(block_id) {
                                cursor.skip_bytes(block_length)?;
                                continue;
                            }
                            self.read_block(cursor, block_id, new_abbrev_width, visitor)?;
                        }
                    }
                    DefineAbbreviation => {
                        Self::define_abbrev(cursor, &mut block_local_abbrevs)?;
                    }
                    UnabbreviatedRecord => {
                        visitor.visit(block_id, Record::from_cursor(cursor)?);
                    }
                }
            } else {
                let abbrev_index = abbrev_id as usize - 4;
                let global_abbrevs = self
                    .global_abbrevs
                    .get(&block_id)
                    .map(|v| v.as_slice())
                    .unwrap_or_default();

                // > Any abbreviations defined in a BLOCKINFO record for the particular block type receive IDs first, in order,
                // > followed by any abbreviations defined within the block itself.
                let abbrev =
                    if let Some(local_index) = abbrev_index.checked_sub(global_abbrevs.len()) {
                        block_local_abbrevs.get(local_index)
                    } else {
                        global_abbrevs.get(abbrev_index)
                    };

                let abbrev = abbrev.ok_or(Error::NoSuchAbbrev {
                    block_id,
                    abbrev_id: abbrev_id as usize,
                })?;

                visitor.visit(block_id, Record::from_cursor_abbrev(cursor, abbrev)?);
                continue;
            }
        }
        if block_id != Self::TOP_LEVEL_BLOCK_ID {
            return Err(Error::MissingEndBlock(block_id));
        }
        Ok(())
    }
}
