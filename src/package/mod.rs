//! Package management foundation (WJ-PKG-01).
//!
//! Provides manifest parsing, dependency resolution stubs, and registry client
//! interfaces for the Windjammer package ecosystem.

pub mod manifest;
pub mod registry;
pub mod resolver;

pub use manifest::{Dependency, PackageManifest};
pub use registry::{PackageMetadata, RegistryClient};
pub use resolver::{ResolutionError, ResolutionResult, ResolvedDependency, Resolver};
