//! Windjammerscript: the Windjammer interpreter (`wj run --interpret`, `wj test --interpret`, `wj repl`).
//!
//! See `MODULE.md` for the architecture and `docs/INTERPRETER.md` for status and design.

mod analysis;
pub mod builtins;
pub mod caps;
// The `durable fn` journal is one source shared with `windjammer-runtime`: see crates/windjammer-runtime/src/durable_core/mod.rs.
#[path = "../../crates/windjammer-runtime/src/durable_core/mod.rs"]
pub mod durable_core;
mod host_api;
// Raw sockets (ADR-0032): one source shared with `windjammer-runtime` (crates/windjammer-runtime/src/net_core/mod.rs).
#[path = "../../crates/windjammer-runtime/src/net_core/mod.rs"]
pub(crate) mod net_core;
pub mod debugger;
pub mod durable_rt;
pub mod limits;
pub mod mcp_host;
pub mod effect_port;
mod c_library;
mod call;
mod coerce;
pub mod crate_host;
mod compiled_facts;
mod defaults;
pub mod deps;
pub mod engine;
pub mod error;
mod eval_expr;
mod eval_stmt;
pub mod features;
pub mod error_report;
mod failure_locals;
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
pub mod vecbody;

pub use engine::Interpreter;
pub use error::{ErrorKind, RuntimeError};
pub use value::Value;
