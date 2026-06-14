//! Single-file and multipass project build entry points.

use crate::analyzer::{Analyzer, SignatureRegistry};
use crate::codegen::rust::CodeGenerator;
use crate::metadata::CrateMetadata;
use crate::parser::ast::core::Item;
use crate::parser::ast::types::Type;
use crate::type_inference::{FloatInference, IntInference};
use crate::CompilationTarget;
use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use super::cache_management::write_if_changed;
use super::dependency_resolution::{find_dependency_metadata_roots, find_wj_files};
use super::salsa_library_build::build_library;

/// Check if a Type is Copy in single-file context (overrides stale metadata Copy).
fn is_type_copy_for_single_file_build(ty: &Type, analyzer: &Analyzer) -> bool {
    match ty {
        Type::Int | Type::Int32 | Type::Uint | Type::Float | Type::Bool => true,
        Type::String => false,
        Type::Vec(_) => false,
        Type::Array(inner, _) => is_type_copy_for_single_file_build(inner, analyzer),
        Type::Custom(name) | Type::Generic(name) => {
            crate::type_classification::is_copy_primitive(name) || analyzer.is_copy_struct(name)
        }
        Type::Option(inner) => is_type_copy_for_single_file_build(inner, analyzer),
        Type::Result(ok, err) => {
            is_type_copy_for_single_file_build(ok, analyzer)
                && is_type_copy_for_single_file_build(err, analyzer)
        }
        Type::Parameterized(name, _) => name != "Vec" && name != "HashMap",
        Type::Tuple(types) => types
            .iter()
            .all(|t| is_type_copy_for_single_file_build(t, analyzer)),
        Type::FunctionPointer { .. } => false,
        Type::Reference(_) | Type::MutableReference(_) => true,
        Type::RawPointer { .. } => true,
        _ => false,
    }
}

/// Build a Windjammer project - compiles .wj files to Rust.
pub fn build_project(
    path: &Path,
    output: &Path,
    target: CompilationTarget,
    enable_lint: bool,
) -> Result<()> {
    build_project_ext(path, output, target, enable_lint, false, &[])
}

