use std::{error, fmt};

#[derive(Debug, Clone)]
pub enum Error {
    BufferOverflow,
    VbrOverflow,
    Alignment,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::BufferOverflow => "buffer overflow",
            Self::VbrOverflow => "vbr overflow",
            Self::Alignment => "bad alignment",
        })
    }
}

impl error::Error for Error {}

#[derive(Clone)]
pub struct Cursor<'input> {
    buffer: &'input [u8],
    offset: usize,
}

impl<'input> Cursor<'input> {
    #[must_use]
    pub fn new(buffer: &'input [u8]) -> Self {
        Self { buffer, offset: 0 }
    }

    #[must_use]
    pub fn is_at_end(&self) -> bool {
        self.offset >= (self.buffer.len() << 3)
    }

    #[inline]
    pub fn peek(&self, bits: u8) -> Result<u64, Error> {
        self.read_bits(bits).ok_or(Error::BufferOverflow)
    }

    #[inline]
    pub fn read(&mut self, bits: u8) -> Result<u64, Error> {
        if bits < 1 || bits > 64 {
            return Err(Error::VbrOverflow);
        }
        let res = self.peek(bits)?;
        self.offset += bits as usize;
        Ok(res)
    }

    fn read_bits(&self, count: u8) -> Option<u64> {
        let upper_bound = self.offset + count as usize;
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

    pub fn read_bytes(&mut self, length_bytes: usize) -> Result<&'input [u8], Error> {
        if self.offset % 8 != 0 {
            return Err(Error::Alignment);
        }
        let byte_start = self.offset >> 3;
        let byte_end = byte_start + length_bytes;
        let bytes = self
            .buffer
            .get(byte_start..byte_end)
            .ok_or(Error::BufferOverflow)?;
        self.offset = byte_end << 3;
        Ok(bytes)
    }

    pub fn skip_bytes(&mut self, count: usize) -> Result<(), Error> {
        if self.offset % 8 != 0 {
            return Err(Error::Alignment);
        }
        let byte_end = (self.offset >> 3) + count;
        if byte_end > self.buffer.len() {
            return Err(Error::BufferOverflow);
        }
        self.offset = byte_end << 3;
        Ok(())
    }

    /// Create a cursor for `length_bytes`, and skip over `length_bytes`
    /// Must be aligned to 32 bits.
    pub(crate) fn take_slice(&mut self, length_bytes: usize) -> Result<Self, Error> {
        if self.offset % 32 != 0 {
            return Err(Error::Alignment);
        }
        Ok(Cursor {
            buffer: self.read_bytes(length_bytes)?,
            offset: 0,
        })
    }

    /// Read a VBR number in `width`-wide encoding.
    /// The number may be up to 64-bit long regardless of the `width`.
    #[inline]
    pub fn read_vbr(&mut self, width: u8) -> Result<u64, Error> {
        if width < 1 || width > 32 {
            // This is `MaxChunkSize` in LLVM
            return Err(Error::VbrOverflow);
        }
        let test_bit = 1u64 << (width - 1);
        let mask = test_bit - 1;
        let mut res = 0;
        let mut offset = 0;
        loop {
            let next = self.read(width)?;
            res |= (next & mask) << offset;
            offset += width - 1;
            // 64 may not be divisible by width
            if offset > 63 + width {
                return Err(Error::VbrOverflow);
            }
            if next & test_bit == 0 {
                break;
            }
        }
        Ok(res)
    }

    /// Skip bytes until a 32-bit boundary (no-op if already aligned)
    pub fn align32(&mut self) -> Result<(), Error> {
        let new_offset = if self.offset % 32 == 0 {
            self.offset
        } else {
            (self.offset + 32) & !(32 - 1)
        };
        self.buffer = self
            .buffer
            .get((new_offset >> 3)..)
            .ok_or(Error::BufferOverflow)?;
        self.offset = 0;
        Ok(())
    }

    /// Maximum number of bits that can be read
    #[must_use]
    pub fn unconsumed_bit_len(&self) -> usize {
        (self.buffer.len() << 3) - self.offset
    }
}

struct CursorDebugBytes<'a>(&'a [u8]);

impl fmt::Debug for CursorDebugBytes<'_> {
    #[cold]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[0x")?;
        for &b in self.0.iter().take(200) {
            write!(f, "{b:02x}")?;
        }
        if self.0.len() > 200 {
            f.write_str("...")?;
        }
        write!(f, "; {}]", self.0.len())
    }
}

