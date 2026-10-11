//! Span and diagnostic-location oracle gates. See docs/testing/INVARIANTS.md.
//!
//! Run: `cargo test --release --test span_oracle_suite`

#[path = "common/span_oracle/mod.rs"]
mod span_oracle;

#[path = "common/span_oracle/lexer_gate.rs"]
mod lexer_gate;

#[path = "common/span_oracle/ast_gate.rs"]
mod ast_gate;

#[path = "common/span_oracle/diag_gate.rs"]
mod diag_gate;

#[path = "common/span_oracle/sourcemap_gate.rs"]
mod sourcemap_gate;

#[path = "common/span_oracle/misc_gate.rs"]
mod misc_gate;
