//! Package management foundation (WJ-PKG-01).
//!
//! Manifest parsing for `wj.toml`. Dependency resolution and a registry client are design-only
//! (ADR-0010, WJ-PKG-01): the earlier stubs made no network calls and were removed.

pub mod graph;
pub mod path_deps;
pub mod manifest;

pub use manifest::{Dependency, PackageManifest};
