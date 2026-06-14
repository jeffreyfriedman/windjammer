//! Compiler module — builds Windjammer projects for the CLI and integration tests.
//!
//! Submodules split orchestration (`compilation_pipeline`), filesystem and dep metadata
//! (`dependency_resolution`), incremental output handling (`cache_management`), Copy registry
//! (`library_copy_registry`), and the large multipass library path (`library_multipass`).

pub mod cache_management;
mod compilation_pipeline;
mod declaration_stub_registry;
mod dependency_resolution;
pub mod incremental;
mod library_copy_registry;
pub mod library_multipass;
mod salsa_library_build;

pub use cache_management::write_if_changed;
pub use compilation_pipeline::{build_project, build_project_ext};

use crate::parser::ast::core::Item;
use anyhow::Result;

/// Detect whether a parsed program is a GPU shader file (@vertex, @fragment, @compute).
pub fn is_shader_file(program: &crate::parser::Program) -> bool {
    let registry = crate::decorator_registry::DecoratorRegistry::new();
    for item in &program.items {
        if let Item::Function { decl, .. } = item {
            for decorator in &decl.decorators {
                if registry.is_gpu_decorator(&decorator.name) {
                    return true;
                }
            }
        }
    }
    false
}

/// Bail with a formatted error if a type inference pass produced errors.
/// `kind` is "Float" or "Int", `context` is an optional file path for single-file mode.
pub(crate) fn bail_on_inference_errors(
    errors: &[String],
    kind: &str,
    context: Option<&std::path::Path>,
) -> anyhow::Result<()> {
    if errors.is_empty() {
        return Ok(());
    }
    for error in errors {
        if let Some(path) = context {
            eprintln!("{} inference error in {}: {}", kind, path.display(), error);
        } else {
            eprintln!("{} inference error: {}", kind, error);
        }
    }
    let ctx = context
        .map(|p| format!(" in {}", p.display()))
        .unwrap_or_default();
    Err(anyhow::anyhow!(
        "{} type inference failed{}: {} error(s)",
        kind,
        ctx,
        errors.len()
    ))
}

/// Generate Cargo.toml and/or wasm manifest for the output directory.
/// Resolves `source_dir` from `input_path` (file → parent, dir → as-is).
/// When `clean_nested` is true, removes stale nested Cargo.toml files first.
pub(crate) fn generate_cargo_manifests(
    input_path: &std::path::Path,
    output: &std::path::Path,
    target: crate::CompilationTarget,
    clean_nested: bool,
) -> anyhow::Result<()> {
    let source_dir = if input_path.is_file() {
        input_path.parent().unwrap_or(input_path)
    } else {
        input_path
    };
    if target == crate::CompilationTarget::Rust {
        if clean_nested {
            cache_management::clean_nested_cargo_toml(output);
        }
        crate::cargo_toml::generate_single_file_cargo_toml(output, source_dir, target)?;
    }
    if target == crate::CompilationTarget::Wasm {
        crate::cargo_toml::generate_wasm_cargo_toml(output, source_dir)?;
    }
    Ok(())
}

/// Parse a `.wj` source string into a `(Parser, Program)` pair.
///
/// Returns both the parser (which owns the AST arenas) and the program.
/// The caller must keep the `Parser` alive as long as the `Program` is used,
/// since `Program` references borrow from the parser's arenas.
pub(crate) fn parse_wj_source(
    file: &std::path::Path,
    source: &str,
) -> anyhow::Result<(crate::parser::Parser, crate::parser::Program<'static>)> {
    let mut lexer = crate::lexer::Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = crate::parser::Parser::new_with_source(
        tokens,
        file.to_string_lossy().to_string(),
        source.to_string(),
    );
    let program = parser
        .parse()
        .map_err(|e| anyhow::anyhow!("Parse error in {}: {}", file.display(), e))?;
    Ok((parser, program))
}

