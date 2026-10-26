//! Windjammerscript: the Windjammer interpreter (`wj run --interpret`, `wj test --interpret`, `wj repl`).
//!
//! See `MODULE.md` for the architecture and `docs/INTERPRETER.md` for status and design.

mod analysis;
pub mod builtins;
pub mod caps;
pub mod durable_rt;
pub mod effect_port;
mod c_library;
mod call;
mod coerce;
mod defaults;
pub mod deps;
pub mod engine;
pub mod error;
mod eval_expr;
mod eval_stmt;
pub mod features;
mod fasthash;
mod format;
mod from_impls;
mod generics;
mod heap;
pub mod host_bridge;
mod hot;
mod intrinsics;
pub mod int;
mod kmap;
mod iter;
mod locks;
mod moves;
mod native;
pub mod loader;
mod ops;
mod pattern;
mod place;
pub mod preflight;
pub mod project;
mod register;
mod resolved;
pub mod repl;
pub mod runner;
mod sched;
mod scope;
mod slicing;
mod task_blocks;
mod type_keys;
mod u128ops;
pub mod value;

pub use engine::Interpreter;
pub use error::{ErrorKind, RuntimeError};
pub use value::Value;
