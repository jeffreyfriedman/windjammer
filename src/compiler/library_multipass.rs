//! Multi-file library build with global multi-pass analysis.

use crate::analyzer::{Analyzer, SignatureRegistry};
use crate::codegen::rust::CodeGenerator;
use crate::lexer::Lexer;
use crate::metadata::{metadata_function_sig_from_analyzer, CrateMetadata};
use crate::parser::ast::core::Item;
use crate::parser::Parser;
use crate::type_inference::{FloatInference, IntInference};
use crate::CompilationTarget;
use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Instant;

fn profiling_enabled() -> bool {
    std::env::var("WJ_PROFILE").map_or(false, |v| v == "1" || v == "true")
}

fn profile_phase(phase: &str, start: Instant) {
    if profiling_enabled() {
        eprintln!("[wj-profile] {}: {}ms", phase, start.elapsed().as_millis());
    }
}

/// Detect if source code imports from Windjammer stdlib (`std::*`)
fn uses_windjammer_stdlib(source: &str) -> HashSet<String> {
    let mut stdlib_modules = HashSet::new();
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("use std::") {
            // Extract module: "use std::collections::HashMap" -> "collections"
            if let Some(rest) = trimmed.strip_prefix("use std::") {
                if let Some(module) = rest.split("::").next() {
                    stdlib_modules.insert(module.to_string());
                }
            }
        }
    }
    stdlib_modules
}

/// Find Windjammer stdlib directory (relative to compiler binary)
fn find_stdlib_dir() -> Option<PathBuf> {
    // Check: ../std/ relative to compiler executable
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            // In development: windjammer/target/release/wj -> windjammer/std/
            let dev_stdlib = parent.parent()?.parent()?.join("std");
            if dev_stdlib.is_dir() {
                return Some(dev_stdlib);
            }
            // In installed: ~/.cargo/bin/wj -> ~/.cargo/wj-stdlib/
            let installed_stdlib = parent.parent()?.join("wj-stdlib");
            if installed_stdlib.is_dir() {
                return Some(installed_stdlib);
            }
        }
    }
    None
}

/// Match a step-2 metadata key (`Type::method`) to the converged multipass registry entry.
fn resolve_converged_local_signature<'a>(
    registry: &'a SignatureRegistry,
    name: &str,
) -> Option<&'a crate::analyzer::FunctionSignature> {
    if let Some(sig) = registry.get_signature(name) {
        return Some(sig);
    }
    if let Some((ty, method)) = name.rsplit_once("::") {
        let suffix = format!("::{ty}::{method}");
        if let Some((_, sig)) = registry
            .signatures
            .iter()
            .find(|(k, _)| k.ends_with(&suffix))
        {
            return Some(sig);
        }
    }
    registry.find_signature_ending_with(name)
}

