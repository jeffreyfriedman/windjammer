//! Effects as facts of the semantic layer (ADR-0021, ADR-0017).
//!
//! * [`kind`]        the effect vocabulary (`fs.read`, `net.connect`, ...)
//! * [`sinks`]       THE table of standard-library functions that perform effects (no other code classifies them)
//! * [`scope_match`] resource matching: paths (canonicalized), hosts, names
//! * [`grant`]       what a package or run may do; parsed from `wj.toml [capabilities]` and `--allow`
//! * [`analysis`]    the pass: per-function effect sets with provenance, over `crate::resolve` call resolutions
//! * [`report`]      the `wj explain-effects` text/JSON
//! * [`packages`]   per-package grants (the application and each dependency)
//! * [`lock`]       `.wj-capabilities.lock`: what dependencies are known to do; escalations
//! * [`policy`]     `[policy]` rules (`forbid`, `only_via`) over the facts
//! * [`violations`] effects against a grant and against `@uses`/`@pure` declarations (WJ0040-0044)
//!
//! Effects never decide ownership or codegen; backends and the interpreter read these facts.

pub mod analysis;
pub mod grant;
pub mod kind;
pub mod lock;
pub mod packages;
pub mod policy;
pub mod report;
pub mod scope_match;
pub mod sinks;
pub mod violations;

pub use analysis::{analyze, EffectFacts, FnEffects};
pub use grant::{grant_from_manifest_text, Denial, Grant, Scope};
pub use kind::EffectKind;
pub use lock::CapLock;
pub use packages::PackageGrants;
