//! Diagnostics as data: one schema for the front end, lints, the rustc mapper and the IR
//! solvers, consumed by `wj check/build/test --json`, the LSP and human rendering.
//! See `MODULE.md` and `docs/DIAGNOSTICS.md`.

pub mod apply;
pub mod codes;
mod codes_errors;
mod codes_lints;
pub mod convert;
pub mod model;
pub mod render;
pub mod rustc_map;
pub mod rustc_text;
pub mod source;

pub use model::{
    Applicability, Category, Diagnostic, Docs, Edit, Fix, Origin, Report, Severity, Span, Summary,
    SCHEMA_VERSION,
};
pub use source::SourceIndex;