/// Extended build with library mode and external crate metadata.
pub fn build_project_ext(
    path: &Path,
    output: &Path,
    target: CompilationTarget,
    enable_lint: bool,
    library: bool,
    external_metadata: &[(&str, &Path)],
) -> Result<()> {
    let wj_files = find_wj_files(path)?;
    if wj_files.is_empty() {
        return Ok(());
    }
    std::fs::create_dir_all(output)?;

    let external_paths: HashMap<String, PathBuf> = external_metadata
        .iter()
        .map(|(name, p)| (name.replace('-', "_"), (*p).to_path_buf()))
        .collect();

    let mut crate_metadata = CrateMetadata::new();

    let base_path = if path.is_file() {
        path.parent().unwrap_or(path)
    } else {
        path
    };
    let has_nested_structure = wj_files.iter().any(|f| {
        f.strip_prefix(base_path)
            .map(|r| r.parent().is_some())
            .unwrap_or(false)
    });
    if wj_files.len() > 1 || (library && has_nested_structure) {
        return build_library(
            &wj_files,
            path,
            output,
            target,
            library,
            enable_lint,
            &external_paths,
            crate_metadata,
        );
    }

    let mut deferred_lint_errors: Vec<String> = Vec::new();

    for file in &wj_files {
        let source = std::fs::read_to_string(file)?;
        let (_parser, program) = super::parse_wj_source(file, &source)?;
        if let Err(e) = super::emit_parser_warnings(&_parser) {
            deferred_lint_errors.push(format!("{}", e));
        }

        if library {
            crate::metadata::merge_file_skeleton_into_crate(&mut crate_metadata, file, &program);
        }

        let mut analyzer = Analyzer::new();
        analyzer
            .check_forbidden_rust_patterns(&program)
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        if let Err(e) =
            crate::linter::rust_leakage::run_lint_if_enabled(enable_lint, file, &program)
        {
            deferred_lint_errors.push(e);
        }

        let mut global_signatures = SignatureRegistry::new();
        let file_parent = file.parent().unwrap_or(Path::new("."));
        let mut meta_roots: Vec<&Path> = vec![file_parent];
        let ancestor_roots = find_dependency_metadata_roots(file_parent, &external_paths);
        let ancestor_root_refs: Vec<&Path> = ancestor_roots.iter().map(|p| p.as_path()).collect();
        meta_roots.extend_from_slice(&ancestor_root_refs);
        crate::metadata::merge_wj_meta_signatures_and_copy_structs_multi(
            &meta_roots,
            &mut global_signatures,
            &mut analyzer,
        );

        for item in &program.items {
            if let Item::Struct { decl, .. } = item {
                let struct_name = &decl.name;

                if analyzer.is_copy_struct(struct_name) {
                    let is_local_copy = decl.fields.is_empty()
                        || decl
                            .fields
                            .iter()
                            .all(|f| is_type_copy_for_single_file_build(&f.field_type, &analyzer));

                    if !is_local_copy {
                        analyzer.unregister_copy_struct(struct_name);
                    }
                }
            }
        }

        let (_, first_pass_registry, _) = analyzer
            .analyze_program_with_global_signatures(&program, &global_signatures)
            .map_err(|e| anyhow::anyhow!("First-pass analysis error: {}", e))?;

        global_signatures.merge(&first_pass_registry);

        let copy_structs_list = analyzer.get_copy_structs();
        let copy_structs_set: HashSet<String> = copy_structs_list.into_iter().collect();

        let mut analyzer_pass2 = Analyzer::new_with_copy_structs(copy_structs_set);
        analyzer_pass2
            .check_forbidden_rust_patterns(&program)
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        let (analyzed_functions, mut registry, _) = analyzer_pass2
            .analyze_program_with_global_signatures(&program, &global_signatures)
            .map_err(|e| anyhow::anyhow!("Second-pass analysis error: {}", e))?;

        let mut analyzer = analyzer_pass2;

        analyzer
            .infer_trait_signatures_from_impls(&program, &registry)
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        analyzer
            .register_trait_methods_in_registry(&analyzer.analyzed_trait_methods, &mut registry);

        let mut float_inference = FloatInference::new();
        if !external_paths.is_empty() {
            float_inference.set_external_crate_metadata_paths(&external_paths);
        }
        float_inference.infer_program(&program);
        super::bail_on_inference_errors(&float_inference.errors, "Float", Some(file))?;

        let mut int_inference = IntInference::new();
        int_inference.infer_program(&program);
        super::bail_on_inference_errors(&int_inference.errors, "Int", Some(file))?;

        let mut registry_snapshot = registry.clone();

        let cross_crate_field_types =
            crate::metadata::load_merged_external_struct_fields(&external_paths, None);

        let mut codegen = CodeGenerator::new(registry, target);
        codegen.set_source_file(file);
        codegen.set_analyzed_trait_methods(analyzer.analyzed_trait_methods.clone());
        codegen.set_float_inference(float_inference);
        codegen.set_int_inference(int_inference);
        super::apply_inferred_bounds_to_codegen(&mut codegen, &program);
        if !cross_crate_field_types.is_empty() {
            codegen.set_global_struct_field_types(cross_crate_field_types);
        }

        let output_file = if wj_files.len() > 1 && library {
            let base_path = if path.is_file() {
                path.parent().unwrap_or(path)
            } else {
                path
            };
            let src_base =
                std::fs::canonicalize(base_path).unwrap_or_else(|_| base_path.to_path_buf());
            let output_file =
                crate::project_paths::resolve_wj_output_path(&src_base, file, output)?;
            super::ensure_output_parent_dir(&output_file)?;
            output_file
        } else if wj_files.len() == 1 {
            let flat_rs = || {
                let stem = file
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("output");
                output.join(format!("{}.rs", stem))
            };
            if let Some(root) = crate::project_paths::find_source_root(file) {
                match crate::project_paths::get_relative_output_path(root, file, output) {
                    Ok(p) => {
                        super::ensure_output_parent_dir(&p)?;
                        p
                    }
                    Err(_) => flat_rs(),
                }
            } else {
                flat_rs()
            }
        } else {
            let stem = file
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("output");
            output.join(format!("{}.rs", stem))
        };
        super::write_generated_rust_and_meta(
            &mut codegen,
            &program,
            &analyzed_functions,
            &mut registry_snapshot,
            &output_file,
            file,
            analyzer.get_copy_structs(),
            target,
            &ancestor_roots,
            None,
        )?;
    }

    if library && (!crate_metadata.structs.is_empty() || !crate_metadata.functions.is_empty()) {
        super::write_crate_metadata_json(output, &crate_metadata)?;
    }

    let mod_rs_path = output.join("mod.rs");
    let lib_rs_path = output.join("lib.rs");
    if mod_rs_path.exists() {
        let content = std::fs::read_to_string(&mod_rs_path)?;
        let cleaned: String = content
            .lines()
            .filter(|line| {
                let t = line.trim();
                t != "use super::*;" && t != "#[allow(unused_imports)]"
            })
            .collect::<Vec<&str>>()
            .join("\n");
        write_if_changed(&lib_rs_path, &(cleaned + "\n"))?;
    }

    super::generate_cargo_manifests(path, output, target, false)?;

    let _ = super::cache_management::write_compiler_stamp(output);

    if !deferred_lint_errors.is_empty() {
        return Err(anyhow::anyhow!(
            "Rust leakage errors:\n{}",
            deferred_lint_errors.join("\n")
        ));
    }

    Ok(())
}
