# llvm-bitcode-rs

[![GitHub Actions](https://github.com/messense/llvm-bitcode-rs/workflows/CI/badge.svg)](https://github.com/messense/llvm-bitcode-rs/actions?query=workflow%3ACI)
[![codecov](https://codecov.io/gh/messense/llvm-bitcode-rs/branch/master/graph/badge.svg)](https://codecov.io/gh/messense/llvm-bitcode-rs)
[![Crates.io](https://img.shields.io/crates/v/llvm-bitcode.svg)](https://crates.io/crates/llvm-bitcode)
[![docs.rs](https://docs.rs/llvm-bitcode/badge.svg)](https://docs.rs/llvm-bitcode/)

LLVM Bitcode parser in Rust

## Installation

Add it to your ``Cargo.toml``:

```toml
[dependencies]
llvm-bitcode = "0.1"
```

then you are good to go. If you are using Rust 2015 you have to add ``extern crate llvm_bitcode`` to your crate root as well.

## License

This work is released under the MIT license. A copy of the license is provided in the [LICENSE](./LICENSE) file.