//! Single-file and multipass project build entry points.

use crate::analyzer::{Analyzer, SignatureRegistry};
use crate::codegen::rust::CodeGenerator;
use crate::lexer::Lexer;
use crate::linter::rust_leakage::RustLeakageLinter;
use crate::metadata::{
    CrateMetadata, FunctionSignature,
    ModuleMetadata,
};
use crate::parser::ast::core::Item;
use crate::parser::ast::types::Type;
use crate::parser::Parser;
use crate::type_inference::{FloatInference, IntInference};
use crate::CompilationTarget;
use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use super::cache_management::write_if_changed;
use super::dependency_resolution::{find_dependency_metadata_roots, find_wj_files};
use super::library_multipass::build_library_multipass;

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
        return build_library_multipass(
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

    for file in &wj_files {
        let source = std::fs::read_to_string(file)?;
        let mut lexer = Lexer::new(&source);
        let tokens = lexer.tokenize_with_locations();
        let mut parser =
            Parser::new_with_source(tokens, file.to_string_lossy().to_string(), source.clone());
        let program = parser
            .parse()
            .map_err(|e| anyhow::anyhow!("Parse error: {}", e))?;

        for w in parser.warnings() {
            eprintln!(
                "warning: {} [{}:{}:{}]",
                w.message,
                w.file.as_deref().unwrap_or("<unknown>"),
                w.line.unwrap_or(0),
                w.column.unwrap_or(0),
            );
        }

        if library {
            let mut module_meta = ModuleMetadata::new(
                file.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string(),
            );
            for item in &program.items {
                match item {
                    Item::Struct { decl, .. } => {
                        let mut fields = HashMap::new();
                        for field in &decl.fields {
                            fields.insert(
                                field.name.clone(),
                                ModuleMetadata::serialize_type(&field.field_type),
                            );
                        }
                        module_meta.structs.insert(decl.name.clone(), fields);
                    }
                    Item::Function { decl, .. } => {
                        module_meta.functions.insert(
                            decl.name.clone(),
                            FunctionSignature {
                                params: decl
                                    .parameters
                                    .iter()
                                    .map(|p| ModuleMetadata::serialize_type(&p.type_))
                                    .collect(),
                                return_type: decl
                                    .return_type
                                    .as_ref()
                                    .map(ModuleMetadata::serialize_type),
                                is_associated: false,
                                parent_type: None,
                                param_ownership: vec![],
                                has_self_receiver: false,
                                is_extern: decl.is_extern,
                            },
                        );
                    }
                    Item::Impl { block, .. } => {
                        for func_decl in &block.functions {
                            let full_name = format!("{}::{}", block.type_name, func_decl.name);
                            module_meta.functions.insert(
                                full_name,
                                FunctionSignature {
                                    params: func_decl
                                        .parameters
                                        .iter()
                                        .map(|p| ModuleMetadata::serialize_type(&p.type_))
                                        .collect(),
                                    return_type: func_decl
                                        .return_type
                                        .as_ref()
                                        .map(ModuleMetadata::serialize_type),
                                    is_associated: true,
                                    parent_type: Some(block.type_name.clone()),
                                    param_ownership: vec![],
                                    has_self_receiver: false,
                                    is_extern: false,
                                },
                            );
                        }
                    }
                    _ => {}
                }
            }
            crate_metadata.merge_module(&module_meta);
        }

        let mut analyzer = Analyzer::new();
        analyzer
            .check_forbidden_rust_patterns(&program)
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        if enable_lint {
            let file_name = file.to_string_lossy().to_string();
            let mut rust_leakage = RustLeakageLinter::new(&file_name);
            rust_leakage.lint_program(&program);
            for diag in rust_leakage.diagnostics() {
                eprintln!("{}", diag);
            }
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
            .infer_trait_signatures_from_impls(&program)
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        // Update registry with refined trait method signatures from impl inference.
        // infer_trait_signatures_from_impls may upgrade trait methods (e.g. &self → &mut self)
        // based on impl bodies. Propagate these to the registry so .wj.meta has correct data.
        for (trait_name, methods) in &analyzer.analyzed_trait_methods {
            for (method_name, analyzed_func) in methods {
                let sig = analyzer.build_signature(analyzed_func);
                let qualified_name = format!("{}::{}", trait_name, method_name);
                registry.add_function(qualified_name, sig);
            }
        }

        let mut float_inference = FloatInference::new();
        if !external_paths.is_empty() {
            float_inference.set_external_crate_metadata_paths(&external_paths);
        }
        float_inference.infer_program(&program);
        if !float_inference.errors.is_empty() {
            for error in &float_inference.errors {
                eprintln!("Float inference error in {}: {}", file.display(), error);
            }
            return Err(anyhow::anyhow!(
                "Float type inference failed in {}: {} error(s)",
                file.display(),
                float_inference.errors.len()
            ));
        }

        let mut int_inference = IntInference::new();
        int_inference.infer_program(&program);
        if !int_inference.errors.is_empty() {
            for error in &int_inference.errors {
                eprintln!("Int inference error in {}: {}", file.display(), error);
            }
            return Err(anyhow::anyhow!(
                "Int type inference failed in {}: {} error(s)",
                file.display(),
                int_inference.errors.len()
            ));
        }

        let inferred_bounds_map = crate::inference::collect_inferred_bounds(&program.items);

        let mut registry_snapshot = registry.clone();

        let cross_crate_field_types =
            crate::metadata::load_merged_external_struct_fields(&external_paths, None);

        let mut codegen = CodeGenerator::new(registry, target);
        codegen.set_source_file(file);
        codegen.set_analyzed_trait_methods(analyzer.analyzed_trait_methods.clone());
        codegen.set_float_inference(float_inference);
        codegen.set_int_inference(int_inference);
        codegen.set_inferred_bounds(inferred_bounds_map);
        if !cross_crate_field_types.is_empty() {
            codegen.set_global_struct_field_types(cross_crate_field_types);
        }
        let rust_code = codegen.generate_program(&program, &analyzed_functions);
        codegen.apply_self_receiver_upgrades(&mut registry_snapshot);

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
            if let Some(parent) = output_file.parent() {
                std::fs::create_dir_all(parent)?;
            }
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
                        if let Some(parent) = p.parent() {
                            std::fs::create_dir_all(parent)?;
                        }
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
        write_if_changed(&output_file, &rust_code)?;

        if target == CompilationTarget::Rust {
            crate::metadata::emit_module_meta_for_file(
                file, &program, &registry_snapshot, analyzer.get_copy_structs(),
            );
        }
    }

    if library && (!crate_metadata.structs.is_empty() || !crate_metadata.functions.is_empty()) {
        let metadata_path = output.join("metadata.json");
        let metadata_json = serde_json::to_string_pretty(&crate_metadata)?;
        write_if_changed(&metadata_path, &metadata_json)?;
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

    if target == CompilationTarget::Rust {
        let source_dir = if path.is_file() {
            path.parent().unwrap_or(path)
        } else {
            path
        };
        crate::cargo_toml::generate_single_file_cargo_toml(output, source_dir, target)?;
    }

    if target == CompilationTarget::Wasm {
        let source_dir = if path.is_file() {
            path.parent().unwrap_or(path)
        } else {
            path
        };
        crate::cargo_toml::generate_wasm_cargo_toml(output, source_dir)?;
    }

    Ok(())
}
