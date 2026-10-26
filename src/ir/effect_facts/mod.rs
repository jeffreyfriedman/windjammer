//! Effects as facts of the semantic layer (ADR-0021, ADR-0017).
//!
//! * [`model`]      the capability model shared with the runtime: [`kind`] (the vocabulary), [`scope_match`] (paths,
//!   hosts, names) and [`grant`] (what a package or run may do); one source, compiled into both crates
//! * [`sinks`]       THE table of standard-library functions that perform effects (no other code classifies them)
//! * [`manifest`]   `wj.toml [capabilities]` -> grant
//! * [`analysis`]    the pass: per-function effect sets with provenance, over `crate::resolve` call resolutions
//! * [`report`]      the `wj explain-effects` text/JSON
//! * [`packages`]   per-package grants (the application and each dependency)
//! * [`install`]    what a compiled program needs: proven vs runtime-guarded, and the `caps::install` statement
//! * [`lock`]       `.wj-capabilities.lock`: what dependencies are known to do; escalations
//! * [`policy`]     `[policy]` rules (`forbid`, `only_via`) over the facts
//! * [`violations`] effects against a grant and against `@uses`/`@pure` declarations (WJ0040-0044)
//!
//! Effects never decide ownership or codegen; backends and the interpreter read these facts.

// The capability model (kind, scope matching, grant) is one source shared with `windjammer-runtime`: see crates/windjammer-runtime/src/caps_core/mod.rs.
#[path = "../../../crates/windjammer-runtime/src/caps_core/mod.rs"]
pub mod model;
pub use self::model::{grant, kind, scope_match};

pub mod analysis;
pub mod durable;
pub mod install;
pub mod lock;
pub mod manifest;
pub mod packages;
pub mod policy;
pub mod report;
pub mod sinks;
pub mod violations;

pub use analysis::{analyze, EffectFacts, FnEffects};
pub use grant::{Denial, Grant, Scope};
pub use manifest::grant_from_manifest_text;
pub use kind::EffectKind;
pub use lock::CapLock;
pub use packages::PackageGrants;