impl fmt::Debug for Cursor<'_> {
    /// Debug-print only the accessible part of the internal buffer
    #[cold]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let byte_offset = self.offset / 8;
        let bit_offset = self.offset % 8;
        let buffer = CursorDebugBytes(self.buffer.get(byte_offset..).unwrap_or_default());
        f.debug_struct("Cursor")
            .field("offset", &bit_offset)
            .field("buffer", &buffer)
            .field("nextvbr6", &self.peek(6).ok())
            .finish()
    }
}

#[test]
fn test_cursor_bits() {
    let mut c = Cursor::new(&[0b1000_0000]);
    assert_eq!(0, c.peek(1).unwrap());
    assert!(c.peek(9).is_err());
    assert_eq!(0, c.peek(2).unwrap());
    assert_eq!(0, c.peek(3).unwrap());
    assert_eq!(0, c.peek(4).unwrap());
    assert_eq!(0, c.peek(5).unwrap());
    assert_eq!(0, c.peek(6).unwrap());
    assert_eq!(0, c.peek(7).unwrap());
    assert_eq!(0b1000_0000, c.peek(8).unwrap());
    assert_eq!(0, c.read(6).unwrap());
    assert_eq!(0b10, c.peek(2).unwrap());
    assert_eq!(0, c.peek(1).unwrap());
    assert_eq!(0, c.read(1).unwrap());
    assert_eq!(0b1, c.peek(1).unwrap());
    assert_eq!(0b1, c.read(1).unwrap());

    let mut c = Cursor::new(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 0x55, 0x11, 0xff, 1, 127, 0x51]);
    assert_eq!(0, c.peek(1).unwrap());
    assert_eq!(0b1_0000_0000, c.peek(9).unwrap());
    assert_eq!(0, c.peek(2).unwrap());
    assert_eq!(0, c.peek(3).unwrap());
    assert_eq!(0, c.peek(4).unwrap());
    assert_eq!(0, c.peek(5).unwrap());
    assert_eq!(0, c.peek(6).unwrap());
    assert_eq!(0, c.peek(7).unwrap());
    assert_eq!(0, c.peek(8).unwrap());
    assert_eq!(0b1_0000_0000, c.peek(9).unwrap());

    assert_eq!(0, c.peek(7).unwrap());
    assert!(c.read(0).is_err());
    assert_eq!(0, c.read(1).unwrap());
    assert_eq!(0, c.read(2).unwrap());
    assert_eq!(0, c.read(3).unwrap());
    assert_eq!(4, c.read(4).unwrap());
    assert_eq!(0, c.read(5).unwrap());
    assert_eq!(4, c.read(6).unwrap());
    assert_eq!(24, c.read(7).unwrap());
    assert_eq!(64, c.read(8).unwrap());
    assert_eq!(80, c.read(9).unwrap());
    c.align32().unwrap();
    let mut d = c.take_slice(6).unwrap();
    assert_eq!(0x51, c.read(8).unwrap());
    assert!(d.read(0).is_err());
    assert_eq!(0, d.read(1).unwrap());
    assert_eq!(0, d.read(2).unwrap());
    assert_eq!(1, d.read(3).unwrap());
    assert_eq!(4, d.read(4).unwrap());
    assert_eq!(21, d.read(5).unwrap());
    assert_eq!(34, d.read(6).unwrap());
    assert_eq!(120, d.read(7).unwrap());
    assert_eq!(31, d.read(8).unwrap());
    assert!(d.read(63).is_err());
    assert_eq!(496, d.read(9).unwrap());
    assert!(d.read(0).is_err());
    assert_eq!(1, d.read(1).unwrap());
    assert!(d.align32().is_err());
    assert_eq!(1, d.read(2).unwrap());
    assert!(d.align32().is_err());
    assert!(d.read(1).is_err());
}

#[test]
fn test_cursor_bytes() {
    let mut c = Cursor::new(&[0, 1, 2, 3, 4, 5, 6, 7, 8]);
    c.align32().unwrap();
    assert_eq!(0x0100, c.peek(16).unwrap());
    assert_eq!(0x020100, c.peek(24).unwrap());
    assert_eq!(0x03020100, c.peek(32).unwrap());
    assert_eq!(0x0100, c.read(16).unwrap());
    assert_eq!(0x02, c.read(8).unwrap());
    assert_eq!([3, 4, 5, 6], c.read_bytes(4).unwrap());
    c.skip_bytes(1).unwrap();
    assert!(c.read_bytes(2).is_err());
    assert_eq!([8], c.read_bytes(1).unwrap());
}
