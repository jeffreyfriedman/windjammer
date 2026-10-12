//! Conservative AST-level semantic checks that need no type checker. See `check/MODULE.md`.

mod checker;
mod index;
mod mismatch;
mod rules;
mod std_methods;
mod visit;

pub use checker::{check_file, FileInput};
pub use index::Index;
