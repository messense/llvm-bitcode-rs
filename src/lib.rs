//! LLVM Bitcode parser in Rust

/// Bitcode definitions
pub mod bitcode;
mod bits;
/// Bitstream definitions
pub mod bitstream;
/// LLVM IR
pub mod ir;
/// Bitstream reader
pub mod read;
/// Bitstream visitor
pub mod visitor;

pub use self::bitcode::Bitcode;
pub use self::read::BitStreamReader;
pub use self::visitor::BitStreamVisitor;
