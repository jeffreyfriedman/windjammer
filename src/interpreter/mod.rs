//! Windjammerscript: the Windjammer interpreter (`wj run --interpret`, `wj test --interpret`, `wj repl`).
//!
//! See `MODULE.md` for the architecture and `docs/INTERPRETER.md` for status and design.

mod analysis;
pub mod builtins;
mod call;
mod coerce;
mod defaults;
pub mod engine;
pub mod error;
mod eval_expr;
mod eval_stmt;
pub mod features;
mod fasthash;
mod format;
mod generics;
mod heap;
mod intrinsics;
pub mod int;
mod iter;
mod locks;
mod moves;
mod native;
pub mod loader;
mod ops;
mod pattern;
mod place;
mod register;
mod resolved;
pub mod repl;
pub mod runner;
mod sched;
mod scope;
mod slicing;
mod u128ops;
pub mod value;

pub use engine::Interpreter;
pub use error::{ErrorKind, RuntimeError};
pub use value::Value;
