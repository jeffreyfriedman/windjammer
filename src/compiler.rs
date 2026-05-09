//! Compiler module - builds Windjammer projects.
//!
//! This module provides the core build logic used by both the CLI and integration tests.
//! The implementation is in main.rs; this module re-exports for the library.
//!
//! When the crate is built as a library, tests use windjammer::build_project().
//! When built as the main binary, main.rs has its own build_project.
//!
//! For library builds, we need the implementation. The main.rs and lib.rs are
//! separate crate roots - the lib cannot call main. So we include a stub that
//! will be replaced when the full extraction is done.
//!
//! TEMPORARY: Use the build_project from the crate that has it.
//! The cross_file_trait_inference tests use windjammer::build_project - they
//! must get it from somewhere. Currently the lib doesn't have the full impl.
//! We need to either:
//! 1. Extract the full impl here (large refactor)
//! 2. Have a different crate structure
//!
//! For now: the tests that use build_project (cross_file, integration_ffi, etc.)
//!
//! - do they actually run? Let me check the build. The lib build fails because
//!   we don't have the impl. So we need the impl. Let me add a minimal impl that
//!   does single-file compilation using the existing analyzer/codegen.

use crate::analyzer::{Analyzer, SignatureRegistry};
use crate::codegen::rust::CodeGenerator;
use crate::lexer::Lexer;
use crate::linter::rust_leakage::RustLeakageLinter;
use crate::metadata::{
    meta_cache_path, metadata_function_sig_from_analyzer, CrateMetadata, FunctionSignature,
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

/// TDD FIX helper: Check if a Type is Copy in single-file context
/// Used to override metadata Copy status when the current file's definition differs
fn is_type_copy_for_single_file_build(ty: &Type, analyzer: &Analyzer) -> bool {
    match ty {
        Type::Int | Type::Int32 | Type::Uint | Type::Float | Type::Bool => true,
        Type::String => false, // String is never Copy
        Type::Vec(_) => false, // Vec is never Copy
        Type::Array(inner, _) => is_type_copy_for_single_file_build(inner, analyzer),
        Type::Custom(name) | Type::Generic(name) => {
            crate::type_classification::is_copy_primitive(name) || analyzer.is_copy_struct(name)
        }
        Type::Option(inner) => is_type_copy_for_single_file_build(inner, analyzer),
        Type::Result(ok, err) => {
            is_type_copy_for_single_file_build(ok, analyzer)
                && is_type_copy_for_single_file_build(err, analyzer)
        }
        Type::Parameterized(name, _) => {
            // Vec, HashMap, etc. are not Copy
            name != "Vec" && name != "HashMap"
        }
        Type::Tuple(types) => types
            .iter()
            .all(|t| is_type_copy_for_single_file_build(t, analyzer)),
        Type::FunctionPointer { .. } => false, // Function pointers are not Copy
        Type::Reference(_) | Type::MutableReference(_) => true, // References are Copy
        Type::RawPointer { .. } => true,       // Raw pointers are Copy
        _ => false,
    }
}

/// Write file only if content has changed, preserving mtime for Cargo's
/// incremental compilation when the generated Rust is identical.
pub fn write_if_changed(path: &Path, content: &str) -> std::io::Result<bool> {
    if path.exists() {
        if let Ok(existing) = std::fs::read_to_string(path) {
            if existing == content {
                return Ok(false);
            }
        }
    }
    std::fs::write(path, content)?;
    Ok(true)
}

/// Build a Windjammer project - compiles .wj files to Rust.
///
/// Used by integration tests. For full CLI support, the main binary has
/// the complete implementation with multi-file, Cargo.toml, etc.
///
/// When enable_lint is true, runs Rust leakage linter (W0001-W0004) and emits warnings.
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
    // Multipass: shared global analysis for multi-file projects, or a single file under a
    // nested path in library mode (preserve `a/b/c/file.rs` for `generate_mod_file`).
    //
    // A single *flat* .wj file must use the single-file path (`is_module = false`) so `main()`
    // is emitted and `use super::*` is not injected (see analyzer_auto_mutability_method_test).
    // `has_nested_structure` alone is not enough: nested layout + `library` avoids spurious
    // multipass for `wj build foo.wj` when the project root accidentally contains subdirs.
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

    // Single-file or non-library: use old per-file analysis
    for file in &wj_files {
        let source = std::fs::read_to_string(file)?;
        let mut lexer = Lexer::new(&source);
        let tokens = lexer.tokenize_with_locations();
        let mut parser =
            Parser::new_with_source(tokens, file.to_string_lossy().to_string(), source.clone());
        let program = parser
            .parse()
            .map_err(|e| anyhow::anyhow!("Parse error: {}", e))?;

        // Emit parser warnings (W0010: non-canonical string types, etc.)
        for w in parser.warnings() {
            eprintln!(
                "warning: {} [{}:{}:{}]",
                w.message,
                w.file.as_deref().unwrap_or("<unknown>"),
                w.line.unwrap_or(0),
                w.column.unwrap_or(0),
            );
        }

        // Collect metadata for library emission
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

        // Rust leakage linter (W0001-W0004)
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
        // Search for metadata in the file's directory tree AND dependency crate directories.
        // Walk up from file_parent to find sibling project directories containing .wj.meta files.
        let mut meta_roots: Vec<&Path> = vec![file_parent];
        let ancestor_roots = find_dependency_metadata_roots(file_parent, &external_paths);
        let ancestor_root_refs: Vec<&Path> = ancestor_roots.iter().map(|p| p.as_path()).collect();
        meta_roots.extend_from_slice(&ancestor_root_refs);
        crate::metadata::merge_wj_meta_signatures_and_copy_structs_multi(
            &meta_roots,
            &mut global_signatures,
            &mut analyzer,
        );

        // TDD FIX for E0382: Override metadata Copy status if current file defines struct differently
        // If the file defines a struct that metadata says is Copy, but the local definition has
        // non-Copy fields (e.g., Vec), remove it from the Copy set for THIS compilation.
        // This prevents one Copy-able GameState in file A from poisoning non-Copy GameState in file B.
        for item in &program.items {
            if let Item::Struct { decl, .. } = item {
                let struct_name = &decl.name;

                if analyzer.is_copy_struct(struct_name) {
                    // Check if THIS definition is actually Copy
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

        // TDD FIX for E0308: Two-pass analysis for signature pre-collection
        // Pass 1: Analyze current file to collect all method signatures BEFORE main analysis
        // This ensures methods defined later in the file are available when analyzing earlier code
        // (e.g., DialogCondition::evaluate can see Inventory::has_item signature even if Inventory
        // is defined after DialogCondition)
        let (_, first_pass_registry, _) = analyzer
            .analyze_program_with_global_signatures(&program, &global_signatures)
            .map_err(|e| anyhow::anyhow!("First-pass analysis error: {}", e))?;

        // Merge first-pass signatures into global registry for second pass
        global_signatures.merge(&first_pass_registry);

        // Get copy structs from first pass
        let copy_structs_list = analyzer.get_copy_structs();
        let copy_structs_set: HashSet<String> = copy_structs_list.into_iter().collect();

        // Pass 2: Re-analyze with complete signature registry (including current file's signatures)
        let mut analyzer_pass2 = Analyzer::new_with_copy_structs(copy_structs_set);
        analyzer_pass2
            .check_forbidden_rust_patterns(&program)
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        let (analyzed_functions, registry, _) = analyzer_pass2
            .analyze_program_with_global_signatures(&program, &global_signatures)
            .map_err(|e| anyhow::anyhow!("Second-pass analysis error: {}", e))?;

        // Use second-pass analyzer for subsequent operations
        let mut analyzer = analyzer_pass2;

        // TDD: Infer trait signatures from impls (e.g. &mut self when any impl mutates)
        // Single-file: still need this for traits with multiple impls in same file
        analyzer
            .infer_trait_signatures_from_impls(&program)
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        // TDD: Float literal type inference - with external crate metadata for cross-crate inference
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

        // TDD: Integer literal type inference (i32, i64, u32, etc.)
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

        // Trait bound inference: walk function bodies to infer T: Display, T: Clone, etc.
        let mut trait_inference = crate::inference::InferenceEngine::new();
        let mut inferred_bounds_map = std::collections::HashMap::new();
        for item in &program.items {
            if let Item::Function { decl: func, .. } = item {
                let bounds = trait_inference.infer_function_bounds(func);
                if !bounds.is_empty() {
                    inferred_bounds_map.insert(func.name.clone(), bounds);
                }
            }
            if let Item::Impl { block, .. } = item {
                for func in &block.functions {
                    let bounds = trait_inference.infer_function_bounds(func);
                    if !bounds.is_empty() {
                        inferred_bounds_map.insert(func.name.clone(), bounds);
                    }
                }
            }
        }

        // Capture inferred signatures for .wj.meta before registry is moved into codegen
        let registry_snapshot = registry.clone();

        let mut codegen = CodeGenerator::new(registry, target);
        codegen.set_source_file(file);
        codegen.set_analyzed_trait_methods(analyzer.analyzed_trait_methods.clone());
        codegen.set_float_inference(float_inference);
        codegen.set_int_inference(int_inference);
        codegen.set_inferred_bounds(inferred_bounds_map);
        let rust_code = codegen.generate_program(&program, &analyzed_functions);

        // Determine output file -- preserve source directory hierarchy.
        // Single-file `wj build src_wj/ecs/foo.wj` uses the same layout as `build_project` in
        // main (see `project_paths::find_source_root` + `get_relative_output_path`), not a flat
        // `output/foo.rs`.
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

        // Write .wj.meta with inferred ownership for cross-file calls
        if target == CompilationTarget::Rust {
            let module_name = file
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown");
            let mut meta = ModuleMetadata::new(module_name.to_string());
            for item in &program.items {
                match item {
                    Item::Function { decl, .. } => {
                        if let Some(sig) = registry_snapshot.get_signature(&decl.name) {
                            meta.functions.insert(
                                decl.name.clone(),
                                metadata_function_sig_from_analyzer(sig, false, None),
                            );
                        }
                    }
                    Item::Impl { block, .. } => {
                        for func_decl in &block.functions {
                            let full_name = format!("{}::{}", block.type_name, func_decl.name);
                            if let Some(sig) = registry_snapshot.get_signature(&full_name) {
                                meta.functions.insert(
                                    full_name,
                                    metadata_function_sig_from_analyzer(
                                        sig,
                                        true,
                                        Some(block.type_name.clone()),
                                    ),
                                );
                            }
                        }
                    }
                    Item::Struct { decl, .. } => {
                        let mut fields = std::collections::HashMap::new();
                        for field in &decl.fields {
                            fields.insert(
                                field.name.clone(),
                                ModuleMetadata::serialize_type(&field.field_type),
                            );
                        }
                        meta.structs.insert(decl.name.clone(), fields);
                    }
                    _ => {}
                }
            }
            meta.copy_structs = analyzer.get_copy_structs();
            let meta_path = meta_cache_path(file);
            if let Some(parent) = meta_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Ok(json) = serde_json::to_string_pretty(&meta) {
                let _ = write_if_changed(&meta_path, &json);
            }
        }
    }

    // Emit metadata.json when building library
    if library && (!crate_metadata.structs.is_empty() || !crate_metadata.functions.is_empty()) {
        let metadata_path = output.join("metadata.json");
        let metadata_json = serde_json::to_string_pretty(&crate_metadata)?;
        write_if_changed(&metadata_path, &metadata_json)?;
    }

    // Always regenerate lib.rs from mod.rs when mod.rs exists.
    // lib.rs is derived from mod.rs (with `use super::*` stripped),
    // so it must stay in sync when modules are added or removed.
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

    // Always (re)generate Cargo.toml in the output directory for Rust builds.
    // The output dir is compiler-managed; source dir Cargo.toml is preserved separately.
    if target == CompilationTarget::Rust {
        let source_dir = if path.is_file() {
            path.parent().unwrap_or(path)
        } else {
            path
        };
        crate::cargo_toml::generate_single_file_cargo_toml(output, source_dir, target)?;
    }

    // WASM target emits Rust (`cdylib`); emit a matching Cargo.toml for wasm-pack / cargo.
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

/// Copy-shape check for library PASS 0 (mirrors `main.rs` `is_type_copy_quick`).
fn is_type_copy_quick_for_library(
    ty: &crate::parser::Type,
    copy_structs: &HashSet<String>,
    copy_enums: &HashSet<String>,
) -> bool {
    use crate::parser::Type;
    match ty {
        Type::Int | Type::Int32 | Type::Uint | Type::Float | Type::Bool => true,
        Type::Reference(_) => true,
        Type::MutableReference(_) => false,
        Type::Tuple(types) => types
            .iter()
            .all(|t| is_type_copy_quick_for_library(t, copy_structs, copy_enums)),
        Type::Option(inner) => is_type_copy_quick_for_library(inner, copy_structs, copy_enums),
        Type::Result(ok, err) => {
            is_type_copy_quick_for_library(ok, copy_structs, copy_enums)
                && is_type_copy_quick_for_library(err, copy_structs, copy_enums)
        }
        Type::Array(inner, _) => is_type_copy_quick_for_library(inner, copy_structs, copy_enums),
        Type::Vec(_) | Type::String => false,
        Type::RawPointer { pointee, .. } => {
            is_type_copy_quick_for_library(pointee.as_ref(), copy_structs, copy_enums)
        }
        Type::FunctionPointer { .. } => true,
        Type::Custom(name) => {
            copy_structs.contains(name)
                || copy_enums.contains(name)
                || crate::type_classification::is_copy_primitive(name)
        }
        _ => false,
    }
}

/// Discover Copy structs/enums across all library sources (including nested `mod` items).
/// `build_library_multipass` must feed this into `Analyzer::new_with_copy_structs` and codegen's
/// `set_copy_types_registry` so cross-file newtypes match CLI `build_project` behavior.
/// Returns (copy_structs, all_local_struct_names).
/// The second set contains every struct name defined in the current crate,
/// used to filter dep_copy_structs that collide with local non-Copy structs.
fn collect_global_copy_structs_for_library(
    sources: &[(PathBuf, String)],
) -> (HashSet<String>, HashSet<String>) {
    use crate::parser::ast::EnumVariantData;
    use crate::parser::{Expression, Item};

    struct StructInfo {
        name: String,
        field_types: Vec<crate::parser::Type>,
    }

    fn walk_items(
        items: &[Item<'_>],
        all_structs: &mut Vec<StructInfo>,
        global_copy_structs: &mut HashSet<String>,
        copy_enums: &mut HashSet<String>,
        struct_names: &mut HashSet<String>,
    ) {
        for item in items {
            match item {
                Item::Struct { decl, .. } => {
                    let has_copy = decl.decorators.iter().any(|d| {
                        d.name == "derive"
                            && d.arguments.iter().any(|(_, arg)| {
                                matches!(arg, Expression::Identifier { name, .. } if name == "Copy")
                            })
                    });
                    let field_types: Vec<crate::parser::Type> =
                        decl.fields.iter().map(|f| f.field_type.clone()).collect();
                    struct_names.insert(decl.name.clone());
                    all_structs.push(StructInfo {
                        name: decl.name.clone(),
                        field_types,
                    });
                    if has_copy {
                        global_copy_structs.insert(decl.name.clone());
                    }
                }
                Item::Enum { decl, .. } => {
                    let is_unit_only = decl
                        .variants
                        .iter()
                        .all(|v| matches!(v.data, EnumVariantData::Unit));
                    if is_unit_only {
                        copy_enums.insert(decl.name.clone());
                    }
                }
                Item::Mod { items: inner, .. } => {
                    walk_items(
                        inner,
                        all_structs,
                        global_copy_structs,
                        copy_enums,
                        struct_names,
                    );
                }
                _ => {}
            }
        }
    }

    let mut all_structs: Vec<StructInfo> = Vec::new();
    let mut global_copy_structs = HashSet::new();
    let mut copy_enums = HashSet::new();
    let mut struct_names = HashSet::new();

    for (file, source) in sources {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize_with_locations();
        let mut parser =
            Parser::new_with_source(tokens, file.to_string_lossy().to_string(), source.clone());
        let Ok(program) = parser.parse() else {
            eprintln!(
                "Warning: Skipping file for Copy registry (parse error): {}",
                file.display()
            );
            continue;
        };
        walk_items(
            &program.items,
            &mut all_structs,
            &mut global_copy_structs,
            &mut copy_enums,
            &mut struct_names,
        );
    }

    // Remove enum names that collide with struct names. The unqualified copy
    // registry cannot distinguish between `enum Foo` (unit → Copy) and
    // `struct Foo { name: String }` (non-Copy) in different modules. When both
    // exist, the struct's field check must govern — otherwise the enum's Copy
    // status leaks through `is_type_copy_quick_for_library` and poisons the
    // struct that references it by name.
    copy_enums.retain(|name| !struct_names.contains(name));

    // TDD FIX: When multiple structs share the same name (different modules/files),
    // be CONSERVATIVE: only mark as Copy if ALL definitions are Copy.
    // Otherwise one Copy-able GameState in file A poisons non-Copy GameState in file B,
    // causing E0382 errors when passing game_state to multiple functions.
    //
    // Strategy: Group structs by name, check if ALL variants are Copy, only then add to registry.
    use std::collections::HashMap;
    let mut structs_by_name: HashMap<String, Vec<&StructInfo>> = HashMap::new();
    for s in &all_structs {
        structs_by_name.entry(s.name.clone()).or_default().push(s);
    }

    loop {
        let mut changed = false;
        for (name, variants) in &structs_by_name {
            if global_copy_structs.contains(name) {
                continue;
            }

            // Check if ALL variants of this struct name are Copy
            let all_variants_copy = variants.iter().all(|s| {
                s.field_types.is_empty()
                    || s.field_types.iter().all(|ty| {
                        is_type_copy_quick_for_library(ty, &global_copy_structs, &copy_enums)
                    })
            });

            if all_variants_copy {
                global_copy_structs.insert(name.clone());
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    global_copy_structs.extend(copy_enums.iter().cloned());
    (global_copy_structs, struct_names)
}

/// Remove any Cargo.toml files nested under the output root.
/// Older compiler versions generated per-directory manifests; these confuse Cargo
/// into treating subdirectories as separate packages, causing cyclic dependency errors.
fn clean_nested_cargo_toml(output_dir: &std::path::Path) {
    fn visit(dir: &std::path::Path, root: &std::path::Path) {
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                if path.file_name().and_then(|n| n.to_str()) == Some("target") {
                    continue;
                }
                visit(&path, root);
            } else if path.file_name().and_then(|n| n.to_str()) == Some("Cargo.toml")
                && path.parent() != Some(root)
            {
                let _ = std::fs::remove_file(&path);
            }
        }
    }
    visit(output_dir, output_dir);
}

/// TDD FIX: Build library with global multi-pass analysis
/// Solves cross-file transitive mutability inference
#[allow(clippy::too_many_arguments)]
fn build_library_multipass(
    wj_files: &[PathBuf],
    base_path: &Path,
    output: &Path,
    target: CompilationTarget,
    library: bool,
    enable_lint: bool,
    external_paths: &HashMap<String, PathBuf>,
    mut crate_metadata: CrateMetadata,
) -> Result<()> {
    // Step 1: Read all source files (keep sources alive for lifetime safety)
    let mut sources: Vec<(PathBuf, String)> = Vec::new();

    for file in wj_files {
        let canon = std::fs::canonicalize(file).unwrap_or_else(|_| file.to_path_buf());
        let source = std::fs::read_to_string(&canon)?;
        sources.push((canon, source));
    }

    // Filter out shader files (detected by @vertex/@fragment/@compute decorators).
    // These target the WJSL→WGSL pipeline, not Rust codegen.
    //
    // Two-pass filter:
    //   Pass 1: Remove files with shader entry-point decorators
    //   Pass 2: Remove mod.wj files whose sub-modules were ALL filtered
    //
    // For mod.wj files that survive pass 2 (some children filtered, some not),
    // we collect the filtered child module names per directory so we can strip
    // them from the AST before codegen — preventing wrong code from ever being
    // generated.
    let mut removed_stems: HashSet<PathBuf> = HashSet::new();
    let mut shader_count = 0usize;
    // Map: directory path → set of module names that were filtered in that dir
    let mut filtered_modules_by_dir: HashMap<PathBuf, HashSet<String>> = HashMap::new();

    sources.retain(|(file, source)| {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize_with_locations();
        let mut parser =
            Parser::new_with_source(tokens, file.to_string_lossy().to_string(), source.clone());
        if let Ok(program) = parser.parse() {
            if is_shader_file(&program) {
                removed_stems.insert(file.clone());
                // Record the module name for its parent directory
                if let Some(parent) = file.parent() {
                    if let Some(stem) = file.file_stem().and_then(|s| s.to_str()) {
                        filtered_modules_by_dir
                            .entry(parent.to_path_buf())
                            .or_default()
                            .insert(stem.to_string());
                    }
                }
                shader_count += 1;
                return false;
            }
        }
        true
    });

    // Pass 2: mod.wj files whose only items are `pub mod` declarations
    // referencing filtered shader files should also be skipped.
    if !removed_stems.is_empty() {
        sources.retain(|(file, source)| {
            let is_mod = file
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n == "mod.wj")
                .unwrap_or(false);
            if !is_mod {
                return true;
            }
            let parent = match file.parent() {
                Some(p) => p,
                None => return true,
            };
            let mut lexer = Lexer::new(source);
            let tokens = lexer.tokenize_with_locations();
            let mut parser =
                Parser::new_with_source(tokens, file.to_string_lossy().to_string(), source.clone());
            let program = match parser.parse() {
                Ok(p) => p,
                Err(_) => return true,
            };
            let has_non_mod_items = program
                .items
                .iter()
                .any(|item| !matches!(item, Item::Mod { .. }));
            if has_non_mod_items {
                return true;
            }
            let all_subs_removed = program.items.iter().all(|item| {
                if let Item::Mod { name, .. } = item {
                    let sub_file = parent.join(format!("{}.wj", name));
                    let sub_dir_mod = parent.join(name.as_str()).join("mod.wj");
                    removed_stems.contains(&sub_file) || removed_stems.contains(&sub_dir_mod)
                } else {
                    true
                }
            });
            if all_subs_removed {
                // Record filtered directory module name for its grandparent
                if let Some(dir_name) = parent.file_name().and_then(|n| n.to_str()) {
                    if let Some(grandparent) = parent.parent() {
                        filtered_modules_by_dir
                            .entry(grandparent.to_path_buf())
                            .or_default()
                            .insert(dir_name.to_string());
                    }
                }
                shader_count += 1;
                false
            } else {
                true
            }
        });
    }
    if shader_count > 0 {
        eprintln!(
            "  Skipped {} shader file(s) from Rust pipeline (use WJSL target for GPU shaders)",
            shader_count
        );
    }

    if sources.is_empty() {
        return Ok(());
    }

    let src_base: PathBuf = {
        let raw = if base_path.is_file() {
            base_path.parent().unwrap_or(base_path).to_path_buf()
        } else {
            base_path.to_path_buf()
        };
        std::fs::canonicalize(&raw).unwrap_or(raw)
    };

    let (mut global_copy_structs, local_struct_names) =
        collect_global_copy_structs_for_library(&sources);

    // Load Copy structs AND function signatures from dependency crate metadata.
    // Function signatures provide ownership info for cross-crate calls (e.g.,
    // voxelgrid_to_svo64_flat from windjammer-game-core). The metadata includes
    // module-qualified names for unambiguous lookup.
    let dep_roots = find_dependency_metadata_roots(&src_base, external_paths);
    let mut dep_registry = SignatureRegistry::new();
    {
        let mut dep_copy_structs = Vec::new();
        let mut dep_struct_fields: HashMap<String, Vec<Vec<String>>> = HashMap::new();
        for root in &dep_roots {
            crate::metadata::merge_wj_meta_signatures_from_dir_inner_pub(
                root,
                &mut dep_registry,
                &mut dep_copy_structs,
                &mut dep_struct_fields,
            );
        }
        crate::metadata::infer_copy_from_metadata_structs_pub(
            &dep_struct_fields,
            &mut dep_copy_structs,
        );
        // Only import dep Copy status for struct names that do NOT have a local
        // definition. When the current crate defines a struct with the same name
        // as a dep struct, the local definition's Copy status (already computed
        // by collect_global_copy_structs_for_library) takes precedence.
        // Without this filter, a Copy `PlayerState` from an engine crate would
        // poison a non-Copy `PlayerState` in the game crate, causing E0382.
        for name in dep_copy_structs {
            if !local_struct_names.contains(&name) {
                global_copy_structs.insert(name);
            }
        }
    }

    // Step 2: Build initial registries from ALL files (first pass)
    // - global_registry: For ownership inference (SignatureRegistry)
    // - global_float_signatures: For float inference (function param types)
    // - global_struct_fields: For float inference (struct field types)
    // Seed with dependency crate signatures (ownership from .wj.meta files).
    // Also load the project's own .wj.meta files from prior builds so that
    // module-qualified ownership info (e.g., draw::draw_text → Borrowed) is
    // available from the very first analysis pass.
    let mut global_registry = dep_registry;
    crate::metadata::merge_wj_meta_signatures_from_dir(&src_base, &mut global_registry);
    let mut global_float_signatures: HashMap<
        String,
        (
            Vec<crate::parser::ast::types::Type>,
            Option<crate::parser::ast::types::Type>,
        ),
    > = HashMap::new();
    let mut global_struct_fields: HashMap<
        String,
        HashMap<String, crate::parser::ast::types::Type>,
    > = HashMap::new();
    let mut struct_defining_module_paths: HashMap<String, Vec<Vec<String>>> = HashMap::new();

    for (file, source) in &sources {
        // Parse with proper lifetime (program borrows source)
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize_with_locations();
        let mut parser =
            Parser::new_with_source(tokens, file.to_string_lossy().to_string(), source.clone());
        let program = parser
            .parse()
            .map_err(|e| anyhow::anyhow!("Parse error in {}: {}", file.display(), e))?;

        // Collect metadata for library emission
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

        // Collect function signatures for float inference
        for item in &program.items {
            match item {
                Item::Function { decl, .. } => {
                    let param_types: Vec<crate::parser::ast::types::Type> =
                        decl.parameters.iter().map(|p| p.type_.clone()).collect();
                    global_float_signatures
                        .insert(decl.name.clone(), (param_types, decl.return_type.clone()));
                }
                Item::Impl { block, .. } => {
                    for func_decl in &block.functions {
                        let param_types: Vec<crate::parser::ast::types::Type> = func_decl
                            .parameters
                            .iter()
                            .map(|p| p.type_.clone())
                            .collect();
                        let full_name = format!("{}::{}", block.type_name, func_decl.name);
                        global_float_signatures
                            .insert(full_name, (param_types, func_decl.return_type.clone()));
                    }
                }
                _ => {}
            }
        }

        // Collect struct field types for float/int inference (module-qualified keys).
        fn merge_struct_fields_from_items(
            items: &[crate::parser::ast::core::Item<'_>],
            module_prefix: &[String],
            global_struct_fields: &mut HashMap<
                String,
                HashMap<String, crate::parser::ast::types::Type>,
            >,
            struct_defining_module_paths: &mut HashMap<String, Vec<Vec<String>>>,
        ) {
            use crate::parser::ast::core::Item;
            use crate::type_inference::struct_field_registry;
            for item in items {
                match item {
                    Item::Struct { decl, .. } => {
                        let qualified =
                            struct_field_registry::qualify_struct_key(module_prefix, &decl.name);
                        let mut fields = HashMap::new();
                        for field in &decl.fields {
                            fields.insert(field.name.clone(), field.field_type.clone());
                        }
                        global_struct_fields.insert(qualified, fields);
                        struct_defining_module_paths
                            .entry(decl.name.clone())
                            .or_default()
                            .push(module_prefix.to_vec());
                    }
                    Item::Mod { name, items, .. } => {
                        let mut next = module_prefix.to_vec();
                        next.push(name.clone());
                        merge_struct_fields_from_items(
                            items,
                            &next,
                            global_struct_fields,
                            struct_defining_module_paths,
                        );
                    }
                    _ => {}
                }
            }
        }
        let file_module = crate::analyzer::type_collector::wj_file_to_module_path(&src_base, file)
            .unwrap_or_default();
        merge_struct_fields_from_items(
            &program.items,
            &file_module,
            &mut global_struct_fields,
            &mut struct_defining_module_paths,
        );

        // First-pass analysis
        let mut analyzer = Analyzer::new_with_copy_structs(global_copy_structs.clone());
        let (_, registry, _) = analyzer
            .analyze_program(&program)
            .map_err(|e| anyhow::anyhow!("Analysis error in {}: {}", file.display(), e))?;

        // Merge into global registry using public API
        global_registry.merge(&registry);

        // Also register module-qualified names so the code generator can find the
        // correct signature for qualified function calls.
        // Uses the full module path (e.g., combat::abilities::Ability::activate)
        // to avoid collisions when two files have the same stem name.
        let file_stem = file.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let module_path = file_module.join("::");
        if !file_stem.is_empty() {
            for (name, sig) in &registry.signatures {
                // Register under file_stem::name (e.g., abilities::draw_text)
                if !name.contains("::") {
                    let qualified = format!("{}::{}", file_stem, name);
                    global_registry.add_function(qualified, sig.clone());
                }
                // Also register under full module path for disambiguation
                // (e.g., combat::abilities::Ability::activate)
                if !module_path.is_empty() {
                    let full_qualified = format!("{}::{}", module_path, name);
                    global_registry.add_function(full_qualified, sig.clone());
                }
            }
        }
    }

    // Step 3: Global multi-pass iteration until convergence
    const MAX_GLOBAL_PASSES: usize = 10;
    let mut pass_number = 1;

    loop {
        let mut new_registry = global_registry.clone();

        // Re-analyze ALL files with current global registry
        for (file, source) in &sources {
            // Re-parse (lifetime scoped to this iteration)
            let mut lexer = Lexer::new(source);
            let tokens = lexer.tokenize_with_locations();
            let mut parser =
                Parser::new_with_source(tokens, file.to_string_lossy().to_string(), source.clone());
            let program = parser
                .parse()
                .map_err(|e| anyhow::anyhow!("Parse error: {}", e))?;

            let mut analyzer = Analyzer::new_with_copy_structs(global_copy_structs.clone());
            analyzer.set_global_struct_field_types(global_struct_fields.clone());
            let (_, file_registry, _) = analyzer
                .analyze_program_with_global_signatures(&program, &global_registry)
                .map_err(|e| anyhow::anyhow!("Analysis error in pass {}: {}", pass_number, e))?;

            // FIX: Only merge entries that CHANGED from global_registry.
            // analyze_program_with_global_signatures returns a FULL registry (global clone +
            // file-specific entries). Merging all entries would let passthrough global entries
            // from later files overwrite correct values set by earlier files in this iteration.
            // Example: manager.wj correctly infers tick=MutBorrowed, but state.wj's passthrough
            // tick=Borrowed would overwrite it because state.wj analyzed with the same stale
            // global_registry.
            let file_stem = file.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            let file_module =
                crate::analyzer::type_collector::wj_file_to_module_path(&src_base, file)
                    .unwrap_or_default();
            let module_path = file_module.join("::");
            for (name, sig) in &file_registry.signatures {
                match global_registry.signatures.get(name) {
                    None => {
                        new_registry.signatures.insert(name.clone(), sig.clone());
                    }
                    Some(old_sig) => {
                        if sig.param_ownership != old_sig.param_ownership
                            || sig.return_ownership != old_sig.return_ownership
                            || sig.has_self_receiver != old_sig.has_self_receiver
                        {
                            new_registry.signatures.insert(name.clone(), sig.clone());
                            // Keep ALL module-qualified aliases in sync.
                            // When a Type::method entry changes (e.g., Ability::activate
                            // gets player corrected from Owned→MutBorrowed), the
                            // module-qualified alias (combat_abilities::Ability::activate)
                            // must also be updated. Without this, the codegen's collision
                            // fallback finds the stale step 2 entry.
                            if !file_stem.is_empty() {
                                let qualified = format!("{}::{}", file_stem, name);
                                new_registry.signatures.insert(qualified, sig.clone());
                                if !module_path.is_empty() {
                                    let full_qualified = format!("{}::{}", module_path, name);
                                    new_registry.signatures.insert(full_qualified, sig.clone());
                                }
                            }
                        }
                    }
                }
            }
        }

        // Convergence check: did any signatures change in this pass?
        let mut changed = false;
        for (name, sig) in &new_registry.signatures {
            match global_registry.signatures.get(name) {
                None => {
                    changed = true;
                    break;
                }
                Some(old_sig) => {
                    if sig.param_ownership != old_sig.param_ownership
                        || sig.return_ownership != old_sig.return_ownership
                        || sig.has_self_receiver != old_sig.has_self_receiver
                    {
                        changed = true;
                        break;
                    }
                }
            }
        }

        if !changed || pass_number >= MAX_GLOBAL_PASSES {
            global_registry = new_registry;
            break;
        }

        global_registry = new_registry;
        pass_number += 1;
    }

    // Collect `pub use` re-exports from every file first so `use super::*` / `use crate::...::*`
    // can resolve struct field types (glob has no explicit type path).
    let mut module_re_exports: HashMap<String, HashMap<String, String>> = HashMap::new();
    for (file, source) in &sources {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize_with_locations();
        let mut parser =
            Parser::new_with_source(tokens, file.to_string_lossy().to_string(), source.clone());
        let program = parser
            .parse()
            .map_err(|e| anyhow::anyhow!("Parse error in {}: {}", file.display(), e))?;
        let file_module = crate::analyzer::type_collector::wj_file_to_module_path(&src_base, file)
            .unwrap_or_default();
        crate::type_inference::struct_field_registry::merge_module_reexports_from_items(
            &program.items,
            &file_module,
            &global_struct_fields,
            &struct_defining_module_paths,
            &mut module_re_exports,
        );
        if crate::type_inference::struct_field_registry::debug_struct_import_trace()
            && file.to_string_lossy().contains("dialogue")
        {
            eprintln!(
                "=== WJ_DEBUG: file={} file_module_path={:?}",
                file.display(),
                file_module
            );
        }
    }

    if crate::type_inference::struct_field_registry::debug_struct_import_trace() {
        eprintln!("=== GLOBAL MODULE_RE_EXPORTS (post pre-pass) ===");
        let mut mods: Vec<_> = module_re_exports.keys().cloned().collect();
        mods.sort();
        for m in &mods {
            if !m.contains("dialogue") && !m.is_empty() {
                continue;
            }
            let exports = &module_re_exports[m];
            eprintln!("  module {:?}: {} exports", m, exports.len());
            for (name, key) in exports {
                if name.contains("Dialogue") {
                    eprintln!("    {} → {}", name, key);
                }
            }
        }
    }

    // Step 4A: Global float inference pass (collect constraints from ALL files first)
    let mut global_float_inference = FloatInference::new();
    if !external_paths.is_empty() {
        global_float_inference.set_external_crate_metadata_paths(external_paths);
    }
    global_float_inference.set_global_function_signatures(global_float_signatures.clone());
    global_float_inference.set_global_struct_field_types(&global_struct_fields);
    global_float_inference.set_struct_defining_module_paths(struct_defining_module_paths.clone());
    global_float_inference.set_module_re_exports(module_re_exports.clone());

    // Collect constraints from ALL files into one FloatInference instance
    for (file, source) in &sources {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize_with_locations();
        let mut parser =
            Parser::new_with_source(tokens, file.to_string_lossy().to_string(), source.clone());
        let program = parser
            .parse()
            .map_err(|e| anyhow::anyhow!("Parse error in {}: {}", file.display(), e))?;
        let file_module = crate::analyzer::type_collector::wj_file_to_module_path(&src_base, file)
            .unwrap_or_default();
        global_float_inference.set_current_file_module_path(file_module);
        global_float_inference.infer_program(&program);
    }

    // Check for float inference errors
    if !global_float_inference.errors.is_empty() {
        for error in &global_float_inference.errors {
            eprintln!("Float inference error: {}", error);
        }
        return Err(anyhow::anyhow!(
            "Float type inference failed: {} error(s)",
            global_float_inference.errors.len()
        ));
    }

    // Step 4A2: Global int inference pass (same architecture as float)
    let mut global_int_inference = IntInference::new();
    global_int_inference.set_global_function_signatures(global_float_signatures.clone());
    global_int_inference.set_global_struct_field_types(&global_struct_fields);
    global_int_inference.set_struct_defining_module_paths(struct_defining_module_paths);
    global_int_inference.set_module_re_exports(module_re_exports);

    for (file, source) in &sources {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize_with_locations();
        let mut parser =
            Parser::new_with_source(tokens, file.to_string_lossy().to_string(), source.clone());
        let program = parser
            .parse()
            .map_err(|e| anyhow::anyhow!("Parse error in {}: {}", file.display(), e))?;
        let file_module = crate::analyzer::type_collector::wj_file_to_module_path(&src_base, file)
            .unwrap_or_default();
        global_int_inference.set_current_file_module_path(file_module);
        global_int_inference.infer_program(&program);
    }

    if !global_int_inference.errors.is_empty() {
        for error in &global_int_inference.errors {
            eprintln!("Int inference error: {}", error);
        }
        return Err(anyhow::anyhow!(
            "Int type inference failed: {} error(s)",
            global_int_inference.errors.len()
        ));
    }

    let type_defining_modules = build_type_defining_modules_for_library(&sources, &src_base)?;
    let extern_submodule_qualifiers = build_extern_submodule_qualifier_map(&sources, &src_base)?;

    // Step 4B-pre: Build GLOBAL analyzed_trait_methods across ALL files.
    // Each file's Analyzer is fresh, so cross-file trait info (e.g. RenderPort defined
    // in render_port.wj but implemented in voxel_gpu_renderer.wj) would be missing
    // if we only used per-file analysis. This step mirrors main.rs's finalize_trait_inference.
    //
    // Runs on a separate thread with a large stack because the merged program (~3000 items)
    // can produce deep recursive analysis.
    let global_analyzed_trait_methods = {
        let global_copy_structs_clone = global_copy_structs.clone();
        let sources_for_thread: Vec<(std::path::PathBuf, String)> = sources
            .iter()
            .map(|(p, s)| (p.clone(), s.clone()))
            .collect();

        let handle = std::thread::Builder::new()
            .name("trait-inference".to_string())
            .stack_size(64 * 1024 * 1024)
            .spawn(move || -> Result<HashMap<String, HashMap<String, crate::analyzer::AnalyzedFunction<'static>>>, String> {
                let mut shared_analyzer = Analyzer::new_with_copy_structs(global_copy_structs_clone);

                // Parse ALL files upfront and keep parsers alive. The parser's arena owns AST
                // nodes; dropping a parser frees its arena, invalidating any `&'ast` references.
                // Previously, parsers were created in a loop and dropped each iteration, causing
                // use-after-free (SIGSEGV) when `all_items` held dangling arena references.
                let mut parsers: Vec<Parser> = Vec::with_capacity(sources_for_thread.len());
                let mut programs: Vec<crate::parser::Program<'_>> = Vec::with_capacity(sources_for_thread.len());

                for (_file, source) in &sources_for_thread {
                    let mut lexer = Lexer::new(source);
                    let tokens = lexer.tokenize_with_locations();
                    let parser = Parser::new_with_source(
                        tokens,
                        String::new(),
                        source.clone(),
                    );
                    parsers.push(parser);
                }

                for parser in &mut parsers {
                    if let Ok(program) = parser.parse() {
                        programs.push(program);
                    }
                }

                for program in &programs {
                    shared_analyzer.register_traits_from_program(program)
                        .unwrap_or_else(|e| eprintln!("Trait registration warning: {}", e));
                }

                let mut all_items = Vec::new();
                for program in programs {
                    all_items.extend(program.items);
                }

                let merged_program = crate::parser::Program { items: all_items };
                shared_analyzer.infer_trait_signatures_from_impls(&merged_program)?;
                // parsers (and their arenas) are dropped here, AFTER analysis is complete
                Ok(shared_analyzer.analyzed_trait_methods.clone())
            })
            .map_err(|e| anyhow::anyhow!("Failed to spawn trait inference thread: {}", e))?;

        match handle.join() {
            Ok(Ok(methods)) => methods,
            Ok(Err(e)) => {
                eprintln!("Cross-file trait inference warning: {}", e);
                HashMap::new()
            }
            Err(_) => {
                eprintln!("⚠️  Global trait inference thread panicked (stack overflow?) — skipping cross-file trait methods.");
                HashMap::new()
            }
        }
    };

    // Step 4B: Final analysis + code generation (using shared global_float_inference)
    for (file, source) in sources.iter() {
        // Final parse
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize_with_locations();
        let mut parser =
            Parser::new_with_source(tokens, file.to_string_lossy().to_string(), source.clone());
        let mut program = parser
            .parse()
            .map_err(|e| anyhow::anyhow!("Parse error: {}", e))?;

        // Strip Item::Mod entries for modules that were filtered out (e.g., shaders).
        // This prevents generating invalid `pub mod X;` declarations in the first
        // place, rather than cleaning them up post-hoc.
        if let Some(parent_dir) = file.parent() {
            if let Some(filtered_names) = filtered_modules_by_dir.get(parent_dir) {
                program.items = strip_filtered_mod_items(program.items, filtered_names);
            }
        }

        let mut analyzer = Analyzer::new_with_copy_structs(global_copy_structs.clone());
        analyzer.set_global_struct_field_types(global_struct_fields.clone());

        // Rust leakage linter
        if enable_lint {
            let file_name = file.to_string_lossy().to_string();
            let mut rust_leakage = RustLeakageLinter::new(&file_name);
            rust_leakage.lint_program(&program);
            for diag in rust_leakage.diagnostics() {
                eprintln!("{}", diag);
            }
        }

        // Register traits so per-file analysis can resolve trait contracts
        analyzer
            .register_traits_from_program(&program)
            .unwrap_or_else(|e| eprintln!("Trait registration warning: {}", e));

        // Final analysis with converged registry
        let (analyzed_functions, registry, _) = analyzer
            .analyze_program_with_global_signatures(&program, &global_registry)
            .map_err(|e| anyhow::anyhow!("Final analysis error: {}", e))?;

        analyzer
            .infer_trait_signatures_from_impls(&program)
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        // Merge per-file trait analysis with global cross-file trait methods.
        // Global takes priority (it has the merged view from ALL implementations).
        let mut merged_trait_methods = analyzer.analyzed_trait_methods.clone();
        for (trait_name, methods) in &global_analyzed_trait_methods {
            let entry = merged_trait_methods.entry(trait_name.clone()).or_default();
            for (method_name, method_analysis) in methods {
                entry.insert(method_name.clone(), method_analysis.clone());
            }
        }

        // Preserve directory structure (directory-module layout when `foo.wj` + `foo/*.wj` co-exist).
        let output_file = crate::project_paths::resolve_wj_output_path(&src_base, file, output)?;
        if let Some(parent) = output_file.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Library-style modules: `use super::*` + automatic sibling `use super::Type` imports.
        // Use the global registry (all cross-file signatures) merged with the per-file registry
        // so that method calls to other files' types resolve correctly for auto-borrowing.
        let mut full_registry = global_registry.clone();
        full_registry.merge(&registry);
        let registry_snapshot = full_registry.clone();
        let mut codegen = CodeGenerator::new_for_module(full_registry, target);
        codegen.set_copy_types_registry(global_copy_structs.clone());
        codegen.set_global_struct_field_types(global_struct_fields.clone());
        codegen.set_output_file(&output_file);
        codegen.set_source_file(file);
        codegen.set_library_source_root(src_base.clone());
        codegen.set_type_defining_modules(type_defining_modules.clone());
        codegen.set_extern_submodule_qualifiers(extern_submodule_qualifiers.clone());
        codegen.set_analyzed_trait_methods(merged_trait_methods);
        codegen.set_float_inference(global_float_inference.clone());
        codegen.set_int_inference(global_int_inference.clone());

        // Trait bound inference for this file's functions
        let mut trait_inference = crate::inference::InferenceEngine::new();
        let mut inferred_bounds_map = std::collections::HashMap::new();
        for item in &program.items {
            if let Item::Function { decl: func, .. } = item {
                let bounds = trait_inference.infer_function_bounds(func);
                if !bounds.is_empty() {
                    inferred_bounds_map.insert(func.name.clone(), bounds);
                }
            }
            if let Item::Impl { block, .. } = item {
                for func in &block.functions {
                    let bounds = trait_inference.infer_function_bounds(func);
                    if !bounds.is_empty() {
                        inferred_bounds_map.insert(func.name.clone(), bounds);
                    }
                }
            }
        }
        codegen.set_inferred_bounds(inferred_bounds_map);

        let rust_code = codegen.generate_program(&program, &analyzed_functions);
        write_if_changed(&output_file, &rust_code)?;

        // Write .wj.meta with inferred ownership for cross-file calls
        if target == CompilationTarget::Rust {
            let module_name = file
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown");
            let mut meta = ModuleMetadata::new(module_name.to_string());
            for item in &program.items {
                match item {
                    Item::Function { decl, .. } => {
                        if let Some(sig) = registry_snapshot.get_signature(&decl.name) {
                            meta.functions.insert(
                                decl.name.clone(),
                                metadata_function_sig_from_analyzer(sig, false, None),
                            );
                        }
                    }
                    Item::Impl { block, .. } => {
                        for func_decl in &block.functions {
                            let full_name = format!("{}::{}", block.type_name, func_decl.name);
                            if let Some(sig) = registry_snapshot.get_signature(&full_name) {
                                meta.functions.insert(
                                    full_name,
                                    metadata_function_sig_from_analyzer(
                                        sig,
                                        true,
                                        Some(block.type_name.clone()),
                                    ),
                                );
                            }
                        }
                    }
                    Item::Struct { decl, .. } => {
                        let mut fields = std::collections::HashMap::new();
                        for field in &decl.fields {
                            fields.insert(
                                field.name.clone(),
                                ModuleMetadata::serialize_type(&field.field_type),
                            );
                        }
                        meta.structs.insert(decl.name.clone(), fields);
                    }
                    _ => {}
                }
            }
            meta.copy_structs = analyzer.get_copy_structs();
            let meta_path = meta_cache_path(file);
            if let Some(parent) = meta_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Ok(json) = serde_json::to_string_pretty(&meta) {
                let _ = write_if_changed(&meta_path, &json);
            }
        }
    }

    // Emit metadata.json
    if library && (!crate_metadata.structs.is_empty() || !crate_metadata.functions.is_empty()) {
        let metadata_path = output.join("metadata.json");
        let metadata_json = serde_json::to_string_pretty(&crate_metadata)?;
        write_if_changed(&metadata_path, &metadata_json)?;
    }

    // Generate mod.rs (and lib.rs) so individual module files are tied
    // together as submodules. Without this, `use super::*;` in generated
    // files would fail because Cargo wouldn't know about the crate structure.
    if target == CompilationTarget::Rust {
        crate::build_utils::generate_mod_file_with_layout(
            output,
            Some((output, src_base.as_path())),
        )?;
    }

    // Always (re)generate Cargo.toml in the output directory for Rust builds.
    if target == CompilationTarget::Rust {
        // Clean stale nested Cargo.toml files left by older compiler versions.
        // Only the root Cargo.toml is valid; nested ones confuse Cargo into
        // treating subdirectories as separate packages (cyclic dependency errors).
        clean_nested_cargo_toml(output);

        let source_dir = if base_path.is_file() {
            base_path.parent().unwrap_or(base_path)
        } else {
            base_path
        };
        crate::cargo_toml::generate_single_file_cargo_toml(output, source_dir, target)?;
    }

    if target == CompilationTarget::Wasm {
        let source_dir = if base_path.is_file() {
            base_path.parent().unwrap_or(base_path)
        } else {
            base_path
        };
        crate::cargo_toml::generate_wasm_cargo_toml(output, source_dir)?;
    }

    Ok(())
}

/// Map `(parent_module, symbol)` → child module for symbols defined under `parent/child/*.wj`.
/// Fixes `parent::symbol` call sites when Rust places the item in `parent::child`.
fn build_extern_submodule_qualifier_map(
    sources: &[(PathBuf, String)],
    base: &Path,
) -> Result<HashMap<(String, String), String>> {
    use crate::parser::ast::core::Item;
    let mut map: HashMap<(String, String), String> = HashMap::new();
    let mut conflicts: HashSet<(String, String)> = HashSet::new();

    fn merge_extern_submodule_symbols_from_items(
        items: &[Item<'_>],
        module_prefix: &[String],
        map: &mut HashMap<(String, String), String>,
        conflicts: &mut HashSet<(String, String)>,
    ) {
        for item in items {
            match item {
                Item::Function { decl, .. } if decl.is_extern => {
                    insert_extern_submodule_entry(map, conflicts, module_prefix, &decl.name);
                }
                Item::Struct { decl, .. } => {
                    insert_extern_submodule_entry(map, conflicts, module_prefix, &decl.name);
                }
                Item::Enum { decl, .. } => {
                    insert_extern_submodule_entry(map, conflicts, module_prefix, &decl.name);
                }
                Item::Mod {
                    name,
                    items: nested,
                    ..
                } => {
                    let mut next = module_prefix.to_vec();
                    next.push(name.clone());
                    merge_extern_submodule_symbols_from_items(nested, &next, map, conflicts);
                }
                _ => {}
            }
        }
    }

    fn insert_extern_submodule_entry(
        map: &mut HashMap<(String, String), String>,
        conflicts: &mut HashSet<(String, String)>,
        module_prefix: &[String],
        symbol: &str,
    ) {
        if module_prefix.len() < 2 {
            return;
        }
        let parent = module_prefix[module_prefix.len() - 2].clone();
        let child = module_prefix.last().unwrap().clone();
        let key = (parent, symbol.to_string());
        if conflicts.contains(&key) {
            return;
        }
        match map.get(&key) {
            Some(existing) if existing != &child => {
                map.remove(&key);
                conflicts.insert(key);
            }
            Some(_) => {}
            None => {
                map.insert(key, child);
            }
        }
    }

    for (file, source) in sources {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize_with_locations();
        let mut parser =
            Parser::new_with_source(tokens, file.to_string_lossy().to_string(), source.clone());
        let program = parser
            .parse()
            .map_err(|e| anyhow::anyhow!("Parse error in {}: {}", file.display(), e))?;
        let Some(module_path) = crate::analyzer::type_collector::wj_file_to_module_path(base, file)
        else {
            continue;
        };
        merge_extern_submodule_symbols_from_items(
            &program.items,
            &module_path,
            &mut map,
            &mut conflicts,
        );
    }

    for k in conflicts {
        map.remove(&k);
    }

    Ok(map)
}

/// Map struct/enum/trait/type-alias names to Rust module paths (from library root) for auto-import resolution.
///
/// Each type name may be defined in multiple modules (e.g. `TileId`); codegen picks the matching
/// path from the `crate::parent::Type` import prefix (fixes E0432 when parent `mod.rs` does not re-export).
fn build_type_defining_modules_for_library(
    sources: &[(PathBuf, String)],
    base: &Path,
) -> Result<HashMap<String, Vec<Vec<String>>>> {
    let mut map: HashMap<String, Vec<Vec<String>>> = HashMap::new();
    for (file, source) in sources {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize_with_locations();
        let mut parser =
            Parser::new_with_source(tokens, file.to_string_lossy().to_string(), source.clone());
        let program = parser
            .parse()
            .map_err(|e| anyhow::anyhow!("Parse error in {}: {}", file.display(), e))?;
        let Some(module_path) = crate::analyzer::type_collector::wj_file_to_module_path(base, file)
        else {
            continue;
        };
        for name in crate::analyzer::type_collector::collect_local_type_names(&program) {
            map.entry(name).or_default().push(module_path.clone());
        }
    }
    Ok(map)
}

/// Find dependency metadata directories by walking up from the file's parent directory.
/// Looks for sibling directories that contain `.wj.meta` files (e.g., windjammer-game-core/src_wj/).
/// Also includes explicit external paths from `--metadata` CLI flags.
fn find_dependency_metadata_roots(
    file_parent: &Path,
    external_paths: &HashMap<String, PathBuf>,
) -> Vec<PathBuf> {
    let mut roots = Vec::new();

    // 1. Include explicit external paths
    for path in external_paths.values() {
        roots.push(path.clone());
    }

    // 2. Walk up from file_parent to find workspace root, then search for src_wj/ directories
    let canonical =
        std::fs::canonicalize(file_parent).unwrap_or_else(|_| file_parent.to_path_buf());
    let mut current = canonical.as_path();
    // Walk up at most 6 levels to find sibling project directories
    for _ in 0..6 {
        let Some(parent) = current.parent() else {
            break;
        };
        if let Ok(entries) = std::fs::read_dir(parent) {
            for entry in entries.flatten() {
                let p = entry.path();
                if !p.is_dir() {
                    continue;
                }
                // Skip the directory tree we're already in
                if canonical.starts_with(&p) {
                    continue;
                }
                // Look for src/ directories containing .wj files (Windjammer project convention)
                let src_dir = p.join("src");
                if src_dir.is_dir() {
                    roots.push(src_dir);
                }
                // Also check one level deeper (e.g., windjammer-game/windjammer-game-core/src/)
                if let Ok(sub_entries) = std::fs::read_dir(&p) {
                    for sub_entry in sub_entries.flatten() {
                        let sub = sub_entry.path();
                        if sub.is_dir() {
                            let sub_src = sub.join("src");
                            if sub_src.is_dir() {
                                roots.push(sub_src);
                            }
                        }
                    }
                }
            }
        }
        current = parent;
    }

    roots
}

fn find_wj_files(path: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    if path.is_file() {
        if path.extension().and_then(|s| s.to_str()) == Some("wj") {
            if path.file_name().and_then(|n| n.to_str()) == Some("mod.wj") {
                if let Some(parent) = path.parent() {
                    find_wj_files_recursive(parent, &mut files)?;
                } else {
                    files.push(path.to_path_buf());
                }
            } else {
                files.push(path.to_path_buf());
            }
        }
    } else if path.is_dir() {
        find_wj_files_recursive(path, &mut files)?;
    }
    files.sort();
    Ok(files)
}

fn find_wj_files_recursive(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("wj") {
            files.push(path);
        } else if path.is_dir() {
            find_wj_files_recursive(&path, files)?;
        }
    }
    Ok(())
}

/// Detect whether a parsed program is a GPU shader file by checking for
/// shader entry-point decorators (@vertex, @fragment, @compute).
///
/// Content-based detection: any .wj file can be a shader regardless of
/// its directory name. Uses DecoratorRegistry for centralized classification
/// rather than hardcoded decorator lists.
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

/// Strip `Item::Mod` entries from a list of AST items when the module name
/// appears in the `filtered_modules` set.
///
/// This is the proper fix for orphaned `pub mod` declarations: instead of
/// generating wrong code and then cleaning it up post-hoc, we remove the
/// filtered modules from the AST BEFORE codegen, so wrong code is never
/// generated in the first place.
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
