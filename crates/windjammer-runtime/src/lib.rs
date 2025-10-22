//! Windjammer Runtime Library
//!
//! This crate provides the actual Rust implementations for Windjammer's standard library.
//! When you write `use std::http` in Windjammer, the compiler generates code that calls
//! functions from this crate.

// Core modules (fully implemented)
pub mod fs;
pub mod http;
pub mod json;
pub mod mime;

// Additional stdlib modules
pub mod async_runtime;
pub mod cli;
pub mod collections;
pub mod crypto;
pub mod csv_mod;
pub mod db;
pub mod encoding;
pub mod env;
pub mod log_mod;
pub mod math;
pub mod process;
pub mod random;
pub mod regex_mod;
pub mod strings;
pub mod testing;
pub mod time;
pub mod ui;

// Re-export commonly used types
pub use http::{Request, Response, Router, ServerResponse};
pub use ui::{VComponent, VElement, VNode, VText};
