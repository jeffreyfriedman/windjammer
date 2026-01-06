//! Windjammer Runtime Library
//!
//! This crate provides the actual Rust implementations for Windjammer's standard library.
//! When you write `use std::http` in Windjammer, the compiler generates code that calls
//! functions from this crate.

// Platform-specific implementations
pub mod platform;

// Core modules (fully implemented)
pub mod fs;
#[cfg(feature = "server")]
pub mod http;
pub mod json;
pub mod mime;

// Additional stdlib modules
#[cfg(feature = "server")]
pub mod async_runtime;
pub mod cli;
pub mod collections;
pub mod crypto;
pub mod csv_mod;
pub mod db;
pub mod encoding;
pub mod env;
pub mod io;
pub mod log_mod;
pub mod math;
pub mod path;
pub mod process;
pub mod random;
pub mod regex_mod;
pub mod strings;
pub mod sync;
pub mod test;
pub mod testing;
pub mod thread;
pub mod bench;
pub mod property;
pub mod mock;
pub mod test_output;
pub mod timeout;
pub mod time;

// Re-export commonly used types
#[cfg(feature = "server")]
pub use http::{Request, Response, Router, ServerResponse};
