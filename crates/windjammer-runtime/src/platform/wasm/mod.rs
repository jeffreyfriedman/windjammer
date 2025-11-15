//! WASM platform implementation
//!
//! This module provides implementations using browser APIs.
//! Used for web applications running in the browser.

#[cfg(target_arch = "wasm32")]
pub mod fs;

#[cfg(target_arch = "wasm32")]
pub mod process;

#[cfg(target_arch = "wasm32")]
pub mod dialog;

#[cfg(target_arch = "wasm32")]
pub mod env;

#[cfg(target_arch = "wasm32")]
pub mod encoding;

#[cfg(target_arch = "wasm32")]
pub mod compute;

#[cfg(target_arch = "wasm32")]
pub mod net;

#[cfg(target_arch = "wasm32")]
pub mod storage;
