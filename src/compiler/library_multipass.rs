//! Multi-file library build with global multi-pass analysis.

use crate::analyzer::{Analyzer, SignatureRegistry};
use crate::codegen::rust::CodeGenerator;
use crate::metadata::{metadata_function_sig_from_analyzer, CrateMetadata};
use crate::parser::ast::core::Item;
use crate::parser::Parser;
// Legacy float/int inference replaced by UnifiedNumericInference
use crate::CompilationTarget;
use anyhow::Result;
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Instant;

fn profiling_enabled() -> bool {
    crate::debug_env::truthy("WJ_PROFILE")
}

fn profile_phase(report: &crate::mem_report::MemReport, phase: &str, start: Instant) {
    report.phase(phase);
    if profiling_enabled() {
        eprintln!("[wj-profile] {}: {}ms", phase, start.elapsed().as_millis());
    }
}

/// Writable access to the shared library registry. When another holder still reads it, `make_mut` copies it
/// whole; that copy is counted (`registry_entries_cloned`), so a per-file copy shows up in the cost ratchet.
fn global_mut<'a>(
    report: &crate::mem_report::MemReport,
    registry: &'a mut std::sync::Arc<SignatureRegistry>,
) -> &'a mut SignatureRegistry {
    if std::sync::Arc::strong_count(registry) > 1 || std::sync::Arc::weak_count(registry) > 0 {
        report.count(crate::mem_report::Work::RegistryEntriesCloned, registry.signatures.len() as u64);
    }
    std::sync::Arc::make_mut(registry)
}

