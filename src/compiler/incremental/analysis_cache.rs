//! Per-file analysis cache backed by `.wj.meta` fingerprints.

use super::build_fingerprint::SourceFingerprint;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Compute the set of file indices that need re-analysis.
///
/// Includes dirty files plus any transitive importers when incremental tracing is enabled.
/// Uses `dep_epoch` for fingerprint comparison to match the epoch written during codegen.
pub fn compute_reanalysis_set(
    sources: &[(PathBuf, String)],
    src_base: &Path,
    output: &Path,
    dep_epoch: u64,
    dependency_graph: &super::dependency_graph::DependencyGraph,
) -> HashSet<usize> {
    if !super::build_fingerprint::is_compiler_stamp_fresh(output) {
        return (0..sources.len()).collect();
    }

    let mut dirty = HashSet::new();
    for (i, (file, source)) in sources.iter().enumerate() {
        // Skip out-of-tree sources (e.g. compiler stdlib injections) — they are
        // analysis-only and have no codegen output to validate.
        if file.strip_prefix(src_base).is_err() {
            continue;
        }

        let output_file =
            match crate::project_paths::resolve_wj_output_path_library(src_base, file, output) {
                Ok(p) => p,
                Err(_) => {
                    dirty.insert(i);
                    continue;
                }
            };

        // Use the library-aware cache check that handles mod.wj → mod.rs merging.
        if crate::compiler::cache_management::is_library_codegen_cache_valid_with_dep_epoch(
            source,
            file,
            &output_file,
            src_base,
            output,
            dep_epoch,
        ) {
            // clean — skip reanalysis for this file
        } else {
            dirty.insert(i);
        }
    }

    if std::env::var("WJ_INCREMENTAL_TRACE").is_ok_and(|v| v == "1" || v == "true") {
        eprintln!(
            "[wj-incremental] {} direct dirty files before dependents",
            dirty.len()
        );
    }

    dependency_graph.transitive_dependents(&dirty)
}

pub fn fingerprint_for_emit(source: &str, dep_roots: &[PathBuf]) -> SourceFingerprint {
    super::build_fingerprint::compute_fingerprint(source, dep_roots)
}

pub fn fingerprint_for_emit_with_dep_epoch(source: &str, dep_epoch: u64) -> SourceFingerprint {
    super::build_fingerprint::compute_fingerprint_with_dep_epoch(source, dep_epoch)
}
