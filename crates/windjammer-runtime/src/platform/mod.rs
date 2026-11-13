//! Platform-specific implementations
//!
//! This module provides platform-specific implementations of Windjammer's standard library.
//! The compiler automatically selects the appropriate platform based on the compilation target.

#[cfg(not(target_arch = "wasm32"))]
pub mod native;

// Browser bindings need the `wasm` feature (wasm-bindgen, web-sys); a wasm32 build without it
// (the engine core, `default-features = false`) has no platform module.
#[cfg(all(target_arch = "wasm32", feature = "wasm"))]
pub mod wasm;
