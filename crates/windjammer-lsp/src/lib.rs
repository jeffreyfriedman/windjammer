//! Windjammer LSP Library
//!
//! This library exposes the Salsa database for benchmarking and testing.

// TODO(v0.35.0): Fix all clippy warnings properly
// These are temporarily allowed to unblock v0.34.0 release
#![allow(unused_mut)]
#![allow(unused_variables)]
#![allow(dead_code)]
#![allow(clippy::needless_range_loop)]
#![allow(clippy::single_match)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::format_in_format_args)]
#![allow(clippy::redundant_closure)]
#![allow(clippy::iter_nth_zero)]
#![allow(clippy::collapsible_match)]
#![allow(clippy::len_zero)]
#![allow(clippy::absurd_extreme_comparisons)]
#![allow(clippy::manual_range_contains)]

pub mod cache;
pub mod database;
pub mod refactoring;
