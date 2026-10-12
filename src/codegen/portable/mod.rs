//! Portable backend support shared by the Go and JS/TS emitters.
//!
//! Pipeline: parser AST -> `lower` (typing, ownership/copy insertion) -> `flatten`
//! (expression-position control flow becomes statements) -> target emitter.

pub mod builtins;
pub mod enumerable;
pub mod env;
pub mod flatten;
pub mod ir;
pub mod lower;
pub mod stdlib;
pub mod ty;
pub mod visit;

pub use ir::{PModule, PResult, Unsupported};

#[cfg(test)]
mod tests;
