//! Windjammerscript: Tree-Walking Interpreter
//!
//! Evaluates Windjammer AST directly, without code generation.
//! Any `.wj` file that runs in the interpreter also compiles to Rust
//! via the standard backend — same source, two execution modes:
//!
//! - `wj run --interpret file.wj`  → instant execution, fast iteration
//! - `wj build --target rust file.wj` → production build, memory safe
//!
//! The interpreter uses reference semantics internally (no ownership tracking)
//! because safety is the compiler's job, not the interpreter's.

pub mod value;
pub mod environment;
pub mod engine;

pub use engine::Interpreter;
pub use value::Value;
pub use environment::Environment;
