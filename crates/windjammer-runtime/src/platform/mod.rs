//! Platform-specific implementations
//!
//! This module provides platform-specific implementations of Windjammer's standard library.
//! The compiler automatically selects the appropriate platform based on the compilation target.

#[cfg(not(target_arch = "wasm32"))]
pub mod native;

#[cfg(target_arch = "wasm32")]
pub mod wasm;
