//! Incremental compilation framework: dependency graph, fingerprints, reanalysis set.

mod analysis_cache;
mod build_fingerprint;
mod dependency_graph;

pub use analysis_cache::{compute_reanalysis_set, fingerprint_for_emit};
pub use build_fingerprint::{
    compiler_build_identity, compute_fingerprint, fingerprint_matches_cached,
    is_codegen_cache_valid, is_compiler_stamp_fresh, write_compiler_stamp, SourceFingerprint,
};
pub use dependency_graph::DependencyGraph;
