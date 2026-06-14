//! Salsa-backed library build entry point (default for `wj build --library`).
//!
//! Uses the incremental framework + optimized multipass pipeline.
//! Set `WJ_LEGACY_MULTIPASS=1` to force the pre-Salsa path during transition.

use crate::compiler::library_multipass::build_library_multipass;
use crate::compiler_database::CompilerDatabase;
use crate::metadata::CrateMetadata;
use crate::CompilationTarget;
use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Build a multi-file library using the Salsa-orchestrated pipeline.
///
/// Currently delegates to the optimized multipass builder with incremental
/// analysis skipping. `CompilerDatabase` is initialized so per-file parse
/// queries share infrastructure with the LSP (future: full per-query analyze).
#[allow(clippy::too_many_arguments)]
pub fn build_library_salsa(
    wj_files: &[PathBuf],
    base_path: &Path,
    output: &Path,
    target: CompilationTarget,
    library: bool,
    enable_lint: bool,
    external_paths: &HashMap<String, PathBuf>,
    crate_metadata: CrateMetadata,
) -> Result<()> {
    let _db = CompilerDatabase::new();
    build_library_multipass(
        wj_files,
        base_path,
        output,
        target,
        library,
        enable_lint,
        external_paths,
        crate_metadata,
    )
}

/// Route library builds: Salsa path by default, legacy multipass on request.
#[allow(clippy::too_many_arguments)]
pub fn build_library(
    wj_files: &[PathBuf],
    base_path: &Path,
    output: &Path,
    target: CompilationTarget,
    library: bool,
    enable_lint: bool,
    external_paths: &HashMap<String, PathBuf>,
    crate_metadata: CrateMetadata,
) -> Result<()> {
    if std::env::var("WJ_LEGACY_MULTIPASS").map_or(false, |v| v == "1" || v == "true") {
        eprintln!("Using legacy multipass build (WJ_LEGACY_MULTIPASS=1)");
        build_library_multipass(
            wj_files,
            base_path,
            output,
            target,
            library,
            enable_lint,
            external_paths,
            crate_metadata,
        )
    } else {
        build_library_salsa(
            wj_files,
            base_path,
            output,
            target,
            library,
            enable_lint,
            external_paths,
            crate_metadata,
        )
    }
}