/// Emit parser warnings/errors to stderr. Returns `Err` if any diagnostic is an error.
pub(crate) fn emit_parser_warnings(parser: &crate::parser::Parser) -> Result<()> {
    let mut has_errors = false;
    for w in parser.warnings() {
        let level = if w.is_error { "error" } else { "warning" };
        eprintln!(
            "{}: {} [{}:{}:{}]",
            level,
            w.message,
            w.file.as_deref().unwrap_or("<unknown>"),
            w.line.unwrap_or(0),
            w.column.unwrap_or(0),
        );
        if w.is_error {
            has_errors = true;
        }
    }
    if has_errors {
        anyhow::bail!("Rust leakage errors detected -- see diagnostics above");
    }
    Ok(())
}

/// Ensure the parent directory of an output file exists.
pub(crate) fn ensure_output_parent_dir(output_file: &std::path::Path) -> anyhow::Result<()> {
    if let Some(parent) = output_file.parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(())
}

/// Generate the final Rust code, apply self-receiver upgrades, write the output,
/// and emit `.wj.meta` metadata when targeting Rust.
#[allow(clippy::too_many_arguments)]
pub(crate) fn write_generated_rust_and_meta<'ast>(
    codegen: &mut crate::codegen::rust::CodeGenerator<'ast>,
    program: &crate::parser::Program<'ast>,
    analyzed_functions: &[crate::analyzer::AnalyzedFunction<'ast>],
    registry_snapshot: &mut crate::analyzer::SignatureRegistry,
    output_file: &std::path::Path,
    source_file: &std::path::Path,
    copy_structs: Vec<String>,
    target: crate::CompilationTarget,
    dep_roots: &[std::path::PathBuf],
    dep_epoch: Option<u64>,
) -> anyhow::Result<()> {
    let rust_code = codegen.generate_program(program, analyzed_functions);
    codegen.apply_self_receiver_upgrades(registry_snapshot);
    cache_management::write_if_changed(output_file, &rust_code)?;
    if target == crate::CompilationTarget::Rust {
        let source = std::fs::read_to_string(source_file)?;
        let fingerprint = Some(if let Some(epoch) = dep_epoch {
            incremental::fingerprint_for_emit_with_dep_epoch(&source, epoch).into()
        } else {
            incremental::fingerprint_for_emit(&source, dep_roots).into()
        });
        crate::metadata::emit_module_meta_for_file_with_fingerprint(
            source_file,
            program,
            registry_snapshot,
            copy_structs,
            fingerprint,
        );
    }
    Ok(())
}

/// Serialize and incrementally write `CrateMetadata` as `metadata.json` in the output dir.
pub(crate) fn write_crate_metadata_json(
    output: &std::path::Path,
    metadata: &crate::metadata::CrateMetadata,
) -> anyhow::Result<()> {
    let metadata_path = output.join("metadata.json");
    let metadata_json = serde_json::to_string_pretty(metadata)?;
    cache_management::write_if_changed(&metadata_path, &metadata_json)?;
    Ok(())
}

/// Collect inferred bounds from a program and apply them to the code generator.
pub(crate) fn apply_inferred_bounds_to_codegen<'ast>(
    codegen: &mut crate::codegen::rust::CodeGenerator<'ast>,
    program: &crate::parser::Program<'ast>,
) {
    let inferred_bounds_map = crate::inference::collect_inferred_bounds(&program.items);
    codegen.set_inferred_bounds(inferred_bounds_map);
}

/// Remove `Item::Mod` entries whose names are in `filtered_modules` before codegen.
pub fn strip_filtered_mod_items<'ast>(
    items: Vec<crate::parser::ast::core::Item<'ast>>,
    filtered_modules: &std::collections::HashSet<String>,
) -> Vec<crate::parser::ast::core::Item<'ast>> {
    if filtered_modules.is_empty() {
        return items;
    }
    items
        .into_iter()
        .filter(|item| {
            if let crate::parser::ast::core::Item::Mod { name, .. } = item {
                !filtered_modules.contains(name)
            } else {
                true
            }
        })
        .collect()
}
