//! Windjammer Runtime Library
//!
//! This crate provides the actual Rust implementations for Windjammer's standard library.
//! When you write `use std::http` in Windjammer, the compiler generates code that calls
//! functions from this crate.

pub mod fs;
pub mod http;
pub mod json;
pub mod mime;

// Re-export commonly used types
pub use http::{Request, Response, Router, ServerResponse};