/// Re-scan output for hand-written / cfg-gated `.rs` siblings and refresh `mod.rs`/`lib.rs`
/// even when incremental transpile is skipped (Bug #11 platform modules).
fn refresh_mod_declarations_for_handwritten_rs(
    output: &Path,
    src_base: &Path,
    target: CompilationTarget,
) -> Result<()> {
    if target == CompilationTarget::Rust {
        crate::build_utils::generate_mod_file_with_layout(output, Some((output, src_base)))?;
    }
    Ok(())
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

/// Find Windjammer stdlib directory (`std/`).
///
/// Shared by multipass + single-file API-type collection. Prefer env / crate
/// manifest over `current_exe()` so `cargo test` binaries still resolve `std/`.
pub(crate) fn find_stdlib_dir(override_dir: Option<&Path>) -> Option<PathBuf> {
    crate::compiler::library_copy_registry::find_stdlib_dir_for_api_types(override_dir)
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
    registry.find_unique_signature_ending_with(name)
}

/// TDD FIX: Build library with global multi-pass analysis
/// Solves cross-file transitive mutability inference
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_library_multipass(
    wj_files: &[PathBuf],
    base_path: &Path,
    output: &Path,
    target: CompilationTarget,
    _library: bool,
    enable_lint: bool,
    external_paths: &HashMap<String, PathBuf>,
    mut crate_metadata: CrateMetadata,
    config: &crate::compiler_config::CompilerConfig,
) -> Result<()> {
    let phase_start = Instant::now();
    let report = crate::mem_report::MemReport::new(config.mem_report);

    // The shared cache root: where `.wj.meta` entries of a manifest-less tree live.
    let cache_root = config.cache_root();

    // Step 0: Detect stdlib usage and prepend stdlib files FIRST
    let mut sources: Vec<(PathBuf, String)> = Vec::new();
    let (needed_stdlib_modules, mut std_stubs) = super::std_stubs::StdStubs::prepend_sources(
        wj_files,
        find_stdlib_dir(config.stdlib_path.as_deref()),
        &mut sources,
    );

    // Step 1: Read all user source files (keep sources alive for lifetime safety)
    for file in wj_files {
        let canon = std::fs::canonicalize(file).unwrap_or_else(|_| file.to_path_buf());
        let source = std::fs::read_to_string(&canon)?;
        sources.push((canon, source));
    }
    // Emit foundational modules before consumers so converged signatures (`keys_equal` →
    // `&Key`, `MemoryEngine::get` owned Key) land in `final_global_registry` before
    // `index/store` / `txn/manager` call sites codegen (alphabetical order put `index`
    // before `types`).
    sources.sort_by(|(a, _), (b, _)| {
        fn tier(path: &std::path::Path) -> u8 {
            let s = path.to_string_lossy();
            if s.contains("/types/") {
                0
            } else if s.contains("/engine/") {
                1
            } else if s.contains("/substrate/") {
                2
            } else {
                10
            }
        }
        tier(a)
            .cmp(&tier(b))
            .then_with(|| a.to_string_lossy().cmp(&b.to_string_lossy()))
    });

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
    profile_phase(&report, "Parse upfront", parse_start);

    let shader_filter_start = Instant::now();
    let (filtered_modules_by_dir, shader_count) =
        filter_shader_files(&mut sources, &mut parsers, &mut parsed_programs);
    if shader_count > 0 {
        eprintln!(
            "  Skipped {} shader file(s) from Rust pipeline (use WJSL target for GPU shaders)",
            shader_count
        );
    }
    profile_phase(&report, "Shader filter (parsed AST)", shader_filter_start);
    profile_phase(&report, "Step 0+1: Source reading + parse + shader filter",
        phase_start,
    );

    if sources.is_empty() {
        return Ok(());
    }

    let src_base: PathBuf = {
        let raw = crate::project_paths::entry_root(base_path).to_path_buf();
        std::fs::canonicalize(&raw).unwrap_or(raw)
    };

    let incremental_setup_start = Instant::now();
    // Dependency metadata roots (needed for both freshness check and analysis)
    let dep_roots =
        super::dependency_resolution::find_dependency_metadata_roots(&src_base, external_paths);
    let dep_epoch_snapshot = super::incremental::dep_metadata_epoch(&dep_roots);

    super::cache_management::invalidate_meta_cache_on_compiler_change(&src_base, &cache_root, output);

    let dependency_graph =
        super::incremental::DependencyGraph::build(&sources, &parsed_programs, &src_base);
    profile_phase(&report, "Incremental setup (deps + dep graph)",
        incremental_setup_start,
    );

    // INCREMENTAL: Whole-crate fast path — if no .wj file has changed and dep
    // metadata is also unchanged, skip the entire transpilation pipeline.
    if super::cache_management::all_sources_fresh(&sources, &src_base, output, &cache_root, &dep_roots) {
        let stale = super::cache_management::find_stale_codegen_outputs_with_dep_epoch(
            &sources, &src_base, output, &cache_root, &dep_roots, Some(dep_epoch_snapshot),
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
        refresh_mod_declarations_for_handwritten_rs(output, &src_base, target)?;
        return Ok(());
    }

    if !super::cache_management::is_compiler_stamp_fresh(output) {
        eprintln!("⟳ Compiler changed — re-transpiling all sources");
    }

    let mut reanalysis_set = super::incremental::compute_reanalysis_set(
        &sources,
        &src_base,
        output,
        &cache_root,
        dep_epoch_snapshot,
        &dependency_graph,
    );
    if reanalysis_set.is_empty() {
        eprintln!(
            "⚡ Incremental: 0/{} files changed — nothing to do ({:.1}s)",
            sources.len(),
            phase_start.elapsed().as_secs_f64()
        );
        return Ok(());
    }
    std_stubs.mark_reanalysis(&sources, &mut reanalysis_set);
    if reanalysis_set.len() < sources.len() {
        eprintln!(
            "⚡ Incremental analysis: {}/{} files need re-analysis",
            reanalysis_set.len(),
            sources.len()
        );
    }

    let copy_registry_start = Instant::now();
    let (
        mut global_copy_structs,
        local_struct_names,
        explicit_non_copy_structs,
        global_explicit_copy_structs,
    ) = super::library_copy_registry::collect_global_copy_structs_for_library_with_programs(
        &sources,
        Some(&parsed_programs),
    );
    let global_non_copy_enums =
        super::library_copy_registry::collect_non_copy_enums_for_library_with_programs(
            &sources,
            Some(&parsed_programs),
        );

    // Merge explicit non-Copy structs with non-Copy enums for codegen registry
    let mut global_non_copy_types = global_non_copy_enums.clone();
    global_non_copy_types.extend(explicit_non_copy_structs.iter().cloned());
    let crate_partial_eq_types = super::library_codegen_inputs::crate_partial_eq_types(
        &parsed_programs,
        (&global_copy_structs, &global_explicit_copy_structs, &global_non_copy_types),
        target,
    );
    let mut dep_registry = SignatureRegistry::new();
    {
        let mut dep_copy_structs = Vec::new();
        let mut dep_struct_fields: HashMap<String, Vec<Vec<String>>> = HashMap::new();
        for root in &dep_roots {
            crate::metadata::merge_wj_meta_signatures_from_dir_inner_pub(
                root,
                &cache_root,
                &mut dep_registry,
                &mut dep_copy_structs,
                &mut dep_struct_fields,
            );
        }
        // Qualified `dep_crate::fn` call sites need crate-prefix aliases.
        crate::metadata::merge_external_crate_metadata_with_aliases(
            external_paths,
            &mut dep_registry,
            None,
        );
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
    profile_phase(&report, "Copy registry collection", copy_registry_start);

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
    // Merge same-crate `.wj.meta` AFTER dependency filtering. Do not drop again —
    // a second `drop_dependency_signatures_for_local_types` would delete bare
    // `Type::method` keys from defining-module metas (cross-file &str).
    crate::metadata::merge_wj_meta_signatures_from_dir(&src_base, &cache_root, &mut global_registry);
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
    profile_phase(&report, "Declaration stub registry (parallel)", stub_registry_start);

    // insert Box on one owned field per recursive SCC so rustc E0072
    // cannot fire on mutually recursive embeds (complete-chain / reducer graphs).
    crate::codegen::rust::recursive_struct_layout::apply_recursive_struct_boxing(
        &mut global_struct_fields,
    );

    // Step 3: Global multi-pass iteration until convergence
    // Wrap read-only data in Arc for O(1) sharing across all files (avoids O(n) deep clones).
    let global_struct_fields = std::sync::Arc::new(global_struct_fields);
    let struct_defining_module_paths = std::sync::Arc::new(struct_defining_module_paths);
    let global_copy_structs = std::sync::Arc::new(global_copy_structs);

    std_stubs.capture_contracts(&global_registry);
    const MAX_GLOBAL_PASSES: usize = 10;
    let mut pass_number = 1;
    let step3_start = Instant::now();
    let mut global_registry = std::sync::Arc::new(global_registry);

    loop {
        let round_start = Instant::now();
        let global_arc = std::sync::Arc::clone(&global_registry);
        let mut pass_changed = false;

        if sources.len() > 16 {
            eprintln!(
                "⟳ Ownership convergence pass {} ({} files)...",
                pass_number,
                sources.len()
            );
        }

        // Re-analyze files in bounded batches — layered registries avoid per-file full clones.
        // global registry once per worker and OOMs (exit 137) on large crates.
        let mut file_registries: Vec<SignatureRegistry> =
            vec![SignatureRegistry::empty(); sources.len()];
        for batch_start in (0..sources.len()).step_by(STEP4B_REGISTRY_BATCH_SIZE) {
            let batch_end = (batch_start + STEP4B_REGISTRY_BATCH_SIZE).min(sources.len());
            let batch: Vec<usize> = (batch_start..batch_end).collect();
            let batch_results: Vec<(usize, SignatureRegistry)> = batch
                .into_par_iter()
                .map(|i| -> Result<(usize, SignatureRegistry), String> {
                    if !reanalysis_set.contains(&i) {
                        return Ok((i, SignatureRegistry::empty()));
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
                        .analyze_program_with_global_arc(program, &global_arc)
                        .map(|(_, file_registry, _)| (i, file_registry))
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
            for (i, reg) in batch_results {
                file_registries[i] = reg;
            }
        }

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
                let sig = &*std_stubs.effective_sig(file, file_stem, name, sig);
                // Skip passthrough entries inherited from the frozen global snapshot.
                // analyze_program_with_global_arc clones the global registry, so
                // file_registry contains ALL entries (global + file-specific).
                // If an entry is identical to the frozen snapshot, it's a passthrough —
                // merging it would overwrite convergence results from earlier files.
                let is_passthrough = global_arc.get_signature(name).map_or(false, |arc_sig| {
                    !SignatureRegistry::ownership_changed(arc_sig, sig)
                });
                if is_passthrough {
                    continue;
                }
                let should_insert = match global_registry.get_signature(name) {
                    None => true,
                    Some(old_sig) => {
                        SignatureRegistry::ownership_changed(old_sig, sig)
                            && !crate::codegen::rust::signature_promotion::defining_mixed_owned_emission_beats(
                                old_sig, sig,
                            )
                    }
                };
                if should_insert {
                    pass_changed = true;
                    let new_registry = global_mut(&report, &mut global_registry);
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
                            new_registry.add_function(
                                format!("crate::{}::{}", module_path, name),
                                sig.clone(),
                            );
                        }
                    }
                }
            }
        }

        // `std/strings.wj` merge aliases `len` → `strings::len` as owned stubs.
        // Restore the runtime borrow contract after every convergence round.
        global_mut(&report, &mut global_registry)
            .restore_runtime_borrowed_strings_signatures();

        profile_phase(&report, &format!("Step 3 round {}", pass_number), round_start);

        if !pass_changed || pass_number >= MAX_GLOBAL_PASSES {
            break;
        }

        pass_number += 1;
    }
    profile_phase(&report, "Step 3 total", step3_start);

    let reexports_start = Instant::now();
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

    // Step 4A: Unified numeric inference (parallel per-file constraint collection + single solve)
    let step4a_start = Instant::now();
    let global_numeric_inference = run_parallel_numeric_inference(
        &sources,
        &parsed_programs,
        &src_base,
        external_paths,
        &global_float_signatures,
        &global_struct_fields,
        &struct_defining_module_paths,
        module_re_exports,
        &report,
    );
    super::bail_on_inference_errors(&global_numeric_inference.errors, "Numeric", None)?;
    profile_phase(&report, "Step 4A: Unified numeric inference", step4a_start);
    profile_phase(&report, "Re-exports + Step 4A total", reexports_start);

    let global_numeric_inference = std::sync::Arc::new(global_numeric_inference);

    let dep_resolution_start = Instant::now();
    let type_defining_modules =
        super::dependency_resolution::build_type_defining_modules_for_library_with_programs(
            &sources,
            &src_base,
            Some(&parsed_programs),
        )?;
    let extern_submodule_qualifiers =
        super::dependency_resolution::build_extern_submodule_qualifier_map_with_programs(
            &sources,
            &src_base,
            Some(&parsed_programs),
        )?;
    profile_phase(&report, "Dependency resolution (type modules + extern qualifiers)",
        dep_resolution_start,
    );

    // Step 4B-pre: Build GLOBAL analyzed_trait_methods across ALL files.
    // Each file's Analyzer is fresh, so cross-file trait info (e.g. RenderPort defined
    // in render_port.wj but implemented in voxel_gpu_renderer.wj) would be missing
    // if we only used per-file analysis. This step mirrors main.rs's finalize_trait_inference.
    //
    // Do NOT clone every item into one mega-Program (~669 files × thousands of items):
    // that peak allocation OOMs tip engine rebuilds after ownership convergence (jetsam /
    // SIGKILL with no EXIT). Register traits globally, then infer impl signatures
    // per-file so analyzed_trait_methods still accumulates max-permissive receivers.
    let step4b_pre_start = Instant::now();
    let global_analyzed_trait_methods = {
        let mut shared_analyzer = Analyzer::new_with_copy_structs((*global_copy_structs).clone());

        for program in &parsed_programs {
            shared_analyzer
                .register_traits_from_program(program)
                .unwrap_or_else(|e| eprintln!("Trait registration warning: {}", e));
        }

        for program in &parsed_programs {
            if let Err(e) =
                shared_analyzer.infer_trait_signatures_from_impls(program, global_registry.as_ref())
            {
                eprintln!("Cross-file trait inference warning: {}", e);
            }
        }
        shared_analyzer.analyzed_trait_methods.clone()
    };
    profile_phase(&report, "Step 4B-pre: Trait inference (per-file)", step4b_pre_start);

    // Step 4B: Final analysis + code generation (using shared global_float_inference)
    //
    // TWO-PHASE APPROACH: First analyze ALL files to build a consistent final registry,
    // then generate code using that registry. This ensures cross-module call sites see
    // the same Phase 2 optimized signatures as the function definitions.
    //
    // INCREMENTAL (Phase 2): Compute dirty files to skip codegen for unchanged sources.
    // Analysis still runs for ALL files (needed for metadata.json convergence),
    // but the expensive codegen + write is skipped for files whose source hasn't changed.
    let (dirty_indices, skipped_count) = super::cache_management::compute_codegen_files(
        &sources, &src_base, output, &cache_root, dep_epoch_snapshot, &reanalysis_set,
    );
    let dirty_set: HashSet<usize> = dirty_indices.into_iter().collect();
    if skipped_count > 0 {
        eprintln!(
            "⚡ Incremental: {}/{} files unchanged, regenerating {} dirty files",
            skipped_count,
            sources.len(),
            dirty_set.len()
        );
    }

    // Step 4B: Two-phase — analyze ALL files first (merge registry), then codegen so
    // cross-module call sites see converged signatures from files compiled later in order
    // (e.g. game_systems.wj calling QuestManager before quest/manager.wj was analyzed).
    let step4b_start = Instant::now();
    let global_enum_variant_types = std::sync::Arc::new(
        super::library_copy_registry::collect_global_enum_variant_types_for_library(
            &parsed_programs,
        ),
    );
    let mut final_global_registry = global_registry;
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

    let global_analyzed_trait_methods_arc = std::sync::Arc::new(global_analyzed_trait_methods);

    let user_file_indices: Vec<usize> = sources
        .iter()
        .enumerate()
        .filter_map(|(i, (f, _))| f.strip_prefix(&src_base).ok().map(|_| i))
        .collect();
    let user_file_count = user_file_indices.len();
    let mut step4b_analyses: Vec<Option<Step4bFileAnalysis>> =
        (0..sources.len()).map(|_| None).collect();

    // Phase 4B-a: analyze files and merge into the global registry.
    // OPTIMIZATION: Only analyze dirty files + files in the reanalysis set.
    // Unchanged files' registry contributions are already in final_global_registry from Step 3.
    let step4b_analyze_start = Instant::now();
    let mut step_done = 0usize;
    let mut step4b_analyzed_count = 0usize;
    let library_consts = std::sync::Arc::new(Analyzer::library_const_types(&parsed_programs));
    let mut step4b_skipped_count = 0usize;
    {
        let user_in_reanalysis = user_file_indices
            .iter()
            .filter(|i| reanalysis_set.contains(i))
            .count();
        let user_in_dirty = user_file_indices
            .iter()
            .filter(|i| dirty_set.contains(i))
            .count();
        eprintln!(
            "⚡ Step 4B-a prep: user_files={} reanalysis_set_total={} dirty_set_total={} user_in_reanalysis={} user_in_dirty={}",
            user_file_count, reanalysis_set.len(), dirty_set.len(), user_in_reanalysis, user_in_dirty
        );
        report.phase("Step 4B-a prep");
    }
    for i in &user_file_indices {
        let file = &sources[*i].0;
        step_done += 1;

        // Skip full analysis for files that are NOT dirty and NOT in the reanalysis set.
        // Their registry is already merged from Step 3, and codegen will be skipped too.
        // However, their .wj.meta converged signatures must be loaded into
        // local_converged_sigs so the promotion step can install correct ownership
        // (e.g. Borrowed for &str params) into final_global_registry.
        if !dirty_set.contains(i) && !reanalysis_set.contains(i) {
            let file_stem = file
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();
            let module_path =
                crate::analyzer::type_collector::wj_file_to_module_path(&src_base, file)
                    .unwrap_or_default()
                    .join("::");
            let output_file =
                crate::project_paths::resolve_wj_output_path_library(&src_base, file, output)
                    .unwrap_or_else(|_| output.join(file_stem.clone()).with_extension("rs"));

            // Load converged signatures from this file's .wj.meta and install them
            // directly into final_global_registry. These signatures are the converged
            // output from the previous build, so they're authoritative and don't need
            // the prefer_converged_over_stub promotion check.
            let meta_path = crate::metadata::meta_cache_path(file, &cache_root);
            if meta_path.exists() {
                let mut meta_registry = SignatureRegistry::empty();
                let mut meta_copy = Vec::new();
                let mut meta_fields: HashMap<String, Vec<Vec<String>>> = HashMap::new();
                crate::metadata::merge_wj_meta_signatures_from_file_pub(
                    &meta_path,
                    &cache_root,
                    &mut meta_registry,
                    &mut meta_copy,
                    &mut meta_fields,
                );
                let reg = global_mut(&report, &mut final_global_registry);
                for (name, sig) in &meta_registry.signatures {
                    if sig.param_ownership.is_empty() {
                        continue;
                    }
                    let is_local =
                        crate::metadata::signature_targets_local_struct(name, &local_struct_names)
                            || (!name.contains("::")
                                && crate_metadata.functions.contains_key(name));
                    if !is_local {
                        continue;
                    }
                    // Install under all key variants: short, file-stem-qualified,
                    // and module-path-qualified.
                    let mut keys = vec![name.clone()];
                    if !file_stem.is_empty() && !name.starts_with(&format!("{}::", file_stem)) {
                        keys.push(format!("{}::{}", file_stem, name));
                    }
                    if !module_path.is_empty() && !name.starts_with(&format!("{}::", module_path)) {
                        keys.push(format!("{}::{}", module_path, name));
                        keys.push(format!("crate::{}::{}", module_path, name));
                    }
                    for key in keys {
                        reg.signatures.insert(key.clone(), sig.clone());
                    }
                    local_converged_sigs.insert(name.clone(), sig.clone());
                }
            }

            step4b_analyses[*i] = Some(Step4bFileAnalysis {
                registry: SignatureRegistry::empty(),
                analyzed_functions: Vec::new(),
                own_trait_methods: None,
                copy_structs: Vec::new(),
                output_file,
                file_stem,
                module_path,
                lint_errors: Vec::new(),
            });
            step4b_skipped_count += 1;
            continue;
        }

        step4b_analyzed_count += 1;
        if user_file_count > 16 && step4b_analyzed_count <= 20 {
            eprintln!(
                "⟳ Analyze: file {}/{} ({})...",
                step_done,
                user_file_count,
                file.file_name().and_then(|n| n.to_str()).unwrap_or("?")
            );
        }

        let file_start = Instant::now();
        let mut analysis = analyze_file_for_step4b(
            *i,
            &sources,
            &parsed_programs,
            &stripped_programs,
            &src_base,
            output,
            &final_global_registry,
            &global_copy_structs,
            &global_struct_fields,
            &struct_defining_module_paths,
            &library_consts,
            enable_lint,
        )
        .map_err(|e| anyhow::anyhow!(e))?;
        if user_file_count > 16 {
            let secs = file_start.elapsed().as_secs_f64();
            if secs >= 5.0 {
                eprintln!(
                    "  ⏱ {} took {:.1}s",
                    file.file_name().and_then(|n| n.to_str()).unwrap_or("?"),
                    secs
                );
            }
        }

        deferred_lint_errors.extend(analysis.lint_errors.clone());

        let full_registry = merge_file_analysis_into_global(
            &report,
            &mut analysis,
            &mut final_global_registry,
            &global_analyzed_trait_methods_arc,
            &global_copy_structs,
            &global_struct_fields,
            &struct_defining_module_paths,
        );
        report.progress("Step 4B-a analyzed file", step_done, user_file_count);

        for (name, sig) in &full_registry.signatures {
            if !sig.param_ownership.is_empty()
                && (crate::metadata::signature_targets_local_struct(name, &local_struct_names)
                    || (!name.contains("::") && crate_metadata.functions.contains_key(name)))
            {
                local_converged_sigs.insert(name.clone(), sig.clone());
                if !name.contains("::") && !analysis.file_stem.is_empty() {
                    local_converged_sigs
                        .insert(format!("{}::{}", analysis.file_stem, name), sig.clone());
                }
            }
        }

        step4b_analyses[*i] = Some(analysis);
    }
    if step4b_skipped_count > 0 {
        eprintln!(
            "⚡ Step 4B-a: analyzed {}, skipped {} unchanged files",
            step4b_analyzed_count, step4b_skipped_count
        );
    }
    profile_phase(&report, "Step 4B-a: Analysis (sequential)", step4b_analyze_start);

    // Promote converged local signatures over stale engine/dependency stubs on canonical
    // `Type::method` keys. Step 4B merge can leave engine metadata on the short key while
    // body analysis converged under a module-qualified alias (e.g. quest/manager.wj).
    {
        use crate::codegen::rust::call_signature_resolution::{
            body_borrow_must_not_replace_owned_copy_formal,
            body_borrow_must_not_replace_owned_formal_stub, prefer_converged_over_stub,
        };
        let reg = global_mut(&report, &mut final_global_registry);
        for (name, sig) in &local_converged_sigs {
            let mut keys = vec![name.clone()];
            let parts: Vec<&str> = name.split("::").collect();
            if parts.len() >= 2 {
                let method = parts[parts.len() - 1];
                let ty = parts[parts.len() - 2];
                if ty.chars().next().is_some_and(|c| c.is_ascii_uppercase()) {
                    keys.push(format!("{ty}::{method}"));
                }
            }
            for key in keys {
                match reg.get_signature(&key) {
                    None => {
                        // Defining-module meta may only exist under a module-qualified
                        // alias; always install the canonical `Type::method` key.
                        reg.signatures.insert(key, sig.clone());
                    }
                    Some(existing)
                        if prefer_converged_over_stub(existing, sig)
                            && !body_borrow_must_not_replace_owned_formal_stub(existing, sig)
                            && !body_borrow_must_not_replace_owned_copy_formal(
                                existing,
                                sig,
                                &global_copy_structs,
                            ) =>
                    {
                        reg.signatures.insert(key, sig.clone());
                    }
                    Some(_) => {}
                }
            }
        }
    }

    // Diagnostic: dump registry state for key methods before codegen.

    // Bare-pass demotion / field-forward restore must run before IR lowering so
    // call-site constraints see the final ownership (e.g. owned HttpReply, not &mut).
    {
        use crate::codegen::rust::signature_promotion::wrap_converged_borrow_param_types;
        let reg = global_mut(&report, &mut final_global_registry);
        for sig in reg.signatures.values_mut() {
            wrap_converged_borrow_param_types(sig);
        }
    }
    {
        use crate::codegen::rust::signature_promotion::wrap_converged_borrow_param_types;
        let program_refs: Vec<&crate::parser::Program> = parsed_programs.iter().collect();
        let reg = global_mut(&report, &mut final_global_registry);
        crate::compiler::multipass_bare_pass_demotion::promote_callees_from_bare_pass_callers(
            reg,
            &program_refs,
            &global_copy_structs,
        );
        crate::compiler::multipass_bare_pass_demotion::restore_pub_owned_non_copy_api_formals(
            reg,
            &program_refs,
        );
        crate::compiler::multipass_bare_pass_demotion::restore_owned_formals_forwarded_to_owned_callees(
            reg,
            &program_refs,
        );
        for sig in reg.signatures.values_mut() {
            wrap_converged_borrow_param_types(sig);
        }
    }

    // Path-dep generated Rust ABI is last writer after analysis stubs converge
    // (`wj_querystring::get` key: &str must beat Owned/Owned importer stubs).
    {
        let reg = global_mut(&report, &mut final_global_registry);
        crate::metadata::merge_external_crate_metadata_with_aliases(external_paths, reg, None);
    }

    // Step 4B-a-IR: Lower converged analyses to IR using the merged global registry.
    // Cross-file call constraints resolve against final_global_registry.
    let step4b_ir_start = Instant::now();
    let formal_scope = super::value_formal_fixed_point::FormalScope::new(&parsed_programs, &global_copy_structs);
    let mut ir_functions_by_index: Vec<Option<Vec<crate::ir::node::IrFunction>>> =
        vec![None; sources.len()];
    {
        use crate::ir::pipeline::{DiagnosticSeverity, IrPipeline};
        let mut pipeline = IrPipeline::new();
        let mut library_fields = library_field_types(&global_struct_fields, &parsed_programs);
        // Rounds until callers and callees agree on value-formal modes (value_formal_fixed_point).
        for round in 1..=super::value_formal_fixed_point::MAX_ROUNDS {
            let mut ir_errors = Vec::new();
            for (i, analysis_opt) in step4b_analyses.iter().enumerate() {
                let Some(analysis) = analysis_opt.as_ref().filter(|a| !a.analyzed_functions.is_empty()) else { continue };
                // Per-file import aliases (`qs_get` → `wj_querystring::get`) overwrite analyzer Owned stubs before IR expected types are set.
                let mut ir_reg = SignatureRegistry::layered(std::sync::Arc::clone(&final_global_registry));
                crate::codegen::rust::program_generation::install_program_import_alias_signatures(&mut ir_reg, &parsed_programs[i]);
                let module = with_file_field_types(&mut library_fields, &parsed_programs[i], |fields| {
                    pipeline.lower_to_ir(&analysis.analyzed_functions, &ir_reg, Some(fields))
                });
                // Release the read-through before writing the global registry, so the write is in place
                // instead of a full copy of the library registry per file and round.
                drop(ir_reg);
                for diag in module.diagnostics.iter().filter(|d| matches!(d.severity, DiagnosticSeverity::Error)) {
                    ir_errors.push(format!("[IR ERROR] {}: {}", sources[i].0.display(), diag.message));
                }
                crate::ir::signature_bridge::sync_ir_ownership_to_registry(&module.functions, &analysis.analyzed_functions, global_mut(&report, &mut final_global_registry));
                ir_functions_by_index[i] = Some(module.functions);
                report.progress(&format!("Step 4B-a-IR round {round} lower file"), i + 1, step4b_analyses.len());
            }
            let published = publish_value_formals(&formal_scope, global_mut(&report, &mut final_global_registry), &sources, &ir_functions_by_index, &step4b_analyses);
            if !published || round == super::value_formal_fixed_point::MAX_ROUNDS {
                ir_errors.iter().for_each(|e| eprintln!("{e}"));
                break;
            }
        }
    }
    // IR sync can re-promote field-forward helpers to MutBorrowed from analyzer
    // body classification — restore owned formals again for call-site codegen.
    {
        let program_refs: Vec<&crate::parser::Program> = parsed_programs.iter().collect();
        let reg = global_mut(&report, &mut final_global_registry);
        crate::compiler::multipass_bare_pass_demotion::restore_pub_owned_non_copy_api_formals(
            reg,
            &program_refs,
        );
        crate::compiler::multipass_bare_pass_demotion::restore_owned_formals_forwarded_to_owned_callees(
            reg,
            &program_refs,
        );
        // 4B-a / `.wj.meta` alias inserts re-apply `strings.wj` owned stubs
        // (`len` → `strings::len`). Runtime AsRef/&str must win before codegen.
        reg.restore_runtime_borrowed_strings_signatures();
        // The restores above predict formals; the spelled value formals are the IR's: it has the last word.
        publish_value_formals(&formal_scope, reg, &sources, &ir_functions_by_index, &step4b_analyses);
        publish_into_file_registries(&formal_scope, &sources, &ir_functions_by_index, &mut step4b_analyses);
    }
    profile_phase(&report, "Step 4B-a-IR: IR lowering", step4b_ir_start);

    // Detect if any file in the project uses serde (via `use std::json`).
    // When true, all structs in the project auto-derive Serialize/Deserialize.
    let project_has_serde = parsed_programs.iter().any(|program| {
        use crate::parser::ast::core::Item;
        for item in &program.items {
            if let Item::Use { path, .. } = item {
                let path_str = path.join("::");
                if path_str.contains("json") && (path_str.starts_with("std::") || path_str == "std")
                {
                    return true;
                }
            }
        }
        false
    });

    // Phase 4B-b: codegen using the fully merged global registry.
    // PARALLEL: Each file's codegen is independent — use rayon for throughput.
    let step4b_codegen_start = Instant::now();

    // First pass: identify which files actually need codegen (cheap filter).
    let mut codegen_indices: Vec<usize> = (0..sources.len())
        .filter(|i| {
            let is_user_file = sources[*i].0.strip_prefix(&src_base).is_ok();
            if !is_user_file {
                return false;
            }
            let analysis = match step4b_analyses[*i].as_ref() {
                Some(a) => a,
                None => return false,
            };
            if !dirty_set.contains(i)
                && !reanalysis_set.contains(i)
                && analysis.output_file.exists()
            {
                return false;
            }
            if !dirty_set.contains(i) && analysis.output_file.exists() {
                let source = std::fs::read_to_string(&sources[*i].0)
                    .unwrap_or_else(|_| sources[*i].1.clone());
                if super::cache_management::is_library_codegen_cache_valid_with_dep_epoch(
                    &source,
                    &sources[*i].0,
                    &analysis.output_file,
                    &src_base,
                    output,
                    &cache_root,
                    dep_epoch_snapshot,
                ) {
                    return false;
                }
            }
            true
        })
        .collect();
    codegen_indices = dependency_graph.sort_indices_for_codegen(&codegen_indices);

    if crate::debug_env::enabled("WJ_DEBUG_CODEGEN_ORDER") {
        let order: Vec<String> = codegen_indices
            .iter()
            .filter_map(|i| {
                sources[*i]
                    .0
                    .strip_prefix(&src_base)
                    .ok()
                    .map(|p| p.display().to_string())
            })
            .collect();
        eprintln!("WJ_DEBUG_CODEGEN_ORDER: {order:?}");
    }

    let codegen_written_count = codegen_indices.len();
    if user_file_count > 16 {
        eprintln!(
            "⚡ Step 4B-b: {} files need codegen (skipping {})",
            codegen_written_count,
            user_file_count - codegen_written_count
        );
    }

    // Wrap shared data in Arcs for parallel access.
    let type_defining_modules_arc = std::sync::Arc::new(type_defining_modules);
    let extern_submodule_qualifiers_arc = std::sync::Arc::new(extern_submodule_qualifiers);
    let global_explicit_copy_structs_arc = std::sync::Arc::new(global_explicit_copy_structs);
    let global_non_copy_types_arc = std::sync::Arc::new(global_non_copy_types);
    let ir_functions_by_index_arc = std::sync::Arc::new(ir_functions_by_index);

    // Batched parallel codegen: process files in batches to bound peak memory.
    // Each worker deep-clones shared registries, so unbounded parallelism on
    // large codebases (2000+ files) can exceed available RAM.
    const CODEGEN_BATCH_SIZE: usize = 64;

    let total_codegen = codegen_indices.len();
    let mut _codegen_done = 0usize;
    // Cross-file: `&mut` formal slots discovered during codegen must reach later call sites
    // (`Ability::activate` → `self.dash.activate(&mut self.player)`).
    let mut accumulated_emitted_mut_arg_indices: std::collections::HashMap<
        String,
        std::collections::HashSet<usize>,
    > = std::collections::HashMap::new();

    let mut converged_qualified_keys = super::converged_module_signatures::register_all_files(global_mut(&report, &mut final_global_registry), &sources, &src_base, |i| stripped_programs[i].as_ref().unwrap_or(&parsed_programs[i]), |i| step4b_analyses[i].as_ref().map(|a| &a.registry));

    for batch_start in (0..total_codegen).step_by(CODEGEN_BATCH_SIZE) {
        let batch_end = (batch_start + CODEGEN_BATCH_SIZE).min(total_codegen);
        let batch = &codegen_indices[batch_start..batch_end];

        if total_codegen > 50 {
            eprintln!(
                "⚙️  Codegen batch {}-{}/{}",
                batch_start + 1,
                batch_end,
                total_codegen
            );
        }

        // Sequential within each batch so codegen-time signature refresh
        // (refresh_method_registry_from_emitted_formals) propagates to later files
        // in the same library build (MemoryEngine::range_scan → LsmEngine call sites).
        for i in batch {
            let file = &sources[*i].0;
            let file_progress =
                total_codegen > 50 || crate::debug_env::enabled("WJ_DEBUG_CODEGEN_FILE");
            let file_start = Instant::now();
            if file_progress {
                let rel = file
                    .strip_prefix(&src_base)
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|_| file.display().to_string());
                eprintln!(
                    "⚙️  Codegen file {}/{}: {} (registry≈{} sigs)",
                    _codegen_done + 1,
                    total_codegen,
                    rel,
                    final_global_registry.signatures.len()
                );
            }
            let analysis = step4b_analyses[*i]
                .as_ref()
                .expect("analysis must exist after phase 4B-a");
            let merged_trait_methods =
                analysis.merged_trait_methods(&global_analyzed_trait_methods_arc);
            let program: &crate::parser::Program<'static> = stripped_programs[*i]
                .as_ref()
                .unwrap_or(&parsed_programs[*i]);

            // Layered local registry: O(|local|) overlay + Arc global fallback.
            // Never copy the full global map (~10k–20k+ keys) into each file.
            let mut full_registry = crate::analyzer::SignatureRegistry::layered(
                std::sync::Arc::clone(&final_global_registry),
            );
            // C3: this file's own definitions take its own analysis; "beats" rules compare versions of
            // one function and must not let a same-name function of another file win.
            let own_defs = own_definition_keys(program);
            for (name, sig) in analysis.registry.signatures.iter() {
                if converged_qualified_keys.contains(name) { continue; }
                if let Some(existing) = full_registry.get_signature(name).filter(|_| !own_defs.contains(name)) {
                    if crate::codegen::rust::signature_promotion::defining_mixed_owned_emission_beats(
                        existing, sig,
                    ) || crate::codegen::rust::signature_promotion::codegen_refreshed_beats_analysis_only(
                        existing, sig,
                    ) || crate::codegen::rust::signature_promotion::shared_ref_emission_beats(
                        existing, sig,
                    ) || crate::codegen::rust::signature_promotion::defining_text_demotion_beats_owned_string_snapshot(
                        existing, sig,
                    ) {
                        continue;
                    }
                }
                full_registry.signatures.insert(name.clone(), sig.clone());
            }
            // Prefer post-bare-pass / field-forward restored ownership over stale
            // per-file analysis MutBorrowed so preregistered formals and call sites agree.
            crate::codegen::rust::signature_promotion::promote_overlapping_global_signatures_into_local(
                &mut full_registry,
                final_global_registry.as_ref(),
            );
            keep_own_definitions(&mut full_registry, &analysis.registry, &own_defs);
            crate::metadata::merge_external_crate_metadata_with_aliases(
                external_paths,
                &mut full_registry,
                None,
            );
            crate::codegen::rust::program_generation::install_program_import_alias_signatures(
                &mut full_registry,
                program,
            );
            {
                let mut tmp_analyzer = Analyzer::for_library_pass(
                    global_copy_structs.clone(),
                    global_struct_fields.clone(),
                    struct_defining_module_paths.clone(),
                );
                tmp_analyzer.analyzed_trait_methods = merged_trait_methods.clone();
                tmp_analyzer.register_trait_methods_in_registry(
                    &merged_trait_methods,
                    &mut full_registry,
                );
                write_back_counted(&report, &mut full_registry);
            }
            write_back_global_if_needed(&report, &mut final_global_registry);

            // Crate-root main.wj is the binary entry ([[bin]]), not a library module.
            // Using new_for_module skipped `fn main` and injected `use super::*`.
            let is_crate_root_main =
                crate::module_system::module_graph::is_bin_entry_file(file, &src_base);

            let mut codegen = if is_crate_root_main {
                CodeGenerator::new(full_registry, target)
            } else {
                CodeGenerator::new_for_module(full_registry, target)
            };
            codegen.set_config(config.clone());
            codegen.set_global_signature_registry(std::sync::Arc::clone(&final_global_registry));
            codegen.merge_function_emitted_mut_arg_indices(&accumulated_emitted_mut_arg_indices);
            codegen.set_copy_types_registry((*global_copy_structs).clone());
            codegen.set_crate_partial_eq_types(crate_partial_eq_types.clone());
            codegen.set_explicit_copy_types_registry((*global_explicit_copy_structs_arc).clone());
            codegen.set_non_copy_types_registry((*global_non_copy_types_arc).clone());
            codegen.set_global_struct_field_types((*global_struct_fields).clone());
            codegen.set_library_const_types(&library_consts);
            codegen.set_global_enum_variant_types((*global_enum_variant_types).clone());
            super::library_codegen_inputs::apply_stdlib_api_types(&mut codegen, &needed_stdlib_modules, config);
            codegen.set_output_position(&analysis.output_file, output, src_base.as_path());
            codegen.set_source_file(file);
            codegen.set_library_source_root(src_base.clone());
            codegen.set_type_defining_modules((*type_defining_modules_arc).clone());
            codegen.set_extern_submodule_qualifiers((*extern_submodule_qualifiers_arc).clone());
            codegen.set_analyzed_trait_methods(merged_trait_methods);
            codegen.set_shared_numeric_inference(std::sync::Arc::clone(&global_numeric_inference));
            codegen.set_serde_available(project_has_serde);
            if let Some(ir_functions) = ir_functions_by_index_arc[*i].clone() {
                codegen.set_ir_functions(ir_functions);
            }

            super::apply_inferred_bounds_to_codegen(&mut codegen, program);
            super::write_generated_rust_and_meta(
                &mut codegen,
                program,
                &analysis.analyzed_functions,
                &analysis.output_file,
                file,
                analysis.copy_structs.clone(),
                target,
                &dep_roots,
                Some(dep_epoch_snapshot),
            )?;

            // Bin + lib package: `crate::` in main must resolve via the lib crate name.
            // Skip when there is no configured package name (temp/integration trees keep
            // `use crate::` for single-crate cargo-check harnesses).
            if is_crate_root_main {
                let lib_name = bin_entry_lib_name(&src_base, base_path, config);
                if let Some(lib_name) = lib_name {
                    let lib_name = lib_name.replace('-', "_");
                    rewrite_bin_crate_imports_to_lib(&analysis.output_file, &lib_name)?;
                }
            }

            // Take what the global registry needs from the generator and drop it first: it holds the global
            // registry (directly and through its layered registry), and writing the global while it is alive
            // forces a full copy of the library registry for every file. The merges read only the local layer.
            let mut codegen_registry = std::mem::replace(
                &mut codegen.signature_registry,
                crate::analyzer::SignatureRegistry::empty(),
            );
            codegen_registry.detach_shared_layers();
            let codegen_mut_arg_indices = std::mem::take(&mut codegen.function_emitted_mut_arg_indices);
            let codegen_self_receiver_upgrades = std::mem::take(&mut codegen.self_receiver_upgrades);
            drop(codegen);

            let reg = global_mut(&report, &mut final_global_registry);
            reg.merge(&codegen_registry);
            super::converged_module_signatures::register_converged_module_qualified_signatures(reg, &crate::analyzer::type_collector::wj_file_to_module_path(&src_base, file).unwrap_or_default(), program, &codegen_registry, &mut converged_qualified_keys);
            for (name, indices) in &codegen_mut_arg_indices {
                accumulated_emitted_mut_arg_indices
                    .entry(name.clone())
                    .or_default()
                    .extend(indices.iter().copied());
            }
            for (name, mode) in &codegen_self_receiver_upgrades {
                if let Some(sig) = reg.signatures.get_mut(name) {
                    sig.upgrade_self_receiver(*mode);
                }
            }

            if file_progress {
                let ms = file_start.elapsed().as_millis();
                if ms >= 2_000 {
                    let rel = file
                        .strip_prefix(&src_base)
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|_| file.display().to_string());
                    eprintln!("⚠️  Codegen slow: {} took {}ms", rel, ms);
                }
            }
            _codegen_done += 1;
            report.progress("Step 4B-b codegen file", _codegen_done, total_codegen);
        }

        // Release per-file analysis data for completed files to reclaim memory.
        for idx in batch {
            step4b_analyses[*idx] = None;
        }
    }

    profile_phase(&report, "Step 4B-b: Codegen (parallel)", step4b_codegen_start);
    profile_phase(&report, "Step 4B: Analysis + codegen (sequential)", step4b_start);

    let post_codegen_start = Instant::now();
    // Emit metadata.json for cross-crate call-site ownership. Path dependents discover
    // this via wj.toml (`discover_wj_toml_path_dependency_metadata`). Always write on
    // multipass builds — not only `--library` — so `wj build src -o gen` still publishes
    // converged formals (`emitted_rust_ref_params`) for downstream crates.
    {
        let mut local_keys: Vec<String> = crate_metadata.functions.keys().cloned().collect();
        // Incremental / empty-skeleton builds may leave crate_metadata empty after a
        // deleted metadata.json — recover local method keys from the post-codegen registry.
        if local_keys.is_empty() {
            for name in final_global_registry.signatures.keys() {
                if let Some(struct_name) = crate::metadata::struct_name_from_method_key(name) {
                    if local_struct_names.contains(struct_name) {
                        local_keys.push(name.clone());
                    }
                } else if !name.contains("::") {
                    local_keys.push(name.clone());
                }
            }
        }
        for name in final_global_registry.signatures.keys() {
            if final_global_registry.is_trait_method_key(name) {
                local_keys.push(name.clone());
            }
        }
        local_keys.sort();
        local_keys.dedup();
        for name in local_keys {
            // Prefer post-codegen `final_global_registry` (has `emitted_rust_ref_params`
            // after formal demotion) over pre-codegen `local_converged_sigs`.
            let sig = resolve_converged_local_signature(final_global_registry.as_ref(), &name)
                .cloned()
                .or_else(|| local_converged_sigs.get(&name).cloned())
                .or_else(|| {
                    local_converged_sigs
                        .iter()
                        .find(|(k, _)| k.ends_with(&format!("::{name}")))
                        .map(|(_, v)| v.clone())
                });
            if let Some(sig) = sig {
                let (is_associated, parent_type) = if let Some(struct_name) =
                    crate::metadata::struct_name_from_method_key(&name)
                {
                    (true, Some(struct_name.to_string()))
                } else {
                    (false, None)
                };
                let mut meta_sig =
                    metadata_function_sig_from_analyzer(&sig, is_associated, parent_type);
                crate::metadata::stamp_trait_method_flag(
                    final_global_registry.as_ref(),
                    &name,
                    &mut meta_sig,
                );
                crate_metadata.functions.insert(name, meta_sig);
            }
        }
        crate_metadata.copy_structs = global_copy_structs.iter().cloned().collect::<Vec<_>>();
        crate_metadata.copy_structs.sort();
        crate_metadata.collect_program_facts(&parsed_programs);
        if crate_metadata.has_entries() {
            super::write_crate_metadata_json(output, &crate_metadata)?;
        }
    }

    // Generate mod.rs, Cargo.toml, stale checks only when files were actually
    // regenerated. When nothing changed, skip these expensive I/O operations.
    if codegen_written_count > 0 {
        if target == CompilationTarget::Rust {
            crate::build_utils::generate_mod_file_with_layout(
                output,
                Some((output, src_base.as_path())),
            )?;
            crate::build_utils::cleanup_stale_module_files(output)?;
            // Wire Rust integration tests of the generated crate only: a build writes into its output (and
            // its cache), never into the source tree (it used to rewrite a hand-written `src/lib.rs`).
            if let Some(crate_root) =
                crate::rust_integration_tests::find_project_root_with_tests(output)
                    .filter(|root| root.starts_with(output))
            {
                let _ = crate::rust_integration_tests::sync_rust_integration_tests(&crate_root);
            }
        }

        super::generate_cargo_manifests(base_path, output, target, true, config)?;

        let stale = super::cache_management::find_stale_codegen_outputs_with_dep_epoch(
            &sources, &src_base, output, &cache_root, &dep_roots, Some(dep_epoch_snapshot),
        );
        if !stale.is_empty() {
            eprintln!(
                "⚠️  {} file(s) have stale fingerprints after codegen (will self-heal on next build): {:?}",
                stale.len(),
                stale.iter().take(5).collect::<Vec<_>>()
            );
        }
    }

    // Record the compiler version now that all codegen has completed successfully.
    let _ = super::cache_management::write_compiler_stamp(output);
    profile_phase(&report, "Post-codegen (metadata + mod.rs + stale check)", post_codegen_start);
    report.report_work();

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

/// Merge one analyzed file into the global registry and return its full registry (own layer plus trait methods).
///
/// The file's registry reads through to the global registry as it stood before this file. Everything that needs
/// that view is computed first; then the file's layers are detached, so the global registry is updated in place.
/// Keeping the read-through while merging forced a copy of the whole library registry for every file, and every
/// stored analysis kept its copy alive (memory quadratic in the file count, Q-MEM-1). The merges see exactly
/// the same inputs as before.
#[allow(clippy::too_many_arguments)]
fn merge_file_analysis_into_global(
    report: &crate::mem_report::MemReport,
    analysis: &mut Step4bFileAnalysis,
    global: &mut std::sync::Arc<SignatureRegistry>,
    library_trait_methods: &TraitMethods,
    copy_structs: &std::sync::Arc<HashSet<String>>,
    struct_fields: &std::sync::Arc<HashMap<String, HashMap<String, crate::parser::ast::types::Type>>>,
    module_paths: &std::sync::Arc<HashMap<String, Vec<Vec<String>>>>,
) -> SignatureRegistry {
    let mut full_registry = analysis.registry.clone();
    let merged_trait_methods = analysis.merged_trait_methods(library_trait_methods);
    let mut tmp_analyzer = Analyzer::for_library_pass(copy_structs.clone(), struct_fields.clone(), module_paths.clone());
    tmp_analyzer.analyzed_trait_methods = merged_trait_methods.clone();
    tmp_analyzer.register_trait_methods_in_registry(&merged_trait_methods, &mut full_registry);
    write_back_counted(report, &mut full_registry);
    let alias_trait_keys = analysis.registry.trait_key_flags_of_own_signatures();
    analysis.registry.detach_shared_layers();
    full_registry.detach_shared_layers();
    let reg = global_mut(report, global);
    reg.merge(&analysis.registry);
    reg.register_module_aliases_with_trait_flags(&analysis.registry, &alias_trait_keys, &analysis.file_stem, &analysis.module_path);
    reg.merge(&full_registry);
    write_back_counted(report, reg);
    full_registry
}

/// Trait owned-string write-back, counting the impl keys it walks (`trait_keys_scanned`).
fn write_back_counted(report: &crate::mem_report::MemReport, registry: &mut SignatureRegistry) {
    let walked = crate::codegen::rust::call_signature_resolution::write_back_trait_owned_string_to_impls(registry);
    report.count(crate::mem_report::Work::TraitKeysScanned, walked as u64);
}

/// Write back onto the shared global registry only when it would change something: a file's layered
/// registry still reads the global, so writing it copies it whole.
fn write_back_global_if_needed(report: &crate::mem_report::MemReport, global: &mut std::sync::Arc<SignatureRegistry>) {
    let (noop, walked) = crate::codegen::rust::call_signature_resolution::trait_owned_string_write_back_is_noop(global);
    report.count(crate::mem_report::Work::TraitKeysScanned, walked as u64);
    if !noop {
        write_back_counted(report, global_mut(report, global));
    }
}

struct Step4bFileAnalysis {
    registry: SignatureRegistry,
    analyzed_functions: Vec<crate::analyzer::AnalyzedFunction<'static>>,
    /// The file's own analyzed trait methods; `None` for a file skipped as unchanged (it merges no trait
    /// methods). The library-wide ones are overlaid on demand ([`Step4bFileAnalysis::merged_trait_methods`])
    /// instead of storing a copy of every library trait method per file (quadratic, Q-MEM-1).
    own_trait_methods: Option<TraitMethods>,
    copy_structs: Vec<String>,
    output_file: PathBuf,
    file_stem: String,
    module_path: String,
    lint_errors: Vec<String>,
}

type TraitMethods = HashMap<String, HashMap<String, crate::analyzer::AnalyzedFunction<'static>>>;

impl Step4bFileAnalysis {
    /// The file's trait methods with the library-wide ones laid over them (library entries win).
    fn merged_trait_methods(&self, global: &TraitMethods) -> TraitMethods {
        let Some(own) = &self.own_trait_methods else {
            return HashMap::new();
        };
        let mut merged = own.clone();
        for (trait_name, methods) in global.iter() {
            let entry = merged.entry(trait_name.clone()).or_default();
            for (method_name, method_analysis) in methods {
                entry.insert(method_name.clone(), method_analysis.clone());
            }
        }
        merged
    }
}

#[allow(clippy::too_many_arguments)]
fn analyze_file_for_step4b(
    i: usize,
    sources: &[(PathBuf, String)],
    parsed_programs: &[crate::parser::Program<'static>],
    stripped_programs: &[Option<crate::parser::Program<'static>>],
    src_base: &Path,
    output: &Path,
    global_registry: &std::sync::Arc<SignatureRegistry>,
    global_copy_structs: &std::sync::Arc<std::collections::HashSet<String>>,
    global_struct_fields: &std::sync::Arc<
        HashMap<String, HashMap<String, crate::parser::ast::types::Type>>,
    >,
    struct_defining_module_paths: &std::sync::Arc<HashMap<String, Vec<Vec<String>>>>,
    library_consts: &std::sync::Arc<HashMap<String, crate::parser::ast::types::Type>>,
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
    analyzer.ownership_preconverged = true;
    analyzer.seed_module_const_types(library_consts.clone());

    analyzer
        .register_traits_from_program(program)
        .map_err(|e| format!("Trait registration: {}", e))?;

    let (analyzed_functions, registry, _) = analyzer
        .analyze_program_with_global_arc(program, global_registry)
        .map_err(|e| format!("Final analysis error: {}", e))?;

    let own_trait_methods = Some(analyzer.analyzed_trait_methods.clone());

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
        own_trait_methods,
        copy_structs: analyzer.get_copy_structs(),
        output_file,
        file_stem,
        module_path,
        lint_errors,
    })
}

#[allow(clippy::too_many_arguments)]
fn run_parallel_numeric_inference(
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
    report: &crate::mem_report::MemReport,
) -> crate::ir::numeric_bridge::UnifiedNumericInference {
    use crate::ir::numeric_bridge::UnifiedNumericInference;

    let mut global = UnifiedNumericInference::new();
    if !external_paths.is_empty() {
        global.set_external_crate_metadata_paths(external_paths);
        for (key, dir) in external_paths.iter() {
            for name in crate::metadata::load_string_const_names(dir) {
                global.register_string_const(&name);
            }
            global.add_external_crate_consts(key, crate::metadata::load_const_types_from_metadata(dir));
        }
    }
    global.set_global_function_signatures(global_float_signatures);
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

    report.phase("Step 4A numeric: programs prepared");
    // Each partial starts as a clone of the prepared whole-library state, so holding one partial per file
    // at once costs files x library (quadratic: 7+ GB on a 677-file library). Collect in bounded chunks
    // and merge each chunk in source order before the next: the same partials are merged in the same
    // order, so the solve is unchanged, and at most NUMERIC_PARTIALS_IN_FLIGHT copies are alive.
    let base = global.clone();
    let chunk = numeric_partials_in_flight();
    for chunk_start in (0..sources.len()).step_by(chunk) {
        let chunk_end = (chunk_start + chunk).min(sources.len());
        let partials: Vec<UnifiedNumericInference> = (chunk_start..chunk_end)
            .into_par_iter()
            .map(|i| {
                let file = &sources[i].0;
                let mut local = base.clone();
                let file_module =
                    crate::analyzer::type_collector::wj_file_to_module_path(src_base, file)
                        .unwrap_or_default();
                local.set_current_file_module_path(file_module);
                local.register_own_functions(&parsed_programs[i]);
                local.collect_program_constraints(&parsed_programs[i]);
                local
            })
            .collect();
        for partial in partials {
            global.merge_parallel_state(partial);
        }
        report.progress("Step 4A numeric constraints file", chunk_end, sources.len());
    }
    drop(base);
    global.finish_solve();
    global
}

/// How many per-file numeric partials (each a clone of the prepared library state) are alive at once:
/// enough to keep every worker busy, bounded so memory stays linear in library size.
fn numeric_partials_in_flight() -> usize {
    (rayon::current_num_threads() * 2).clamp(1, 32)
}

/// The binary is its own crate: `crate::x` paths (in `use` and in expressions) name the package lib, and the
/// lib's public modules are in scope as they are for the crate root in Windjammer (`util::f()` without `use`).
fn bin_main_against_lib(content: &str, lib_name: &str) -> String {
    let body = content.replace("crate::", &format!("{}::", lib_name));
    let glob = format!("#[allow(unused_imports)]\nuse {}::*;\n", lib_name);
    if body.contains(&glob) {
        return body;
    }
    let split = body
        .lines()
        .take_while(|l| l.trim_start().starts_with("#!") || l.trim().is_empty())
        .map(|l| l.len() + 1)
        .sum::<usize>()
        .min(body.len());
    format!("{}{}{}", &body[..split], glob, &body[split..])
}

/// The crate name a bin entry imports its library by. A file build (`wj build runner.wj`) has the entry's directory as
/// its whole package: the generated Cargo.toml names the lib by `infer_project_name_from(<entry dir>)`, and the bin must
/// import that same name even when no manifest configures one.
fn bin_entry_lib_name(src_base: &Path, base_path: &Path, config: &crate::compiler_config::CompilerConfig) -> Option<String> {
    crate::cargo_toml::infer_configured_package_name(src_base)
        .or_else(|| config.entry_use_paths_name_sibling_modules.then(|| crate::cargo_toml::infer_project_name_from(base_path)))
}

/// Rewrite `use crate::…` in a binary `main.rs` so it resolves against the package lib.
fn rewrite_bin_crate_imports_to_lib(main_rs: &Path, lib_name: &str) -> Result<()> {
    if !main_rs.exists() {
        return Ok(());
    }
    let content = std::fs::read_to_string(main_rs)?;
    let updated = bin_main_against_lib(&content, lib_name);
    if updated != content {
        std::fs::write(main_rs, updated)?;
    }
    Ok(())
}

#[cfg(test)]
mod bin_main_tests {
    use super::bin_main_against_lib;

    #[test]
    fn bin_main_names_the_lib_for_crate_paths_and_sees_its_modules() {
        let main = "#![allow(x)]\nuse crate::greet;\nfn main() {\n    crate::util::f();\n    util::g();\n}\n";
        let out = bin_main_against_lib(main, "app");
        assert!(out.starts_with("#![allow(x)]\n#[allow(unused_imports)]\nuse app::*;\n"), "{out}");
        assert!(out.contains("use app::greet;") && out.contains("app::util::f();"), "{out}");
        assert_eq!(bin_main_against_lib(&out, "app"), out, "idempotent");
    }
}

/// Registry keys of the functions `program` itself defines: free functions by name, methods as `Type::method`.
fn own_definition_keys(program: &crate::parser::Program<'_>) -> std::collections::HashSet<String> {
    use crate::parser::ast::core::Item;
    let mut keys = std::collections::HashSet::new();
    for item in &program.items {
        match item {
            Item::Function { decl, .. } => {
                keys.insert(decl.name.clone());
            }
            Item::Impl { block, .. } => {
                let ty = block.type_name.split('<').next().unwrap_or(&block.type_name);
                keys.extend(block.functions.iter().map(|f| format!("{ty}::{}", f.name)));
            }
            _ => {}
        }
    }
    keys
}

/// After global promotions, a key this file defines must still describe *this* file's function: a
/// global entry with a different declared shape (formal types or return type) belongs to a same-name
/// function of another file (C3 identity), so the file's own analysis is restored for it.
fn keep_own_definitions(
    local: &mut SignatureRegistry,
    own_analysis: &SignatureRegistry,
    own_defs: &std::collections::HashSet<String>,
) {
    for name in own_defs {
        let Some(own) = own_analysis.signatures.get(name) else { continue };
        let foreign = local.get_signature(name).is_some_and(|cur| !cur.declares_same_function(own));
        if foreign {
            local.signatures.insert(name.clone(), own.clone());
        }
    }
}

type FieldTypes = HashMap<String, HashMap<String, crate::parser::ast::types::Type>>;

/// Field and variant-payload types of every file of the library (an enum declared in another module types
/// `Shape::Sphere(0.5)`), over the merged struct fields; the first declaration of a name is kept.
fn library_field_types(global: &FieldTypes, programs: &[crate::parser::Program<'static>]) -> FieldTypes {
    let mut out = global.clone();
    for p in programs {
        for (k, v) in crate::type_inference::struct_field_registry::local_struct_field_table(p) {
            out.entry(k).or_insert(v);
        }
    }
    out
}

/// The field types one file's IR is lowered with: the library's, with the file's own declarations winning over a
/// homonym elsewhere.
///
/// The file's declarations are laid over the library table for the duration of `f` and then undone, instead of
/// copying the library table for every file (and every fixed-point round).
fn with_file_field_types<R>(
    library: &mut FieldTypes,
    program: &crate::parser::Program<'static>,
    f: impl FnOnce(&FieldTypes) -> R,
) -> R {
    let local = crate::type_inference::struct_field_registry::local_struct_field_table(program);
    let displaced: Vec<(String, Option<HashMap<String, crate::parser::ast::types::Type>>)> =
        local.into_iter().map(|(k, v)| (k.clone(), library.insert(k, v))).collect();
    let out = f(library);
    for (k, previous) in displaced {
        match previous {
            Some(v) => {
                library.insert(k, v);
            }
            None => {
                library.remove(&k);
            }
        }
    }
    out
}

/// Publish every file's solved value-formal modes (convergence O4); true when one changed.
fn publish_value_formals(
    scope: &super::value_formal_fixed_point::FormalScope,
    registry: &mut SignatureRegistry,
    sources: &[(PathBuf, String)],
    ir_functions: &[Option<Vec<crate::ir::node::IrFunction>>],
    analyses: &[Option<Step4bFileAnalysis>],
) -> bool {
    let mut changed = false;
    for (i, (ir, analysis)) in ir_functions.iter().zip(analyses).enumerate() {
        if let (Some(ir), Some(analysis)) = (ir, analysis) {
            let stem = sources[i].0.file_stem().and_then(|s| s.to_str());
            changed |= scope.publish(registry, stem, ir, &analysis.analyzed_functions);
        }
    }
    changed
}

/// Each file's codegen overlays its own analysis-time registry on the global one: its copies of other files'
/// signatures must carry the published value-formal modes too.
///
/// Same result as running [`publish_value_formals`] on every file's registry, without visiting every function
/// of the library for every file: each registry gets only the publications whose key it holds, applied in the
/// same (file, function, key) order. A publication only rewrites a key that is already present, so the set of
/// keys of a registry does not change while publishing.
fn publish_into_file_registries(
    scope: &super::value_formal_fixed_point::FormalScope,
    sources: &[(PathBuf, String)],
    ir_functions: &[Option<Vec<crate::ir::node::IrFunction>>],
    analyses: &mut [Option<Step4bFileAnalysis>],
) {
    // Every publication in library order: (file, function, key).
    let mut publications: Vec<(usize, usize, String)> = Vec::new();
    for (i, (ir, analysis)) in ir_functions.iter().zip(analyses.iter()).enumerate() {
        if let (Some(ir), Some(analysis)) = (ir, analysis) {
            let stem = sources[i].0.file_stem().and_then(|s| s.to_str());
            for (f, (ir_fn, af)) in ir.iter().zip(&analysis.analyzed_functions).enumerate() {
                for key in scope.publish_keys(stem, ir_fn, af) {
                    publications.push((i, f, key));
                }
            }
        }
    }
    let mut by_key: HashMap<&str, Vec<usize>> = HashMap::new();
    for (n, (_, _, key)) in publications.iter().enumerate() {
        by_key.entry(key.as_str()).or_default().push(n);
    }
    for j in 0..analyses.len() {
        let Some(mut registry) = analyses[j].as_mut().map(|a| std::mem::take(&mut a.registry)) else {
            continue;
        };
        let mut mine: Vec<usize> = registry
            .signatures
            .keys()
            .filter_map(|k| by_key.get(k.as_str()))
            .flatten()
            .copied()
            .collect();
        mine.sort_unstable();
        for n in mine {
            let (i, f, key) = &publications[n];
            let (Some(ir), Some(analysis)) = (&ir_functions[*i], &analyses[*i]) else { continue };
            scope.publish_key(&mut registry, key.clone(), &ir[*f], &analysis.analyzed_functions[*f]);
        }
        if let Some(a) = analyses[j].as_mut() {
            a.registry = registry;
        }
    }
}
