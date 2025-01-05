use std::{error, fmt};

#[derive(Debug, Clone)]
pub enum Error {
    BufferOverflow,
    VbrOverflow,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::BufferOverflow => "buffer overflow",
            Self::VbrOverflow => "vbr overflow",
        })
    }
}

impl error::Error for Error {}

#[derive(Debug, Clone)]
pub struct Cursor<'a> {
    buffer: &'a [u8],
    offset: usize,
}

impl<'a> Cursor<'a> {
    pub fn new(buffer: &'a [u8]) -> Self {
        Self { buffer, offset: 0 }
    }

    pub fn is_at_end(&self) -> bool {
        self.offset >= (self.buffer.len() << 3)
    }

    pub fn peek(&self, count: usize) -> Result<u64, Error> {
        self.read_bits(count).ok_or(Error::BufferOverflow)
    }

    pub fn read(&mut self, count: usize) -> Result<u64, Error> {
        let res = self.peek(count)?;
        self.offset += count;
        Ok(res)
    }

    fn read_bits(&self, count: usize) -> Option<u64> {
        let upper_bound = self.offset + count;
        let top_byte_index = upper_bound >> 3;
        let mut res = 0;
        if upper_bound & 7 != 0 {
            let mask = (1u8 << (upper_bound & 7) as u8) - 1;
            res = u64::from(*self.buffer.get(top_byte_index)? & mask);
        }
        for i in ((self.offset >> 3)..(upper_bound >> 3)).rev() {
            res <<= 8;
            res |= u64::from(*self.buffer.get(i)?);
        }
        if self.offset & 7 != 0 {
            res >>= self.offset as u64 & 7;
        }
        Some(res)
    }

    pub fn read_bytes(&mut self, count: usize) -> Result<&[u8], Error> {
        assert_eq!(self.offset & 0b111, 0);
        let byte_start = self.offset >> 3;
        let byte_end = byte_start + count;
        let bytes = self
            .buffer
            .get(byte_start..byte_end)
            .ok_or(Error::BufferOverflow)?;
        self.offset = byte_end << 3;
        Ok(bytes)
    }

    pub fn skip_bytes(&mut self, count: usize) -> Result<(), Error> {
        assert_eq!(self.offset & 0b111, 0);
        let byte_end = (self.offset >> 3) + count;
        if byte_end > self.buffer.len() {
            return Err(Error::BufferOverflow);
        }
        self.offset = byte_end << 3;
        Ok(())
    }

    pub fn read_vbr(&mut self, width: usize) -> Result<u64, Error> {
        if width < 1 {
            return Err(Error::VbrOverflow);
        }
        let test_bit = (1 << (width - 1)) as u64;
        let mask = test_bit - 1;
        let mut res = 0;
        let mut offset = 0;
        let mut next;
        loop {
            next = self.read(width)?;
            res |= (next & mask) << offset;
            offset += width - 1;
            if offset > 64 {
                return Err(Error::VbrOverflow);
            }
            if next & test_bit == 0 {
                break;
            }
        }
        Ok(res)
    }

    pub fn advance(&mut self, align: usize) -> Result<(), Error> {
        assert!(align > 0);
        assert_eq!(align & (align - 1), 0);
        if self.offset % align == 0 {
            return Ok(());
        }
        let offset = (self.offset + align) & !(align - 1);
        if offset > (self.buffer.len() << 3) {
            return Err(Error::BufferOverflow);
        }
        self.offset = offset;
        Ok(())
    }
}
