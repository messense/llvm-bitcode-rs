//! LLVM Bitcode parser in Rust

/// Bitcode definitions
pub mod bitcode;
mod bits;
/// Bitstream definitions
pub mod bitstream;
/// Bitstream reader
pub mod read;
/// Bitstream visitor
pub mod visitor;

/// Definitions from `LLVMBitCodes.h`
pub mod schema {
    /// IDs of blocks and records inside the blocks
    pub mod blocks;
    /// Flags and values found in records
    pub mod values;
}

pub use self::bitcode::Bitcode;
pub use self::bits::Cursor;
pub use self::read::BitStreamReader;
pub use self::visitor::BitStreamVisitor;
