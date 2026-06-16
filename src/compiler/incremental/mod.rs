//! Incremental compilation framework: dependency graph, fingerprints, reanalysis set.

mod analysis_cache;
mod build_fingerprint;
mod dependency_graph;

pub use analysis_cache::{
    compute_reanalysis_set, fingerprint_for_emit, fingerprint_for_emit_with_dep_epoch,
};
pub use build_fingerprint::{
    compiler_build_identity, compute_fingerprint, compute_fingerprint_with_dep_epoch,
    compute_fingerprint_with_dep_epoch_and_output, compute_fingerprint_with_output,
    dep_metadata_epoch, fingerprint_matches_cached, fingerprint_matches_cached_with_dep_epoch,
    hash_output, is_codegen_cache_valid, is_codegen_cache_valid_with_dep_epoch,
    is_compiler_stamp_fresh, write_compiler_stamp, SourceFingerprint,
};
pub use dependency_graph::DependencyGraph;
