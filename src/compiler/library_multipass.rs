//! Multi-file library build with global multi-pass analysis.

use crate::analyzer::{Analyzer, SignatureRegistry};
use crate::codegen::rust::CodeGenerator;
use crate::metadata::{metadata_function_sig_from_analyzer, CrateMetadata};
use crate::parser::ast::core::Item;
use crate::parser::Parser;
use crate::type_inference::{FloatInference, IntInference};
use crate::CompilationTarget;
use anyhow::Result;
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Instant;

fn profiling_enabled() -> bool {
    std::env::var("WJ_PROFILE").is_ok_and(|v| v == "1" || v == "true")
}

fn profile_phase(phase: &str, start: Instant) {
    if profiling_enabled() {
        eprintln!("[wj-profile] {}: {}ms", phase, start.elapsed().as_millis());
    }
}

/// Remove shader files from parsed sources (uses upfront parse — no re-tokenize).
fn filter_shader_files(
    sources: &mut Vec<(PathBuf, String)>,
    parsers: &mut Vec<Parser>,
    parsed_programs: &mut Vec<crate::parser::Program<'static>>,
) -> (HashMap<PathBuf, HashSet<String>>, usize) {
    let mut filtered_modules_by_dir: HashMap<PathBuf, HashSet<String>> = HashMap::new();
    let mut removed_stems: HashSet<PathBuf> = HashSet::new();
    let mut shader_count = 0usize;

    let mut keep_indices: Vec<usize> = Vec::new();
    for i in 0..parsed_programs.len() {
        let file = &sources[i].0;
        if super::is_shader_file(&parsed_programs[i]) {
            removed_stems.insert(file.clone());
            if let Some(parent) = file.parent() {
                if let Some(stem) = file.file_stem().and_then(|s| s.to_str()) {
                    filtered_modules_by_dir
                        .entry(parent.to_path_buf())
                        .or_default()
                        .insert(stem.to_string());
                }
            }
            shader_count += 1;
        } else {
            keep_indices.push(i);
        }
    }

    if !removed_stems.is_empty() {
        let mut pass2_keep: Vec<usize> = Vec::new();
        for &i in &keep_indices {
            let file = &sources[i].0;
            let is_mod = file
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n == "mod.wj")
                .unwrap_or(false);
            if !is_mod {
                pass2_keep.push(i);
                continue;
            }
            let parent = match file.parent() {
                Some(p) => p,
                None => {
                    pass2_keep.push(i);
                    continue;
                }
            };
            let program = &parsed_programs[i];
            let has_non_mod_items = program
                .items
                .iter()
                .any(|item| !matches!(item, Item::Mod { .. }));
            if has_non_mod_items {
                pass2_keep.push(i);
                continue;
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
                if let Some(dir_name) = parent.file_name().and_then(|n| n.to_str()) {
                    if let Some(grandparent) = parent.parent() {
                        filtered_modules_by_dir
                            .entry(grandparent.to_path_buf())
                            .or_default()
                            .insert(dir_name.to_string());
                    }
                }
                shader_count += 1;
            } else {
                pass2_keep.push(i);
            }
        }
        keep_indices = pass2_keep;
    }

    if keep_indices.len() != sources.len() {
        let mut drop_mask = vec![true; sources.len()];
        for &i in &keep_indices {
            drop_mask[i] = false;
        }
        for i in (0..sources.len()).rev() {
            if drop_mask[i] {
                sources.remove(i);
                parsers.remove(i);
                parsed_programs.remove(i);
            }
        }
    }

    (filtered_modules_by_dir, shader_count)
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

    let parse_start = Instant::now();
    // PERFORMANCE: Parse all files once upfront and reuse ASTs across all pipeline
    // phases. Shader filtering uses parsed ASTs (no duplicate tokenization).
    let mut parsers: Vec<Parser> = Vec::with_capacity(sources.len());
    let mut parsed_programs: Vec<crate::parser::Program<'static>> =
        Vec::with_capacity(sources.len());
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

    let shader_filter_start = Instant::now();
    let (filtered_modules_by_dir, shader_count) =
        filter_shader_files(&mut sources, &mut parsers, &mut parsed_programs);
    if shader_count > 0 {
        eprintln!(
            "  Skipped {} shader file(s) from Rust pipeline (use WJSL target for GPU shaders)",
            shader_count
        );
    }
    profile_phase("Shader filter (parsed AST)", shader_filter_start);
    profile_phase(
        "Step 0+1: Source reading + parse + shader filter",
        phase_start,
    );

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

    // Dependency metadata roots (needed for both freshness check and analysis)
    let dep_roots =
        super::dependency_resolution::find_dependency_metadata_roots(&src_base, external_paths);
    let dep_epoch_snapshot = super::incremental::dep_metadata_epoch(&dep_roots);

    let dependency_graph =
        super::incremental::DependencyGraph::build(&sources, &parsed_programs, &src_base);

    // INCREMENTAL: Whole-crate fast path — if no .wj file has changed and dep
    // metadata is also unchanged, skip the entire transpilation pipeline.
    if super::cache_management::all_sources_fresh(&sources, &src_base, output, &dep_roots) {
        let stale = super::cache_management::find_stale_codegen_outputs_with_dep_epoch(
            &sources,
            &src_base,
            output,
            &dep_roots,
            Some(dep_epoch_snapshot),
        );
        if !stale.is_empty() {
            return Err(anyhow::anyhow!(
                "incremental skip rejected: {} file(s) have stale generated output \
                 (mtime/fingerprint mismatch): {:?}",
                stale.len(),
                stale.iter().take(5).collect::<Vec<_>>()
            ));
        }
        let user_count = sources.len() - needed_stdlib_modules.len();
        eprintln!(
            "✓ All {} source files up to date, skipping transpilation",
            user_count
        );
        return Ok(());
    }

    if !super::cache_management::is_compiler_stamp_fresh(output) {
        eprintln!("⟳ Compiler changed — re-transpiling all sources");
    }

    let reanalysis_set = super::incremental::compute_reanalysis_set(
        &sources,
        &src_base,
        output,
        &dep_roots,
        &dependency_graph,
    );
    if reanalysis_set.len() < sources.len() {
        eprintln!(
            "⚡ Incremental analysis: {}/{} files need re-analysis",
            reanalysis_set.len(),
            sources.len()
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

    let stub_registry_start = Instant::now();
    // Collect declaration stubs + float/struct registries from every file (parallel scan,
    // ordered merge). Feeds the global ownership convergence passes (Step 3+).
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
    global_struct_fields.extend(crate::metadata::load_merged_external_struct_fields(
        external_paths,
        Some(&local_struct_names),
    ));

    // Parallel per-file Step 2 scan; merge in source order for deterministic collisions.
    let mut indexed_contributions: Vec<(
        usize,
        super::declaration_stub_registry::DeclarationStubContribution,
    )> = (0..sources.len())
        .into_par_iter()
        .map(|i| {
            let (file, _) = &sources[i];
            (
                i,
                super::declaration_stub_registry::collect_per_file_declaration_stubs(
                    &src_base,
                    file,
                    &parsed_programs[i],
                ),
            )
        })
        .collect();
    indexed_contributions.sort_by_key(|(i, _)| *i);

    let ordered: Vec<(
        PathBuf,
        super::declaration_stub_registry::DeclarationStubContribution,
    )> = indexed_contributions
        .iter()
        .map(|(i, contrib)| (sources[*i].0.clone(), contrib.clone()))
        .collect();
    let source_indices: Vec<usize> = indexed_contributions.iter().map(|(i, _)| *i).collect();

    super::declaration_stub_registry::merge_declaration_stub_contributions(
        &mut global_registry,
        &mut global_float_signatures,
        &mut global_struct_fields,
        &mut struct_defining_module_paths,
        &ordered,
        &mut crate_metadata,
        &parsed_programs,
        &source_indices,
    );
    profile_phase("Declaration stub registry (parallel)", stub_registry_start);

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

        // Re-analyze ALL files with current global registry (parallel per file).
        let file_registries: Vec<SignatureRegistry> = (0..sources.len())
            .into_par_iter()
            .map(|i| {
                if !reanalysis_set.contains(&i) {
                    return Ok(SignatureRegistry::empty());
                }
                let (file, _source) = &sources[i];
                let program = &parsed_programs[i];
                let mut analyzer = Analyzer::for_library_pass(
                    global_copy_structs.clone(),
                    global_struct_fields.clone(),
                    struct_defining_module_paths.clone(),
                );
                analyzer.convergence_only = true;
                analyzer
                    .analyze_program_with_global_signatures(program, &global_registry)
                    .map(|(_, file_registry, _)| file_registry)
                    .map_err(|e| {
                        format!(
                            "Analysis error in pass {} for {}: {}",
                            pass_number,
                            file.display(),
                            e
                        )
                    })
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| anyhow::anyhow!(e))?;

        for (i, (file, _source)) in sources.iter().enumerate() {
            let file_registry = &file_registries[i];
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
                            new_registry
                                .add_function(format!("{}::{}", file_stem, name), sig.clone());
                        }
                        if !module_path.is_empty() {
                            new_registry
                                .add_function(format!("{}::{}", module_path, name), sig.clone());
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

    // Step 4A: Global float + int inference (parallel per-file collection; passes run concurrently)
    let step4a_start = Instant::now();
    let module_re_exports_for_int = module_re_exports.clone();
    let (global_float_inference, global_int_inference) = std::thread::scope(|scope| {
        let float_handle = scope.spawn(|| {
            run_parallel_float_inference(
                &sources,
                &parsed_programs,
                &src_base,
                external_paths,
                &global_float_signatures,
                &global_struct_fields,
                &struct_defining_module_paths,
                module_re_exports,
            )
        });
        let int_handle = scope.spawn(|| {
            run_parallel_int_inference(
                &sources,
                &parsed_programs,
                &src_base,
                &global_float_signatures,
                &global_struct_fields,
                &struct_defining_module_paths,
                module_re_exports_for_int,
            )
        });
        (
            float_handle.join().expect("float inference thread"),
            int_handle.join().expect("int inference thread"),
        )
    });

    super::bail_on_inference_errors(&global_float_inference.errors, "Float", None)?;
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
    // Reuse upfront parse on main thread (parsers stay alive). Avoids 649-file re-tokenize.
    let global_analyzed_trait_methods = {
        let mut shared_analyzer = Analyzer::new_with_copy_structs((*global_copy_structs).clone());

        for program in &parsed_programs {
            shared_analyzer
                .register_traits_from_program(program)
                .unwrap_or_else(|e| eprintln!("Trait registration warning: {}", e));
        }

        let mut all_items = Vec::new();
        for program in &parsed_programs {
            all_items.extend(program.items.iter().cloned());
        }

        let merged_program = crate::parser::Program { items: all_items };
        match shared_analyzer.infer_trait_signatures_from_impls(&merged_program, &global_registry) {
            Ok(()) => shared_analyzer.analyzed_trait_methods.clone(),
            Err(e) => {
                eprintln!("Cross-file trait inference warning: {}", e);
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
        super::cache_management::compute_dirty_files(&sources, &src_base, output, &dep_roots);
    let dirty_set: HashSet<usize> = dirty_indices.into_iter().collect();
    if skipped_count > 0 {
        eprintln!(
            "⚡ Incremental: {}/{} files unchanged, regenerating {} dirty files",
            skipped_count,
            sources.len(),
            dirty_set.len()
        );
    }

    // Phase 1: Analyze files in bounded batches to build final registry without retaining
    // all per-file analysis in memory (664+ engine files OOM with exit 137 otherwise).
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
    struct Phase1RegistryResult {
        index: usize,
        registry: SignatureRegistry,
        file_stem: String,
        module_path: String,
        lint_errors: Vec<String>,
    }

    let global_registry_for_phase1 = global_registry.clone();
    let global_analyzed_trait_methods_arc = std::sync::Arc::new(global_analyzed_trait_methods);

    for batch_start in (0..sources.len()).step_by(STEP4B_REGISTRY_BATCH_SIZE) {
        let batch_end = (batch_start + STEP4B_REGISTRY_BATCH_SIZE).min(sources.len());
        let batch: Vec<usize> = (batch_start..batch_end).collect();
        let mut batch_results: Vec<Phase1RegistryResult> = batch
            .into_par_iter()
            .map(|i| -> Result<Phase1RegistryResult, String> {
                let analysis = analyze_file_for_step4b(
                    i,
                    &sources,
                    &parsed_programs,
                    &stripped_programs,
                    &src_base,
                    output,
                    &global_registry_for_phase1,
                    &global_copy_structs,
                    &global_struct_fields,
                    &struct_defining_module_paths,
                    &global_analyzed_trait_methods_arc,
                    enable_lint,
                )?;
                Ok(Phase1RegistryResult {
                    index: i,
                    registry: analysis.registry,
                    file_stem: analysis.file_stem,
                    module_path: analysis.module_path,
                    lint_errors: analysis.lint_errors,
                })
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| anyhow::anyhow!(e))?;
        batch_results.sort_by_key(|r| r.index);
        for result in batch_results {
            deferred_lint_errors.extend(result.lint_errors);
            final_global_registry.merge(&result.registry);
            final_global_registry.register_module_aliases(
                &result.registry,
                &result.file_stem,
                &result.module_path,
            );
        }
    }
    let final_global_registry = std::sync::Arc::new(final_global_registry);
    profile_phase(
        "Step 4B Phase 1: Final analysis (batched parallel)",
        step4b_phase1_start,
    );

    // Phase 2: Sequential analyze + codegen — one file in memory at a time.
    let step4b_phase2_start = Instant::now();
    for (i, (file, _source)) in sources.iter().enumerate() {
        // Stdlib injects (outside src_base) participate in analysis only — no crate output.
        if file.strip_prefix(&src_base).is_err() {
            continue;
        }
        let analysis = analyze_file_for_step4b(
            i,
            &sources,
            &parsed_programs,
            &stripped_programs,
            &src_base,
            output,
            final_global_registry.as_ref(),
            &global_copy_structs,
            &global_struct_fields,
            &struct_defining_module_paths,
            &global_analyzed_trait_methods_arc,
            enable_lint,
        )
        .map_err(|e| anyhow::anyhow!(e))?;
        let program: &crate::parser::Program<'static> =
            stripped_programs[i].as_ref().unwrap_or(&parsed_programs[i]);

        // Build full registry: shared global (Arc) + per-file local overlay
        let mut full_registry = final_global_registry.as_ref().clone();
        full_registry.merge(&analysis.registry);
        {
            let mut tmp_analyzer = Analyzer::for_library_pass(
                global_copy_structs.clone(),
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
            // Guardrail: never skip codegen if fingerprint validation would fail.
            let (_, source) = &sources[i];
            if !super::cache_management::is_library_codegen_cache_valid(
                source,
                file,
                &analysis.output_file,
                &src_base,
                output,
                &dep_roots,
            ) {
                return Err(anyhow::anyhow!(
                    "incremental codegen skip would leave stale output for {} — \
                     source changed but cache appeared fresh; this is a compiler bug",
                    file.display()
                ));
            }
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
            &dep_roots,
            Some(dep_epoch_snapshot),
        )?;
    }
    profile_phase("Step 4B Phase 2: Code generation", step4b_phase2_start);

    // Emit metadata.json — refresh ONLY locally-defined function signatures from the
    // converged registry. Dumping the full merged registry (engine + game) creates
    // circular 11MB metadata pollution on the next build.
    if library && (!crate_metadata.structs.is_empty() || !crate_metadata.functions.is_empty()) {
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
                .or_else(|| {
                    resolve_converged_local_signature(final_global_registry.as_ref(), &name)
                });
            if let Some(sig) = sig {
                let (is_associated, parent_type) = if let Some(struct_name) =
                    crate::metadata::struct_name_from_method_key(&name)
                {
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
        if let Some(project_root) =
            crate::rust_integration_tests::find_project_root_with_tests(src_base.as_path())
        {
            let _ = crate::rust_integration_tests::sync_rust_integration_tests(&project_root);
        }
    }

    // Always (re)generate Cargo.toml in the output directory for Rust builds.
    super::generate_cargo_manifests(base_path, output, target, true)?;

    // Record the compiler version so the next build can detect upgrades.
    let _ = super::cache_management::write_compiler_stamp(output);

    let stale = super::cache_management::find_stale_codegen_outputs_with_dep_epoch(
        &sources,
        &src_base,
        output,
        &dep_roots,
        Some(dep_epoch_snapshot),
    );
    if !stale.is_empty() {
        return Err(anyhow::anyhow!(
            "build finished but {} file(s) still have stale generated output — \
             incremental codegen bug; affected files: {:?}",
            stale.len(),
            stale.iter().take(10).collect::<Vec<_>>()
        ));
    }

    if !deferred_lint_errors.is_empty() {
        return Err(anyhow::anyhow!(
            "Rust leakage errors:\n{}",
            deferred_lint_errors.join("\n")
        ));
    }

    Ok(())
}

/// Cap parallel Step 4B registry scans — retaining all per-file analysis for 664+ files OOMs.
const STEP4B_REGISTRY_BATCH_SIZE: usize = 16;

struct Step4bFileAnalysis {
    registry: SignatureRegistry,
    analyzed_functions: Vec<crate::analyzer::AnalyzedFunction<'static>>,
    merged_trait_methods:
        HashMap<String, HashMap<String, crate::analyzer::AnalyzedFunction<'static>>>,
    copy_structs: Vec<String>,
    output_file: PathBuf,
    file_stem: String,
    module_path: String,
    lint_errors: Vec<String>,
}

#[allow(clippy::too_many_arguments)]
fn analyze_file_for_step4b(
    i: usize,
    sources: &[(PathBuf, String)],
    parsed_programs: &[crate::parser::Program<'static>],
    stripped_programs: &[Option<crate::parser::Program<'static>>],
    src_base: &Path,
    output: &Path,
    global_registry: &SignatureRegistry,
    global_copy_structs: &std::sync::Arc<std::collections::HashSet<String>>,
    global_struct_fields: &std::sync::Arc<
        HashMap<String, HashMap<String, crate::parser::ast::types::Type>>,
    >,
    struct_defining_module_paths: &std::sync::Arc<HashMap<String, Vec<Vec<String>>>>,
    global_analyzed_trait_methods: &std::sync::Arc<
        HashMap<String, HashMap<String, crate::analyzer::AnalyzedFunction<'static>>>,
    >,
    enable_lint: bool,
) -> Result<Step4bFileAnalysis, String> {
    let (file, _source) = &sources[i];
    let program: &crate::parser::Program<'static> =
        stripped_programs[i].as_ref().unwrap_or(&parsed_programs[i]);

    let mut lint_errors = Vec::new();
    if let Err(e) = crate::linter::rust_leakage::run_lint_if_enabled(enable_lint, file, program) {
        lint_errors.push(e);
    }

    let mut analyzer = Analyzer::for_library_pass(
        global_copy_structs.clone(),
        global_struct_fields.clone(),
        struct_defining_module_paths.clone(),
    );

    analyzer
        .register_traits_from_program(program)
        .map_err(|e| format!("Trait registration: {}", e))?;

    let (analyzed_functions, registry, _) = analyzer
        .analyze_program_with_global_signatures(program, global_registry)
        .map_err(|e| format!("Final analysis error: {}", e))?;

    analyzer
        .infer_trait_signatures_from_impls(program, &registry)
        .map_err(|e| e.to_string())?;

    let mut merged_trait_methods = analyzer.analyzed_trait_methods.clone();
    for (trait_name, methods) in global_analyzed_trait_methods.iter() {
        let entry = merged_trait_methods.entry(trait_name.clone()).or_default();
        for (method_name, method_analysis) in methods {
            entry.insert(method_name.clone(), method_analysis.clone());
        }
    }

    let output_file = crate::project_paths::resolve_wj_output_path_library(src_base, file, output)
        .map_err(|e| e.to_string())?;
    super::ensure_output_parent_dir(&output_file).map_err(|e| e.to_string())?;

    let file_stem = file
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();
    let file_module =
        crate::analyzer::type_collector::wj_file_to_module_path(src_base, file).unwrap_or_default();
    let module_path = file_module.join("::");

    Ok(Step4bFileAnalysis {
        registry,
        analyzed_functions,
        merged_trait_methods,
        copy_structs: analyzer.get_copy_structs(),
        output_file,
        file_stem,
        module_path,
        lint_errors,
    })
}

#[allow(clippy::too_many_arguments)]
fn run_parallel_float_inference(
    sources: &[(PathBuf, String)],
    parsed_programs: &[crate::parser::Program<'static>],
    src_base: &Path,
    external_paths: &std::collections::HashMap<String, PathBuf>,
    global_float_signatures: &HashMap<
        String,
        (Vec<crate::parser::Type>, Option<crate::parser::Type>),
    >,
    global_struct_fields: &HashMap<String, HashMap<String, crate::parser::Type>>,
    struct_defining_module_paths: &HashMap<String, Vec<Vec<String>>>,
    module_re_exports: HashMap<String, HashMap<String, String>>,
) -> FloatInference {
    let mut global = FloatInference::new();
    if !external_paths.is_empty() {
        global.set_external_crate_metadata_paths(external_paths);
    }
    global.set_global_function_signatures(global_float_signatures.clone());
    global.set_global_struct_field_types(global_struct_fields);
    global.set_struct_defining_module_paths(struct_defining_module_paths.clone());
    global.set_module_re_exports(module_re_exports);
    global.reset_imported_type_registry();

    for (i, (file, _source)) in sources.iter().enumerate() {
        let file_module = crate::analyzer::type_collector::wj_file_to_module_path(src_base, file)
            .unwrap_or_default();
        global.set_current_file_module_path(file_module);
        global.prepare_program(&parsed_programs[i]);
    }

    for (i, (file, _source)) in sources.iter().enumerate() {
        let file_module = crate::analyzer::type_collector::wj_file_to_module_path(src_base, file)
            .unwrap_or_default();
        global.set_current_file_module_path(file_module);
        global.prepare_program(&parsed_programs[i]);
    }

    let base = global.clone();
    let partials: Vec<FloatInference> = sources
        .par_iter()
        .enumerate()
        .map(|(i, (file, _source))| {
            let mut local = base.clone();
            let file_module =
                crate::analyzer::type_collector::wj_file_to_module_path(src_base, file)
                    .unwrap_or_default();
            local.set_current_file_module_path(file_module);
            local.collect_program_constraints(&parsed_programs[i]);
            local
        })
        .collect();

    for partial in partials {
        global.merge_parallel_state(partial);
    }
    global.finish_solve();
    global
}

fn run_parallel_int_inference(
    sources: &[(PathBuf, String)],
    parsed_programs: &[crate::parser::Program<'static>],
    src_base: &Path,
    global_float_signatures: &HashMap<
        String,
        (Vec<crate::parser::Type>, Option<crate::parser::Type>),
    >,
    global_struct_fields: &HashMap<String, HashMap<String, crate::parser::Type>>,
    struct_defining_module_paths: &HashMap<String, Vec<Vec<String>>>,
    module_re_exports: HashMap<String, HashMap<String, String>>,
) -> IntInference {
    let mut global = IntInference::new();
    global.set_global_function_signatures(global_float_signatures.clone());
    global.set_global_struct_field_types(global_struct_fields);
    global.set_struct_defining_module_paths(struct_defining_module_paths.clone());
    global.set_module_re_exports(module_re_exports);
    global.reset_imported_type_registry();

    for (i, (file, _source)) in sources.iter().enumerate() {
        let file_module = crate::analyzer::type_collector::wj_file_to_module_path(src_base, file)
            .unwrap_or_default();
        global.set_current_file_module_path(file_module);
        global.prepare_program(&parsed_programs[i]);
    }

    let base = global.clone();
    let partials: Vec<IntInference> = sources
        .par_iter()
        .enumerate()
        .map(|(i, (file, _source))| {
            let mut local = base.clone();
            let file_module =
                crate::analyzer::type_collector::wj_file_to_module_path(src_base, file)
                    .unwrap_or_default();
            local.set_current_file_module_path(file_module);
            local.collect_program_constraints(&parsed_programs[i]);
            local
        })
        .collect();

    for partial in partials {
        global.merge_parallel_state(partial);
    }
    global.finish_solve();
    global
}
