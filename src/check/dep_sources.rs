//! Windjammer dependencies as parsed source, for the effect pass (ADR-0021 per-package grants).
//! Same file selection as the interpreter's dependency loader; files that do not parse are skipped (the
//! dependency's own build reports them).

use crate::module_system::sources::{program_sources, wj_files_under, SourceFile};
use crate::package::graph::{self, DepKind};
use crate::parser::ast::core::Program;
use std::path::{Path, PathBuf};

pub struct DepFile {
    pub module_path: Vec<String>,
    pub file: PathBuf,
    pub program: &'static Program<'static>,
}

pub struct DepSource {
    /// Importable crate name (`-` replaced by `_`); the first segment of every module path below.
    pub name: String,
    /// Pinned revision or source text, for the lock file.
    pub rev: Option<String>,
    pub files: Vec<DepFile>,
}

fn parse(file: &Path) -> Option<&'static Program<'static>> {
    let src = std::fs::read_to_string(file).ok()?;
    let mut lexer = crate::lexer::Lexer::new(&src);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = crate::parser::Parser::new_with_source(tokens, file.display().to_string(), src.clone());
    let program = parser.parse().ok()?;
    Some(Box::leak(Box::new(program)))
}

/// Every Windjammer dependency's source files (module path relative to the dependency, without the crate prefix).
fn dependency_files(manifest_dir: &Path) -> Vec<(graph::DepNode, Vec<SourceFile>)> {
    let Ok(nodes) = graph::resolve(manifest_dir) else { return Vec::new() };
    let mut out = Vec::new();
    for n in nodes.into_iter().filter(|n| n.kind == DepKind::Windjammer) {
        let Some(src) = n.source_root() else { continue };
        let files: Result<Vec<SourceFile>, _> = match n.lib_entry() {
            Some(lib) => program_sources(&src, &lib),
            None => wj_files_under(&src).map(|fs| {
                fs.into_iter()
                    .filter(|f| !crate::test_module_gate::is_test_source_path(f.strip_prefix(&src).unwrap_or(f)))
                    .filter(|f| f.file_name().is_none_or(|n| n != "main.wj"))
                    .map(|f| SourceFile { module_path: crate::resolve::module_path_from_file(&src, &f), file: f })
                    .collect()
            }),
        };
        if let Ok(files) = files {
            out.push((n, files));
        }
    }
    out
}

/// Paths of every dependency source file plus its `wj.toml` (inputs of the check cache key).
pub fn input_paths(manifest_dir: &Path) -> Vec<PathBuf> {
    dependency_files(manifest_dir).into_iter().flat_map(|(_, fs)| fs.into_iter().map(|f| f.file)).collect()
}

pub fn load(manifest_dir: &Path) -> Vec<DepSource> {
    let mut out = Vec::new();
    for (n, files) in dependency_files(manifest_dir) {
        let name = n.crate_name();
        let files = files
            .into_iter()
            .filter_map(|f| {
                let program = parse(&f.file)?;
                let mut module_path = vec![name.clone()];
                module_path.extend(f.module_path);
                Some(DepFile { module_path, file: f.file, program })
            })
            .collect();
        out.push(DepSource { name, rev: n.rev.clone(), files });
    }
    out
}
