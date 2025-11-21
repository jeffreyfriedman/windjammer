//! Native platform implementation
//!
//! This module provides implementations using native Rust standard library.
//! Used for desktop applications without Tauri.

pub mod compute;
pub mod dialog;
pub mod encoding;
pub mod env;
pub mod fs;
pub mod http;
pub mod net;
pub mod process;
pub mod storage;