/// TDD FIX: Build library with global multi-pass analysis
/// Solves cross-file transitive mutability inference
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_library_multipass(
    wj_files: &[PathBuf],
    base_path: &Path,
    output: &Path,
    target: CompilationTarget,
    library: bool,
    enable_lint: bool,
    external_paths: &HashMap<String, PathBuf>,
    mut crate_metadata: CrateMetadata,
) -> Result<()> {
    let phase_start = Instant::now();

    // Step 0: Detect stdlib usage and prepend stdlib files
    let mut sources: Vec<(PathBuf, String)> = Vec::new();
    let mut needed_stdlib_modules = HashSet::new();

    // Quick scan: which stdlib modules does user code import?
    for file in wj_files {
        if let Ok(source) = std::fs::read_to_string(file) {
            needed_stdlib_modules.extend(uses_windjammer_stdlib(&source));
        }
    }

    // If stdlib is needed, prepend stdlib source files FIRST
    if !needed_stdlib_modules.is_empty() {
        if let Some(stdlib_dir) = find_stdlib_dir() {
            for module in &needed_stdlib_modules {
                let stdlib_file = stdlib_dir.join(format!("{}.wj", module));
                if stdlib_file.exists() {
                    if let Ok(source) = std::fs::read_to_string(&stdlib_file) {
                        sources.push((stdlib_file, source));
                    }
                }
            }
        }
    }

    // Step 1: Read all user source files (keep sources alive for lifetime safety)
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
            if super::is_shader_file(&program) {
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

    profile_phase("Step 0+1: Source reading + shader filter", phase_start);

    if sources.is_empty() {
        return Ok(());
    }

    let parse_start = Instant::now();
    // PERFORMANCE: Parse all files once upfront and reuse ASTs across all pipeline
    // phases. Previously each file was re-parsed 9-10 times. The Parser owns arenas
    // (expr_arena, stmt_arena, pattern_arena) that Program references borrow from,
    // so parsers must be kept alive as long as their programs are used.
    let mut parsers: Vec<Parser> = Vec::with_capacity(sources.len());
    let mut parsed_programs: Vec<crate::parser::Program<'static>> = Vec::with_capacity(sources.len());
    let mut deferred_lint_errors: Vec<String> = Vec::new();
    for (file, source) in &sources {
        let (parser, program) = super::parse_wj_source(file, source)?;
        if let Err(e) = super::emit_parser_warnings(&parser) {
            deferred_lint_errors.push(format!("{}", e));
        }
        parsed_programs.push(program);
        parsers.push(parser);
    }
    profile_phase("Parse upfront", parse_start);

    let src_base: PathBuf = {
        let raw = if base_path.is_file() {
            base_path.parent().unwrap_or(base_path).to_path_buf()
        } else {
            base_path.to_path_buf()
        };
        std::fs::canonicalize(&raw).unwrap_or(raw)
    };

    // Dependency metadata roots (needed for both freshness check and analysis)
    let dep_roots =
        super::dependency_resolution::find_dependency_metadata_roots(&src_base, external_paths);

    // INCREMENTAL: Whole-crate fast path — if no .wj file has changed and dep
    // metadata is also unchanged, skip the entire transpilation pipeline.
    if super::cache_management::all_sources_fresh(&sources, &src_base, output, &dep_roots) {
        let user_count = sources.len() - needed_stdlib_modules.len();
        eprintln!(
            "✓ All {} source files up to date, skipping transpilation",
            user_count
        );
        return Ok(());
    }

    if !super::cache_management::is_compiler_stamp_fresh(output) {
        eprintln!(
            "⟳ Compiler changed — re-transpiling all sources"
        );
    }

    let copy_registry_start = Instant::now();
    let (mut global_copy_structs, local_struct_names, explicit_non_copy_structs) =
        super::library_copy_registry::collect_global_copy_structs_for_library(&sources);
    let global_non_copy_enums =
        super::library_copy_registry::collect_non_copy_enums_for_library(&sources);

    // Merge explicit non-Copy structs with non-Copy enums for codegen registry
    let mut global_non_copy_types = global_non_copy_enums.clone();
    global_non_copy_types.extend(explicit_non_copy_structs.iter().cloned());
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
    profile_phase("Copy registry collection", copy_registry_start);

    let step2_start = Instant::now();
    // Step 2: Build initial registries from ALL files (first pass)
    // - global_registry: For ownership inference (SignatureRegistry)
    // - global_float_signatures: For float inference (function param types)
    // - global_struct_fields: For float inference (struct field types)
    // Seed with dependency crate signatures (ownership from .wj.meta files).
    // Also load the project's own .wj.meta files from prior builds so that
    // module-qualified ownership info (e.g., draw::draw_text → Borrowed) is
    // available from the very first analysis pass.
    let mut global_registry = dep_registry;
    // Drop dependency metadata for types defined in this crate so local inference wins.
    // Handles module-qualified keys like `dialogue::tree::DialogueNodeTree::get_node`.
    crate::metadata::drop_dependency_signatures_for_local_types(
        &mut global_registry.signatures,
        &local_struct_names,
    );
    crate::metadata::merge_wj_meta_signatures_from_dir(&src_base, &mut global_registry);
    crate::metadata::drop_dependency_signatures_for_local_types(
        &mut global_registry.signatures,
        &local_struct_names,
    );
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

    // Load typed struct field types from dependency metadata for nested field
    // chain resolution (e.g., self.renderer.voxel_renderer → VoxelGPURenderer).
    global_struct_fields.extend(
        crate::metadata::load_merged_external_struct_fields(external_paths, Some(&local_struct_names))
    );

    for (i, (file, _source)) in sources.iter().enumerate() {
        let program = &parsed_programs[i];

        crate::metadata::merge_file_skeleton_into_crate(&mut crate_metadata, file, program);

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
                    // TDD FIX: Strip generic parameters from type_name for signature registration
                    // Parser stores "HashMap<K, V>" but we look up "HashMap::get"
                    let base_type_name = block.type_name.split('<').next().unwrap_or(&block.type_name);
                    for func_decl in &block.functions {
                        let param_types: Vec<crate::parser::ast::types::Type> = func_decl
                            .parameters
                            .iter()
                            .map(|p| p.type_.clone())
                            .collect();
                        let full_name = format!("{}::{}", base_type_name, func_decl.name);
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

        // Seed global registry with declaration stubs (parameter names + types only).
        // Full ownership inference runs in Step 3 with cross-file context.
        let stub_registry = SignatureRegistry::from_program_declarations(program);
        global_registry.merge(&stub_registry);

        // Also register module-qualified names so the code generator can find the
        // correct signature for qualified function calls.
        // Uses the full module path (e.g., combat::abilities::Ability::activate)
        // to avoid collisions when two files have the same stem name.
        let file_stem = file.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let module_path = file_module.join("::");
        global_registry.register_module_aliases(&stub_registry, file_stem, &module_path);
    }
    profile_phase("Step 2: Declaration stub collection", step2_start);

    // Step 3: Global multi-pass iteration until convergence
    // Wrap read-only data in Arc for O(1) sharing across all files (avoids O(n) deep clones).
    let global_struct_fields = std::sync::Arc::new(global_struct_fields);
    let struct_defining_module_paths = std::sync::Arc::new(struct_defining_module_paths);
    let global_copy_structs = std::sync::Arc::new(global_copy_structs);

    const MAX_GLOBAL_PASSES: usize = 10;
    let mut pass_number = 1;
    let step3_start = Instant::now();

    loop {
        let round_start = Instant::now();
        let mut new_registry = global_registry.clone();

        // Re-analyze ALL files with current global registry
        for (i, (file, _source)) in sources.iter().enumerate() {
            let program = &parsed_programs[i];

            let mut analyzer = Analyzer::for_library_pass(
                (*global_copy_structs).clone(),
                global_struct_fields.clone(),
                struct_defining_module_paths.clone(),
            );
            analyzer.convergence_only = true;
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
                let should_insert = match global_registry.signatures.get(name) {
                    None => true,
                    Some(old_sig) => SignatureRegistry::ownership_changed(old_sig, sig),
                };
                if should_insert {
                    new_registry.signatures.insert(name.clone(), sig.clone());
                    // Keep module-qualified aliases in sync when ownership changes
                    if !file_stem.is_empty() {
                        if !name.contains("::") {
                            new_registry.add_function(
                                format!("{}::{}", file_stem, name),
                                sig.clone(),
                            );
                        }
                        if !module_path.is_empty() {
                            new_registry.add_function(
                                format!("{}::{}", module_path, name),
                                sig.clone(),
                            );
                        }
                    }
                }
            }
        }

        profile_phase(&format!("Step 3 round {}", pass_number), round_start);

        // Convergence check: did any signatures change in this pass?
        let changed = new_registry.signatures.iter().any(|(name, sig)| {
            match global_registry.signatures.get(name) {
                None => true,
                Some(old_sig) => SignatureRegistry::ownership_changed(old_sig, sig),
            }
        });

        if !changed || pass_number >= MAX_GLOBAL_PASSES {
            global_registry = new_registry;
            break;
        }

        global_registry = new_registry;
        pass_number += 1;
    }
    profile_phase("Step 3 total", step3_start);

    // Collect `pub use` re-exports from every file first so `use super::*` / `use crate::...::*`
    // can resolve struct field types (glob has no explicit type path).
    let mut module_re_exports: HashMap<String, HashMap<String, String>> = HashMap::new();
    for (i, (file, _source)) in sources.iter().enumerate() {
        let program = &parsed_programs[i];
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
    let step4a_start = Instant::now();
    let mut global_float_inference = FloatInference::new();
    if !external_paths.is_empty() {
        global_float_inference.set_external_crate_metadata_paths(external_paths);
    }
    global_float_inference.set_global_function_signatures(global_float_signatures.clone());
    global_float_inference.set_global_struct_field_types(&global_struct_fields);
    global_float_inference.set_struct_defining_module_paths((*struct_defining_module_paths).clone());
    global_float_inference.set_module_re_exports(module_re_exports.clone());

    // Collect constraints from ALL files into one FloatInference instance
    for (i, (file, _source)) in sources.iter().enumerate() {
        let program = &parsed_programs[i];
        let file_module = crate::analyzer::type_collector::wj_file_to_module_path(&src_base, file)
            .unwrap_or_default();
        global_float_inference.set_current_file_module_path(file_module);
        global_float_inference.infer_program(program);
    }

    super::bail_on_inference_errors(&global_float_inference.errors, "Float", None)?;

    // Step 4A2: Global int inference pass (same architecture as float)
    let mut global_int_inference = IntInference::new();
    global_int_inference.set_global_function_signatures(global_float_signatures.clone());
    global_int_inference.set_global_struct_field_types(&global_struct_fields);
    global_int_inference.set_struct_defining_module_paths((*struct_defining_module_paths).clone());
    global_int_inference.set_module_re_exports(module_re_exports);

    for (i, (file, _source)) in sources.iter().enumerate() {
        let program = &parsed_programs[i];
        let file_module = crate::analyzer::type_collector::wj_file_to_module_path(&src_base, file)
            .unwrap_or_default();
        global_int_inference.set_current_file_module_path(file_module);
        global_int_inference.infer_program(program);
    }

    super::bail_on_inference_errors(&global_int_inference.errors, "Int", None)?;
    profile_phase("Step 4A: Float/Int inference", step4a_start);

    let type_defining_modules =
        super::dependency_resolution::build_type_defining_modules_for_library(&sources, &src_base)?;
    let extern_submodule_qualifiers =
        super::dependency_resolution::build_extern_submodule_qualifier_map(&sources, &src_base)?;

    // Step 4B-pre: Build GLOBAL analyzed_trait_methods across ALL files.
    // Each file's Analyzer is fresh, so cross-file trait info (e.g. RenderPort defined
    // in render_port.wj but implemented in voxel_gpu_renderer.wj) would be missing
    // if we only used per-file analysis. This step mirrors main.rs's finalize_trait_inference.
    //
    // Runs on a separate thread with a large stack because the merged program (~3000 items)
    // can produce deep recursive analysis.
    let step4b_pre_start = Instant::now();
    let global_analyzed_trait_methods = {
        let global_copy_structs_clone = (*global_copy_structs).clone();
        // The trait inference thread needs its own parsed programs since Program borrows
        // from Parser arenas (can't send references across threads). Re-parse on thread.
        let sources_for_thread: Vec<(PathBuf, String)> = sources.clone();

        let handle = std::thread::Builder::new()
            .name("trait-inference".to_string())
            .stack_size(64 * 1024 * 1024)
            .spawn(move || -> Result<HashMap<String, HashMap<String, crate::analyzer::AnalyzedFunction<'static>>>, String> {
                let mut shared_analyzer = Analyzer::new_with_copy_structs(global_copy_structs_clone);

                let mut thread_parsers: Vec<Parser> = Vec::with_capacity(sources_for_thread.len());
                let mut thread_programs: Vec<crate::parser::Program<'static>> = Vec::with_capacity(sources_for_thread.len());
                for (file, source) in &sources_for_thread {
                    let mut lexer = Lexer::new(source);
                    let tokens = lexer.tokenize_with_locations();
                    let mut parser = Parser::new_with_source(tokens, file.to_string_lossy().to_string(), source.clone());
                    let program = parser.parse().map_err(|e| format!("Parse error in {}: {}", file.display(), e))?;
                    thread_programs.push(program);
                    thread_parsers.push(parser);
                }

                for program in &thread_programs {
                    shared_analyzer.register_traits_from_program(program)
                        .unwrap_or_else(|e| eprintln!("Trait registration warning: {}", e));
                }

                let mut all_items = Vec::new();
                for program in thread_programs {
                    all_items.extend(program.items);
                }

                let merged_program = crate::parser::Program { items: all_items };
                shared_analyzer.infer_trait_signatures_from_impls(&merged_program)?;
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
    profile_phase("Step 4B-pre: Trait inference thread", step4b_pre_start);

    // Step 4B: Final analysis + code generation (using shared global_float_inference)
    //
    // TWO-PHASE APPROACH: First analyze ALL files to build a consistent final registry,
    // then generate code using that registry. This ensures cross-module call sites see
    // the same Phase 2 optimized signatures as the function definitions.
    //
    // INCREMENTAL (Phase 2): Compute dirty files to skip codegen for unchanged sources.
    // Analysis still runs for ALL files (needed for metadata.json convergence),
    // but the expensive codegen + write is skipped for files whose source hasn't changed.
    let (dirty_indices, skipped_count) =
        super::cache_management::compute_dirty_files(&sources, &src_base, output);
    let dirty_set: HashSet<usize> = dirty_indices.into_iter().collect();
    if skipped_count > 0 {
        eprintln!(
            "⚡ Incremental: {}/{} files unchanged, regenerating {} dirty files",
            skipped_count,
            sources.len(),
            dirty_set.len()
        );
    }

    // Phase 1: Analyze ALL files and collect per-file results + build final registry.
    // The final registry has Phase 2 optimizations (str_ref, etc.) from ALL files,
    // so cross-module call sites will see consistent signatures.
    struct PerFileAnalysis<'a> {
        analyzed_functions: Vec<crate::analyzer::AnalyzedFunction<'a>>,
        registry: SignatureRegistry,
        merged_trait_methods: HashMap<String, HashMap<String, crate::analyzer::AnalyzedFunction<'a>>>,
        copy_structs: Vec<String>,
        output_file: PathBuf,
    }

    let mut per_file_analyses: Vec<PerFileAnalysis<'static>> = Vec::with_capacity(sources.len());
    let mut final_global_registry = global_registry.clone();
    let mut local_converged_sigs: HashMap<String, crate::analyzer::FunctionSignature> =
        HashMap::new();

    // Pre-build stripped programs so they live long enough for analysis references.
    let mut stripped_programs: Vec<Option<crate::parser::Program<'static>>> =
        vec![None; sources.len()];
    for (i, (file, _source)) in sources.iter().enumerate() {
        let needs_strip = file
            .parent()
            .and_then(|p| filtered_modules_by_dir.get(p))
            .is_some();
        if needs_strip {
            let mut cloned = parsed_programs[i].clone();
            if let Some(parent_dir) = file.parent() {
                if let Some(filtered_names) = filtered_modules_by_dir.get(parent_dir) {
                    cloned.items = super::strip_filtered_mod_items(cloned.items, filtered_names);
                }
            }
            stripped_programs[i] = Some(cloned);
        }
    }

    let step4b_phase1_start = Instant::now();
    for (i, (file, _source)) in sources.iter().enumerate() {
        let program: &crate::parser::Program<'static> = stripped_programs[i]
            .as_ref()
            .unwrap_or(&parsed_programs[i]);

        let mut analyzer = Analyzer::for_library_pass(
            (*global_copy_structs).clone(),
            global_struct_fields.clone(),
            struct_defining_module_paths.clone(),
        );

        if let Err(e) =
            crate::linter::rust_leakage::run_lint_if_enabled(enable_lint, file, program)
        {
            deferred_lint_errors.push(e);
        }

        analyzer
            .register_traits_from_program(program)
            .unwrap_or_else(|e| eprintln!("Trait registration warning: {}", e));

        let (analyzed_functions, registry, _) = analyzer
            .analyze_program_with_global_signatures(program, &global_registry)
            .map_err(|e| anyhow::anyhow!("Final analysis error: {}", e))?;

        analyzer
            .infer_trait_signatures_from_impls(program)
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        let mut merged_trait_methods = analyzer.analyzed_trait_methods.clone();
        for (trait_name, methods) in &global_analyzed_trait_methods {
            let entry = merged_trait_methods.entry(trait_name.clone()).or_default();
            for (method_name, method_analysis) in methods {
                entry.insert(method_name.clone(), method_analysis.clone());
            }
        }

        let output_file =
            crate::project_paths::resolve_wj_output_path_library(&src_base, file, output)?;
        super::ensure_output_parent_dir(&output_file)?;

        // Merge this file's final signatures into final_global_registry.
        // This captures Phase 2 optimizations (str_ref params, etc.)
        let file_stem = file.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let file_module =
            crate::analyzer::type_collector::wj_file_to_module_path(&src_base, file)
                .unwrap_or_default();
        let module_path = file_module.join("::");
        final_global_registry.merge(&registry);
        final_global_registry.register_module_aliases(&registry, file_stem, &module_path);

        per_file_analyses.push(PerFileAnalysis {
            analyzed_functions,
            registry,
            merged_trait_methods,
            copy_structs: analyzer.get_copy_structs(),
            output_file,
        });
    }
    profile_phase("Step 4B Phase 1: Final analysis", step4b_phase1_start);

    // Phase 2: Code generation using the consistent final_global_registry.
    let step4b_phase2_start = Instant::now();
    for (i, (file, _source)) in sources.iter().enumerate() {
        let analysis = &per_file_analyses[i];
        let program: &crate::parser::Program<'static> = stripped_programs[i]
            .as_ref()
            .unwrap_or(&parsed_programs[i]);

        // Build full registry: final global (with Phase 2 from ALL files) + per-file local
        let mut full_registry = final_global_registry.clone();
        full_registry.merge(&analysis.registry);
        {
            let mut tmp_analyzer = Analyzer::for_library_pass(
                (*global_copy_structs).clone(),
                global_struct_fields.clone(),
                struct_defining_module_paths.clone(),
            );
            tmp_analyzer.analyzed_trait_methods = analysis.merged_trait_methods.clone();
            tmp_analyzer.register_trait_methods_in_registry(
                &analysis.merged_trait_methods,
                &mut full_registry,
            );
        }

        let registry_snapshot = full_registry.clone();
        for (name, sig) in &registry_snapshot.signatures {
            if !sig.param_ownership.is_empty()
                && (crate::metadata::signature_targets_local_struct(name, &local_struct_names)
                    || (!name.contains("::") && crate_metadata.functions.contains_key(name)))
            {
                local_converged_sigs.insert(name.clone(), sig.clone());
            }
        }

        if !dirty_set.contains(&i) && analysis.output_file.exists() {
            continue;
        }

        let mut codegen = CodeGenerator::new_for_module(full_registry, target);
        codegen.set_copy_types_registry((*global_copy_structs).clone());
        codegen.set_non_copy_types_registry(global_non_copy_types.clone());
        codegen.set_global_struct_field_types((*global_struct_fields).clone());
        codegen.set_output_file(&analysis.output_file);
        codegen.set_source_file(file);
        codegen.set_library_source_root(src_base.clone());
        codegen.set_type_defining_modules(type_defining_modules.clone());
        codegen.set_extern_submodule_qualifiers(extern_submodule_qualifiers.clone());
        codegen.set_analyzed_trait_methods(analysis.merged_trait_methods.clone());
        codegen.set_float_inference(global_float_inference.clone());
        codegen.set_int_inference(global_int_inference.clone());

        super::apply_inferred_bounds_to_codegen(&mut codegen, program);
        let mut registry_snapshot_mut = registry_snapshot;
        super::write_generated_rust_and_meta(
            &mut codegen,
            program,
            &analysis.analyzed_functions,
            &mut registry_snapshot_mut,
            &analysis.output_file,
            file,
            analysis.copy_structs.clone(),
            target,
        )?;
    }
    profile_phase("Step 4B Phase 2: Code generation", step4b_phase2_start);

    // Emit metadata.json — refresh ONLY locally-defined function signatures from the
    // converged registry. Dumping the full merged registry (engine + game) creates
    // circular 11MB metadata pollution on the next build.
    if library && (!crate_metadata.structs.is_empty() || !crate_metadata.functions.is_empty())
    {
        let local_keys: Vec<String> = crate_metadata.functions.keys().cloned().collect();
        for name in local_keys {
            let sig = local_converged_sigs
                .get(&name)
                .or_else(|| {
                    local_converged_sigs
                        .iter()
                        .find(|(k, _)| k.ends_with(&format!("::{name}")))
                        .map(|(_, v)| v)
                })
                .or_else(|| resolve_converged_local_signature(&global_registry, &name));
            if let Some(sig) = sig {
                let (is_associated, parent_type) =
                    if let Some(struct_name) = crate::metadata::struct_name_from_method_key(&name) {
                        (true, Some(struct_name.to_string()))
                    } else {
                        (false, None)
                    };
                crate_metadata.functions.insert(
                    name,
                    metadata_function_sig_from_analyzer(sig, is_associated, parent_type),
                );
            }
        }
        super::write_crate_metadata_json(output, &crate_metadata)?;
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
    super::generate_cargo_manifests(base_path, output, target, true)?;

    // Record the compiler version so the next build can detect upgrades.
    let _ = super::cache_management::write_compiler_stamp(output);

    if !deferred_lint_errors.is_empty() {
        return Err(anyhow::anyhow!(
            "Rust leakage errors:\n{}",
            deferred_lint_errors.join("\n")
        ));
    }

    Ok(())
}
