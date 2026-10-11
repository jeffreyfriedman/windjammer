//! Multi-oracle backend conformance system (see docs/backends/CONFORMANCE.md).

#![allow(dead_code)]

pub mod backends;
pub mod compare;
pub mod coverage;
pub mod exec;
pub mod gen_ast;
pub mod gen_build;
pub mod genrun;
pub mod meta;
pub mod mutation;
pub mod oracle;
pub mod parallel;
pub mod program;
pub mod rng;
pub mod runner;
pub mod shrink;
pub mod support;
pub mod toolchain;
