//! Windjammer Runtime Library
//!
//! This crate provides the actual Rust implementations for Windjammer's standard library.
//! When you write `use std::http` in Windjammer, the compiler generates code that calls
//! functions from this crate.

// Re-export rand for property testing
pub use rand;

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
pub mod bench;
pub mod cli;
pub mod collections;
pub mod contracts;
pub mod crypto;
pub mod csv_mod;
pub mod db;
pub mod doc_test;
pub mod encoding;
pub mod env;
pub mod fixtures;
pub mod io;
pub mod log_mod;
pub mod math;
pub mod mock;
pub mod mock_function;
pub mod mock_interface;
pub mod path;
pub mod process;
pub mod property;
pub mod random;
pub mod regex_mod;
pub mod setup_teardown;
pub mod strings;
pub mod sync;
pub mod test;
pub mod test_output;
pub mod testing;
pub mod thread;
pub mod time;
pub mod timeout;

// Re-export commonly used types
#[cfg(feature = "server")]
pub use http::{Request, Response, Router, ServerResponse};
