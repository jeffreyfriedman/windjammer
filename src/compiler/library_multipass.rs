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

    if sources.is_empty() {
        return Ok(());
    }

    // PERFORMANCE: Parse all files once upfront and reuse ASTs across all pipeline
    // phases. Previously each file was re-parsed 9-10 times. The Parser owns arenas
    // (expr_arena, stmt_arena, pattern_arena) that Program references borrow from,
    // so parsers must be kept alive as long as their programs are used.
    let mut parsers: Vec<Parser> = Vec::with_capacity(sources.len());
    let mut parsed_programs: Vec<crate::parser::Program<'static>> = Vec::with_capacity(sources.len());
    for (file, source) in &sources {
        let (parser, program) = super::parse_wj_source(file, source)?;
        parsed_programs.push(program);
        parsers.push(parser);
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
        super::library_copy_registry::collect_global_copy_structs_for_library(&sources);
    let global_non_copy_enums =
        super::library_copy_registry::collect_non_copy_enums_for_library(&sources);

    // Load Copy structs AND function signatures from dependency crate metadata.
    // Function signatures provide ownership info for cross-crate calls (e.g.,
    // voxelgrid_to_svo64_flat from windjammer-game-core). The metadata includes
    // module-qualified names for unambiguous lookup.
    let dep_roots =
        super::dependency_resolution::find_dependency_metadata_roots(&src_base, external_paths);
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
        global_registry.register_module_aliases(&registry, file_stem, &module_path);
    }

    // Step 3: Global multi-pass iteration until convergence
    // Wrap read-only data in Arc for O(1) sharing across all files (avoids O(n) deep clones).
    let global_struct_fields = std::sync::Arc::new(global_struct_fields);
    let struct_defining_module_paths = std::sync::Arc::new(struct_defining_module_paths);
    let global_copy_structs = std::sync::Arc::new(global_copy_structs);

    const MAX_GLOBAL_PASSES: usize = 10;
    let mut pass_number = 1;

    loop {
        let mut new_registry = global_registry.clone();

        // Re-analyze ALL files with current global registry
        for (i, (file, _source)) in sources.iter().enumerate() {
            let program = &parsed_programs[i];

            let mut analyzer = Analyzer::for_library_pass(
                (*global_copy_structs).clone(),
                global_struct_fields.clone(),
                struct_defining_module_paths.clone(),
            );
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

    // Step 4B: Final analysis + code generation (using shared global_float_inference)
    let mut local_converged_sigs: HashMap<String, crate::analyzer::FunctionSignature> =
        HashMap::new();
    for (i, (file, _source)) in sources.iter().enumerate() {
        // Use cached parse. For files that need mod item stripping, clone just the items vec.
        let needs_strip = file.parent()
            .and_then(|p| filtered_modules_by_dir.get(p))
            .is_some();

        let program: std::borrow::Cow<'_, crate::parser::Program<'static>> = if needs_strip {
            let mut cloned = parsed_programs[i].clone();
            if let Some(parent_dir) = file.parent() {
                if let Some(filtered_names) = filtered_modules_by_dir.get(parent_dir) {
                    cloned.items = super::strip_filtered_mod_items(cloned.items, filtered_names);
                }
            }
            std::borrow::Cow::Owned(cloned)
        } else {
            std::borrow::Cow::Borrowed(&parsed_programs[i])
        };

        let mut analyzer = Analyzer::for_library_pass(
            (*global_copy_structs).clone(),
            global_struct_fields.clone(),
            struct_defining_module_paths.clone(),
        );

        crate::linter::rust_leakage::run_lint_if_enabled(enable_lint, file, &program);

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
        // In library mode, mod.wj output goes to _mod_items.rs so --module-file doesn't overwrite it.
        let output_file =
            crate::project_paths::resolve_wj_output_path_library(&src_base, file, output)?;
        super::ensure_output_parent_dir(&output_file)?;

        // Library-style modules: `use super::*` + automatic sibling `use super::Type` imports.
        // Use the global registry (all cross-file signatures) merged with the per-file registry
        // so that method calls to other files' types resolve correctly for auto-borrowing.
        let mut full_registry = global_registry.clone();
        full_registry.merge(&registry);
        analyzer.register_trait_methods_in_registry(&merged_trait_methods, &mut full_registry);
        let mut registry_snapshot = full_registry.clone();
        for (name, sig) in &registry_snapshot.signatures {
            if !sig.param_ownership.is_empty()
                && (crate::metadata::signature_targets_local_struct(name, &local_struct_names)
                    || (!name.contains("::") && crate_metadata.functions.contains_key(name)))
            {
                local_converged_sigs.insert(name.clone(), sig.clone());
            }
        }
        let mut codegen = CodeGenerator::new_for_module(full_registry, target);
        codegen.set_copy_types_registry((*global_copy_structs).clone());
        codegen.set_non_copy_types_registry(global_non_copy_enums.clone());
        codegen.set_global_struct_field_types((*global_struct_fields).clone());
        codegen.set_output_file(&output_file);
        codegen.set_source_file(file);
        codegen.set_library_source_root(src_base.clone());
        codegen.set_type_defining_modules(type_defining_modules.clone());
        codegen.set_extern_submodule_qualifiers(extern_submodule_qualifiers.clone());
        codegen.set_analyzed_trait_methods(merged_trait_methods);
        codegen.set_float_inference(global_float_inference.clone());
        codegen.set_int_inference(global_int_inference.clone());

        super::apply_inferred_bounds_to_codegen(&mut codegen, &program);
        super::write_generated_rust_and_meta(
            &mut codegen,
            &program,
            &analyzed_functions,
            &mut registry_snapshot,
            &output_file,
            file,
            analyzer.get_copy_structs(),
            target,
        )?;
    }

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

    Ok(())
}
