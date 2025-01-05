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
pub struct Bits<'a> {
    buffer: &'a [u8],
}

impl<'a> Bits<'a> {
    pub fn new(buffer: &'a [u8]) -> Self {
        Self { buffer }
    }

    pub fn read_bits(&self, offset: usize, count: usize) -> u64 {
        let upper_bound = offset.wrapping_add(count);
        assert!(count <= 64);
        assert!(upper_bound >= offset);
        assert!(upper_bound <= self.buffer.len() << 3);
        let top_byte_index = upper_bound >> 3;
        let mut res = 0;
        if upper_bound & 7 != 0 {
            let mask = (1u8 << (upper_bound & 7) as u8).wrapping_sub(1);
            res = u64::from(self.buffer[top_byte_index] & mask);
        }
        for i in ((offset >> 3)..(upper_bound >> 3)).rev() {
            res <<= 8;
            res |= u64::from(self.buffer[i]);
        }
        if offset & 7 != 0 {
            res >>= offset as u64 & 7;
        }
        res
    }

    pub fn len(&self) -> usize {
        self.buffer.len() << 3
    }
}

#[derive(Debug, Clone)]
pub struct Cursor<'a> {
    buffer: Bits<'a>,
    offset: usize,
}

impl<'a> Cursor<'a> {
    pub fn new(buffer: Bits<'a>) -> Self {
        Self { buffer, offset: 0 }
    }

    pub fn is_at_start(&self) -> bool {
        self.offset == 0
    }

    pub fn is_at_end(&self) -> bool {
        // TODO: verify this
        self.offset == self.buffer.len()
    }

    pub fn peek(&self, count: usize) -> Result<u64, Error> {
        if self.buffer.len() - self.offset < count {
            return Err(Error::BufferOverflow);
        }
        Ok(self.buffer.read_bits(self.offset, count))
    }

    pub fn read(&mut self, count: usize) -> Result<u64, Error> {
        let res = self.peek(count)?;
        self.offset += count;
        Ok(res)
    }

    pub fn read_bytes(&mut self, count: usize) -> Result<&'a [u8], Error> {
        assert_eq!(self.offset & 0b111, 0);
        let byte_start = self.offset >> 3;
        let byte_end = byte_start + count;
        let bytes = self
            .buffer
            .buffer
            .get(byte_start..byte_end)
            .ok_or(Error::BufferOverflow)?;
        self.offset = byte_end << 3;
        Ok(bytes)
    }

    pub fn skip_bytes(&mut self, count: usize) -> Result<(), Error> {
        assert_eq!(self.offset & 0b111, 0);
        let offset = self.offset.wrapping_add(count << 3);
        assert!(offset >= self.offset);
        if offset > self.buffer.len() {
            return Err(Error::BufferOverflow);
        }
        self.offset = offset;
        Ok(())
    }

    pub fn read_vbr(&mut self, width: usize) -> Result<u64, Error> {
        assert!(width > 1);
        let test_bit = (1 << width.wrapping_sub(1)) as u64;
        let mask = test_bit.wrapping_sub(1);
        let mut res = 0;
        let mut offset = 0;
        let mut next;
        loop {
            next = self.read(width)?;
            res |= (next & mask) << offset;
            offset += width.wrapping_sub(1);
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
        assert!(self.offset.wrapping_add(align.wrapping_sub(1)) >= self.offset);
        assert_eq!(align & align.wrapping_sub(1), 0);
        if self.offset % align == 0 {
            return Ok(());
        }
        let offset = (self.offset.wrapping_add(align)) & !(align.wrapping_sub(1));
        if offset > self.buffer.len() {
            return Err(Error::BufferOverflow);
        }
        self.offset = offset;
        Ok(())
    }
}
