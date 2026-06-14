//! Per-file analysis cache backed by `.wj.meta` fingerprints.

use super::build_fingerprint::SourceFingerprint;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Compute the set of file indices that need re-analysis.
///
/// Includes dirty files plus any transitive importers when incremental tracing is enabled.
pub fn compute_reanalysis_set(
    sources: &[(PathBuf, String)],
    src_base: &Path,
    output: &Path,
    dep_roots: &[PathBuf],
    dependency_graph: &super::dependency_graph::DependencyGraph,
) -> HashSet<usize> {
    if !super::build_fingerprint::is_compiler_stamp_fresh(output) {
        return (0..sources.len()).collect();
    }

    let mut dirty = HashSet::new();
    for (i, (file, source)) in sources.iter().enumerate() {
        let output_file = match crate::project_paths::resolve_wj_output_path_library(
            src_base, file, output,
        ) {
            Ok(p) => p,
            Err(_) => {
                dirty.insert(i);
                continue;
            }
        };

        if super::build_fingerprint::is_codegen_cache_valid(source, file, &output_file, dep_roots) {
            // clean — skip reanalysis for this file
        } else {
            dirty.insert(i);
        }
    }

    if std::env::var("WJ_INCREMENTAL_TRACE").map_or(false, |v| v == "1" || v == "true") {
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
