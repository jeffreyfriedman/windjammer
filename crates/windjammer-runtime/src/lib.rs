//! Windjammer Runtime Library
//!
//! This crate provides the actual Rust implementations for Windjammer's standard library.
//! When you write `use std::http` in Windjammer, the compiler generates code that calls
//! functions from this crate.

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
pub mod game;
pub mod io;
pub mod log_mod;
pub mod math;
pub mod path;
pub mod process;
pub mod random;
pub mod regex_mod;
pub mod strings;
pub mod sync;
pub mod testing;
pub mod thread;
pub mod time;
pub mod ui;

// Re-export commonly used types
pub use game::{EntityId, Game, Mat4, Mesh, Sprite, Transform, Vec2, Vec3, Velocity, World};
#[cfg(feature = "server")]
pub use http::{Request, Response, Router, ServerResponse};
pub use ui::{VComponent, VElement, VNode, VText};
