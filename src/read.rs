use std::{collections::HashMap, convert::TryFrom, error, fmt, mem};

use crate::bitcode::{BlockInfo, Payload, Record, Signature};
use crate::bits::{self, Bits, Cursor};
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
pub struct BitStreamReader<'a> {
    cursor: Cursor<'a>,
    /// Block information
    pub(crate) block_info: HashMap<u64, BlockInfo>,
    global_abbrevs: HashMap<u64, Vec<Abbreviation>>,
}

impl<'a> BitStreamReader<'a> {
    /// Top level fake block ID
    pub const TOP_LEVEL_BLOCK_ID: u64 = u64::MAX;

    /// Create a new reader from bytes
    #[must_use]
    pub fn new(buffer: &'a [u8]) -> Self {
        let cursor = Cursor::new(Bits::new(buffer));
        Self {
            cursor,
            block_info: HashMap::new(),
            global_abbrevs: HashMap::new(),
        }
    }

    /// Read signature, aka. Magic Number
    pub fn read_signature(&mut self) -> Result<Signature, Error> {
        assert!(self.cursor.is_at_start());
        let bits = self.cursor.read(mem::size_of::<u32>() * 8)? as u32;
        Ok(Signature::new(bits))
    }

    /// Read abbreviated operand
    pub fn read_abbrev_op(&mut self) -> Result<Operand, Error> {
        let is_literal = self.cursor.read(1)?;
        if is_literal == 1 {
            return Ok(Operand::Literal(self.cursor.read_vbr(8)?));
        }
        let op_type = self.cursor.read(3)?;
        let op = match op_type {
            1 => Operand::Fixed(self.cursor.read_vbr(5)? as u8),
            2 => Operand::Vbr(self.cursor.read_vbr(5)? as u8),
            3 => Operand::Array(Box::new(self.read_abbrev_op()?)),
            4 => Operand::Char6,
            5 => Operand::Blob,
            _ => return Err(Error::InvalidAbbrev),
        };
        Ok(op)
    }

    /// Read abbreviation
    pub fn read_abbrev(&mut self, num_ops: usize) -> Result<Abbreviation, Error> {
        if num_ops == 0 {
            return Err(Error::InvalidAbbrev);
        }
        let mut operands = Vec::new();
        for i in 0..num_ops {
            let op = self.read_abbrev_op()?;
            let is_array = op.is_array();
            let is_blob = op.is_blob();
            operands.push(op);
            if is_array {
                if i == num_ops - 2 {
                    break;
                } else {
                    return Err(Error::InvalidAbbrev);
                }
            } else if is_blob && i != num_ops - 1 {
                return Err(Error::InvalidAbbrev);
            }
        }
        Ok(Abbreviation { operands })
    }

