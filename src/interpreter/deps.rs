//! Windjammer dependencies loaded as source: the project's `wj.toml` graph (`package::graph`, the same
//! one the compiled build hands to Cargo), each library mounted as a top-level crate module.

use super::error::{ErrorKind, RuntimeError};
use super::project::{parse_files, LoadedFile};
use crate::module_system::sources::{program_sources, wj_files_under, SourceFile};
use crate::package::graph::{self, DepKind};
use std::path::Path;

/// Dependencies of the project that contains `entry`.
#[derive(Default)]
pub struct Dependencies {
    /// `(crate name, files)` for every Windjammer dependency, loaded as source.
    pub windjammer: Vec<(String, Vec<LoadedFile>)>,
    /// Crate names of Rust dependencies (bridged per ADR-0020, never interpreted).
    pub rust: Vec<String>,
}

pub fn dependency_sources(entry: &Path) -> Result<Dependencies, RuntimeError> {
    let Some(root) = crate::module_system::project::find_manifest_dir(entry) else {
        return Ok(Dependencies::default());
    };
    let nodes = graph::resolve(&root).map_err(|e| RuntimeError::new(ErrorKind::Semantic, e))?;
    let mut out = Vec::new();
    let rust = nodes.iter().filter(|n| n.kind == DepKind::Rust).map(|n| n.crate_name()).collect();
    for n in nodes.into_iter().filter(|n| n.kind == DepKind::Windjammer) {
        let Some(src) = n.source_root() else { continue };
        let files: Vec<SourceFile> = match n.lib_entry() {
            Some(lib) => program_sources(&src, &lib),
            None => wj_files_under(&src).map(|fs| {
                fs.into_iter()
                    .filter(|f| !crate::test_module_gate::is_test_source_path(f.strip_prefix(&src).unwrap_or(f)))
                    .filter(|f| f.file_name().is_none_or(|n| n != "main.wj"))
                    .map(|f| SourceFile { module_path: crate::resolve::module_path_from_file(&src, &f), file: f })
                    .collect()
            }),
        }
        .map_err(|e| RuntimeError::new(ErrorKind::Semantic, format!("dependency `{}`: {}", n.name, e)))?;
        out.push((n.crate_name(), parse_files(&files)?));
    }
    Ok(Dependencies { windjammer: out, rust })
}
