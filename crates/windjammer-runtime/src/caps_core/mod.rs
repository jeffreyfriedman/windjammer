//! The capability model shared by the compiler and the runtime (ADR-0021).
//!
//! This directory is compiled twice, from this one source: as `windjammer_runtime::caps_core` (what a compiled
//! program checks at run time) and as `windjammer::ir::effect_facts::model` (what the interpreter and `wj check`
//! check). There is no second implementation of path, host or name matching to drift; the conformance vectors
//! in `tests/effects/vectors.json` pin the behaviour for the ports that cannot share Rust (Go, JS).
//!
//! Only `std` is used here, so the file builds in both crates without new dependencies.

pub mod grant;
pub mod kind;
pub mod scope_match;