    fn read_single_abbreviated_record_operand(&mut self, operand: &Operand) -> Result<u64, Error> {
        match operand {
            Operand::Char6 => {
                let value = self.cursor.read(6)?;
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
            Operand::Fixed(width) => Ok(self.cursor.read(*width as usize)?),
            Operand::Vbr(width) => Ok(self.cursor.read_vbr(*width as usize)?),
            Operand::Array(_) | Operand::Blob => Err(Error::InvalidAbbrev),
        }
    }

    /// Read abbreviated data record
    pub fn read_abbreviated_record(&mut self, abbrev: &Abbreviation) -> Result<Record, Error> {
        let code = self.read_single_abbreviated_record_operand(abbrev.operands.first().unwrap())?;
        let last_operand = abbrev.operands.last().unwrap();
        let last_regular_operand_index =
            abbrev.operands.len() - (if last_operand.is_payload() { 1 } else { 0 });
        let mut fields = Vec::new();
        for op in &abbrev.operands[1..last_regular_operand_index] {
            fields.push(self.read_single_abbreviated_record_operand(op)?);
        }
        let payload = if last_operand.is_payload() {
            match last_operand {
                Operand::Array(element) => {
                    let length = self.cursor.read_vbr(6)? as usize;
                    let mut elements = Vec::with_capacity(length);
                    for _ in 0..length {
                        elements.push(self.read_single_abbreviated_record_operand(element)?);
                    }
                    if matches!(**element, Operand::Char6) {
                        Some(Payload::Char6String(string_from_u64s(&elements)))
                    } else {
                        Some(Payload::Array(elements))
                    }
                }
                Operand::Blob => {
                    let length = self.cursor.read_vbr(6)? as usize;
                    self.cursor.advance(32)?;
                    let data = self.cursor.read_bytes(length)?.to_vec();
                    self.cursor.advance(32)?;
                    Some(Payload::Blob(data))
                }
                _ => unreachable!(),
            }
        } else {
            None
        };
        Ok(Record {
            id: code,
            fields,
            payload,
        })
    }

    /// Read block info block
    pub fn read_block_info_block(&mut self, abbrev_width: usize) -> Result<(), Error> {
        use BuiltinAbbreviationId::*;

        let mut current_block_id = None;
        loop {
            let abbrev_id = self.cursor.read(abbrev_width)?;
            match BuiltinAbbreviationId::try_from(abbrev_id).map_err(|_| Error::NoSuchAbbrev {
                block_id: 0,
                abbrev_id: abbrev_id as usize,
            })? {
                EndBlock => {
                    self.cursor.advance(32)?;
                    return Ok(());
                }
                EnterSubBlock => {
                    return Err(Error::NestedBlockInBlockInfo);
                }
                DefineAbbreviation => {
                    if let Some(block_id) = current_block_id {
                        let num_ops = self.cursor.read_vbr(5)? as usize;
                        let abbrev = self.read_abbrev(num_ops)?;
                        let abbrevs = self.global_abbrevs.entry(block_id).or_default();
                        abbrevs.push(abbrev);
                    } else {
                        return Err(Error::MissingSetBid);
                    }
                }
                UnabbreviatedRecord => {
                    let code = self.cursor.read_vbr(6)?;
                    let num_ops = self.cursor.read_vbr(6)? as usize;
                    let mut operands = Vec::with_capacity(num_ops);
                    for _ in 0..num_ops {
                        operands.push(self.cursor.read_vbr(6)?);
                    }
                    let block = u8::try_from(code)
                        .ok()
                        .and_then(|c| BlockInfoCode::try_from(c).ok())
                        .ok_or(Error::InvalidBlockInfoRecord(code))?;
                    match block {
                        BlockInfoCode::SetBid => {
                            if operands.len() != 1 {
                                return Err(Error::InvalidBlockInfoRecord(code));
                            }
                            current_block_id = operands.first().copied();
                        }
                        BlockInfoCode::BlockName => {
                            if let Some(block_id) = current_block_id {
                                let block_info = self.block_info.entry(block_id).or_default();
                                block_info.name = string_from_u64s(&operands);
                            } else {
                                return Err(Error::MissingSetBid);
                            }
                        }
                        BlockInfoCode::SetRecordName => {
                            if let Some(block_id) = current_block_id {
                                if let Some((record_id, name)) = operands.split_first() {
                                    let block_info = self.block_info.entry(block_id).or_default();
                                    let name = string_from_u64s(name);
                                    block_info.record_names.insert(*record_id, name);
                                } else {
                                    return Err(Error::InvalidBlockInfoRecord(code));
                                }
                            } else {
                                return Err(Error::MissingSetBid);
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
        id: u64,
        abbrev_width: usize,
        visitor: &mut V,
    ) -> Result<(), Error> {
        use BuiltinAbbreviationId::*;

        while !self.cursor.is_at_end() {
            let abbrev_id = self.cursor.read(abbrev_width)?;
            if let Ok(abbrev_id) = BuiltinAbbreviationId::try_from(abbrev_id) {
                match abbrev_id {
                    EndBlock => {
                        self.cursor.advance(32)?;
                        visitor.did_exit_block();
                        return Ok(());
                    }
                    EnterSubBlock => {
                        let block_id = self.cursor.read_vbr(8)?;
                        let new_abbrev_width = self.cursor.read_vbr(4)? as usize;
                        self.cursor.advance(32)?;
                        let block_length = self.cursor.read(32)? as usize * 4;
                        if block_id == 0 {
                            self.read_block_info_block(new_abbrev_width)?
                        } else {
                            if !visitor.should_enter_block(block_id) {
                                self.cursor.skip_bytes(block_length)?;
                                continue;
                            }
                            self.read_block(block_id, new_abbrev_width, visitor)?;
                        }
                    }
                    DefineAbbreviation => {
                        let num_ops = self.cursor.read_vbr(5)? as usize;
                        let abbrev = self.read_abbrev(num_ops)?;
                        let abbrev_info = self.global_abbrevs.entry(id).or_default();
                        abbrev_info.push(abbrev);
                    }
                    UnabbreviatedRecord => {
                        let code = self.cursor.read_vbr(6)?;
                        let num_ops = self.cursor.read_vbr(6)? as usize;
                        let mut operands = Vec::with_capacity(num_ops);
                        for _ in 0..num_ops {
                            operands.push(self.cursor.read_vbr(6)?);
                        }
                        visitor.visit(Record {
                            id: code,
                            fields: operands,
                            payload: None,
                        });
                    }
                }
            } else {
                if let Some(abbrev_info) = self.global_abbrevs.get(&id).cloned() {
                    let abbrev_id = abbrev_id as usize;
                    if abbrev_id - 4 < abbrev_info.len() {
                        visitor.visit(self.read_abbreviated_record(&abbrev_info[abbrev_id - 4])?);
                        continue;
                    }
                }
                return Err(Error::NoSuchAbbrev {
                    block_id: id,
                    abbrev_id: abbrev_id as usize,
                });
            }
        }
        if id != Self::TOP_LEVEL_BLOCK_ID {
            return Err(Error::MissingEndBlock(id));
        }
        Ok(())
    }
}

fn string_from_u64s(slice: &[u64]) -> String {
    slice
        .iter()
        .map(|&x| {
            u32::try_from(x)
                .ok()
                .and_then(char::from_u32)
                .unwrap_or('\u{fffd}')
        })
        .collect()
}
