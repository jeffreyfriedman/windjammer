//! Windjammer Runtime Library
//!
//! This crate provides the actual Rust implementations for Windjammer's standard library.
//! When users write `use std.http` in Windjammer, the compiler transpiles this to
//! `use windjammer_runtime::http::*`, making these implementations available.

pub mod fs;
pub mod http;
pub mod mime;

// Re-export commonly used types
pub use fs::*;
pub use http::*;
pub use mime::*;

