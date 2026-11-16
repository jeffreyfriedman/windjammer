//! Windjammer Runtime Library
//!
//! This crate provides the actual Rust implementations for Windjammer's standard library.
//! When you write `use std::http` in Windjammer, the compiler generates code that calls
//! functions from this crate.

// Re-export rand for property testing
#[cfg(feature = "base")]
pub use rand;

// Platform-specific implementations
#[cfg(feature = "base")]
pub mod platform;

// Core modules (fully implemented)
// `cap` reaches `random` (a capability-checked Rng), so it carries the `base` dependencies too
#[cfg(feature = "base")]
pub mod cap;
pub mod caps;
pub mod caps_core;
#[cfg(feature = "base")]
pub mod durable;
#[cfg(feature = "base")]
pub mod durable_core;
pub mod net_core;
pub mod fs;
#[cfg(feature = "net")]
pub mod http;
#[cfg(feature = "base")]
pub mod json;
#[cfg(feature = "jwt")]
pub mod jwt;
#[cfg(feature = "base")]
pub mod mime;

// Profiling (Tracy zones; optional `tracy` feature)
pub mod profiling;

// Additional stdlib modules
#[cfg(feature = "base")]
pub mod arena;
#[cfg(feature = "async")]
pub mod async_runtime;
pub mod bench;
#[cfg(feature = "cli")]
pub mod cli;
pub mod collections;
pub mod contracts;
#[cfg(feature = "crypto")]
pub mod crypto;
#[cfg(feature = "compress")]
pub mod compress;
#[cfg(feature = "yaml")]
pub mod config;
#[cfg(feature = "csv")]
pub mod csv_mod;
#[cfg(feature = "db")]
pub mod db;
#[cfg(feature = "db")]
pub mod db_pg_decode;
pub mod doc_test;
#[cfg(feature = "base")]
pub mod encoding;
pub mod enumerable;
pub mod env;
pub mod ffi;
pub mod fixtures;
pub mod io;
#[cfg(feature = "log")]
pub mod log_mod;
pub mod marker;
pub mod math;
pub mod mock;
pub mod mock_function;
pub mod mock_interface;
pub mod path;
pub mod process;
#[cfg(feature = "base")]
pub mod property;
#[cfg(feature = "base")]
pub mod random;
#[cfg(feature = "regex")]
pub mod regex_mod;
pub mod setup_teardown;
pub mod strings;
#[cfg(feature = "base")]
pub mod subprocess;
pub mod sync;
pub mod test;
pub mod test_output;
pub mod testing;
pub mod thread;
#[cfg(feature = "datetime")]
pub mod time;
pub mod timeout;
#[cfg(feature = "base")]
pub mod url;
#[cfg(feature = "base")]
pub mod uuid;
#[cfg(feature = "yaml")]
pub mod yaml;

// Re-export commonly used types
#[cfg(feature = "net")]
pub use http::{HttpMethod, Request, Response, Router, Server, ServerRequest, ServerResponse};
#[cfg(feature = "log")]
pub use log_mod::LogLevel;

// Prelude for common imports
pub mod prelude {
    pub use crate::marker::PhantomData;
    pub use crate::testing::*;
}
