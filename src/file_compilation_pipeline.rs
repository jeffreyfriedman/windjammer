//! Orchestration for per-file compilation: `compile_file` and `compile_file_with_compiler`.
//!
//! Separated from `file_compiler` so `ModuleCompiler` and path resolution stay in one place.

use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::CompilationTarget;
use crate::{analyzer, inference, linter, parser};
use crate::{lexer, metadata};

use crate::file_compiler::ModuleCompiler;
use crate::output_generation::{
    generate_main_rust_code, write_single_file_outputs, MainCodegenOutcome,
};

/// Walk up from `start` to find a directory containing `metadata.json` (typically `src/`).
fn metadata_search_root(start: &Path) -> std::path::PathBuf {
    let mut dir = start.to_path_buf();
    loop {
        if dir.join("metadata.json").exists() {
            return dir;
        }
        if !dir.pop() {
            break;
        }
    }
    start.to_path_buf()
}

pub fn compile_file(
    input_path: &Path,
    output_dir: &Path,
    target: CompilationTarget,
) -> Result<(HashSet<String>, Vec<String>)> {
    let mut module_compiler = ModuleCompiler::new(target, true);
    // Search upward for metadata.json so subdir single-file builds see crate signatures.
    let source_root = metadata_search_root(input_path.parent().unwrap_or(Path::new(".")));
    let is_multi_file = false; // Single file compilation
    compile_file_with_compiler(
        &source_root,
        input_path,
        output_dir,
        &mut module_compiler,
        is_multi_file,
        true,
    )
}

/// Compile a file with a provided ModuleCompiler (for shared trait registry)
pub fn compile_file_with_compiler(
    source_root: &Path,
    input_path: &Path,
    output_dir: &Path,
    module_compiler: &mut ModuleCompiler,
    is_multi_file_project: bool,
    store_program: bool, // Whether to add this program to all_programs for trait inference
) -> Result<(HashSet<String>, Vec<String>)> {
    // RECURSION GUARD: Prevent circular module dependencies from causing stack overflow
    // THE WINDJAMMER WAY: Use normalized string for cross-platform path comparison
    // Problem: Windows canonicalize() adds UNC prefixes (\\?\C:\...) inconsistently,
    // causing PathBuf equality checks to fail even for the same file.
    // Solution: Convert to lowercase string with forward slashes for consistent comparison.
    let canonical_path = input_path.canonicalize().unwrap_or_else(|_| {
        // If canonicalize fails (file doesn't exist yet), use absolute path
        std::env::current_dir()
            .ok()
            .and_then(|cwd| cwd.join(input_path).canonicalize().ok())
            .unwrap_or_else(|| input_path.to_path_buf())
    });

    // Normalize path for consistent comparison across platforms
    // Remove UNC prefix on Windows and use forward slashes
    let path_key = {
        let normalized = canonical_path
            .to_string_lossy()
            .replace("\\\\?\\", "") // Remove Windows UNC prefix
            .replace('\\', "/"); // Normalize to forward slashes

        // Only lowercase on Windows (case-insensitive filesystem)
        // macOS/Linux filesystems are case-sensitive!
        #[cfg(target_os = "windows")]
        {
            normalized.to_lowercase()
        }
        #[cfg(not(target_os = "windows"))]
        {
            normalized
        }
    };

    // DEBUG: Print ALL currently compiling files for Windows debugging
    if !module_compiler.compiling_files.is_empty() {
        eprintln!(
            "🔍 Currently compiling {} files:",
            module_compiler.compiling_files.len()
        );
        for (idx, file) in module_compiler.compiling_files.iter().enumerate() {
            eprintln!("   [{}] {}", idx, file);
        }
        eprintln!("🔍 Checking: {}", path_key);
    }

    if module_compiler.compiling_files.contains(&path_key) {
        // Already compiling this file in the current chain - skip to prevent infinite recursion
        // This is OK and expected for circular imports that have already been handled
        eprintln!(
            "⚠️  RECURSION GUARD TRIGGERED: Skipping {} (already in compilation chain)",
            path_key
        );
        eprintln!(
            "   Currently compiling: {}",
            module_compiler.compiling_files.len()
        );
        eprintln!("   🚨 WARNING: This will cause an EMPTY FILE to be written!");
        return Ok((HashSet::new(), Vec::new()));
    }

    // Check recursion depth as additional safety
    if module_compiler.compiling_files.len() >= 50 {
        anyhow::bail!("Maximum module nesting depth exceeded (50 files). Possible circular dependency involving: {}", path_key);
    }

    module_compiler.compiling_files.insert(path_key.clone());
    eprintln!(
        "✅ RECURSION GUARD: Added {} to compilation set (now {} files)",
        path_key,
        module_compiler.compiling_files.len()
    );

    // THE WINDJAMMER WAY: Always cleanup, whether we succeed or fail
    // Call the implementation, then remove path from set regardless of result
    let result = compile_file_impl(
        source_root,
        input_path,
        module_compiler,
        output_dir,
        is_multi_file_project,
        store_program,
        &path_key,
    );

    // Remove path from compilation set now that we're done (success or failure)
    // This runs whether result is Ok or Err
    module_compiler.compiling_files.remove(&path_key);
    eprintln!(
        "✅ RECURSION GUARD: Removed {} from compilation set (now {} files)",
        path_key,
        module_compiler.compiling_files.len()
    );

    result
}

/// Internal implementation of compile_file_with_compiler
/// This is separated out so we can ensure cleanup happens in the outer function
fn compile_file_impl(
    source_root: &Path,
    input_path: &Path,
    module_compiler: &mut ModuleCompiler,
    output_dir: &Path,
    is_multi_file_project: bool,
    store_program: bool,
    _path_key: &str,
) -> Result<(HashSet<String>, Vec<String>)> {
    eprintln!(
        "🚀 ENTERED compile_file_impl for {:?}",
        input_path.file_name()
    );
    let target = module_compiler.target;

    // Read source file
    let source = std::fs::read_to_string(input_path)?;
    // Lex
    let mut lexer = lexer::Lexer::new(&source);
    let tokens = lexer.tokenize_with_locations();

    // Parse
    let mut wj_parser = parser::Parser::new_with_source(
        tokens,
        input_path.to_string_lossy().to_string(),
        source.clone(),
    );
    let program = wj_parser
        .parse()
        .map_err(|e| anyhow::anyhow!("Parse error: {}", e))?;

    // Emit parser warnings (W0010: non-canonical string types, etc.)
    for w in wj_parser.warnings() {
        eprintln!(
            "warning: {} [{}:{}:{}]",
            w.message,
            w.file.as_deref().unwrap_or("<unknown>"),
            w.line.unwrap_or(0),
            w.column.unwrap_or(0),
        );
    }

    // Content-based shader detection: skip files with @vertex/@fragment/@compute
    // from the Rust pipeline. These should be compiled via the WJSL→WGSL path.
    if crate::compiler::is_shader_file(&program) {
        eprintln!(
            "  Skipping shader file {:?} from Rust pipeline (use WJSL target for GPU shaders)",
            input_path.file_name()
        );
        return Ok((HashSet::new(), Vec::new()));
    }

    // LANGUAGE DESIGN CHECK: Prohibit Rust-specific patterns (.as_str())
    // This must happen immediately after parsing, before any other processing
    {
        eprintln!(
            "🔍 LANGUAGE CHECK (file_impl): Scanning for .as_str() in {:?}",
            input_path.file_name()
        );
        let checker_analyzer = analyzer::Analyzer::new();
        checker_analyzer
            .check_forbidden_rust_patterns(&program)
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        eprintln!("✅ LANGUAGE CHECK (file_impl): No .as_str() found");
    }

    // Cross-file ownership: load peer `*.wj.meta` (from prior `wj build` runs) into the registry
    // before analysis so call sites get `&` / `&mut` for callees defined in other files.
    metadata::merge_wj_meta_signatures_from_dir(
        source_root,
        &mut module_compiler.global_signatures,
    );

    // Rust leakage linter: warn about &self, .unwrap(), .iter(), & in calls
    if module_compiler.enable_lint {
        let file_name = input_path.to_string_lossy().to_string();
        let mut rust_leakage = linter::rust_leakage::RustLeakageLinter::new(&file_name);
        rust_leakage.lint_program(&program);
        for diag in rust_leakage.diagnostics() {
            eprintln!("{}", diag);
        }
    }

    // DEBUG: Print Item::Mod entries in the AST
    if std::env::var("WJ_DEBUG_AST").is_ok() {
        let file_name = input_path.file_name().unwrap().to_string_lossy();
        eprintln!("\n=== AST for {} ===", file_name);
        for (idx, item) in program.items.iter().enumerate() {
            if let parser::Item::Mod {
                name,
                items,
                is_public,
                ..
            } = item
            {
                eprintln!(
                    "  Item #{}: {}mod {} (items.len() = {})",
                    idx,
                    if *is_public { "pub " } else { "" },
                    name,
                    items.len()
                );
                if !items.is_empty() {
                    eprintln!("    INLINE MODULE with {} items:", items.len());
                    for (i, nested) in items.iter().enumerate() {
                        match nested {
                            parser::Item::Struct { decl, .. } => {
                                eprintln!("      #{}: struct {}", i, decl.name)
                            }
                            parser::Item::Function { decl, .. } => {
                                eprintln!("      #{}: fn {}", i, decl.name)
                            }
                            _ => eprintln!("      #{}: {:?}", i, nested),
                        }
                    }
                }
            }
        }
        eprintln!("=== End AST ===\n");
    }

    // THE WINDJAMMER WAY: Store this program for cross-file trait inference
    // Only store if requested (to avoid duplicates during regeneration)
    if store_program {
        module_compiler.all_programs.push(program.clone());
    }

    // ARENA FIX: ALWAYS store the parser to keep arena alive
    // The shared analyzer accumulates AST references from all files,
    // so we must keep all parsers alive for the entire compilation
    module_compiler._parsers.push(wj_parser);
    // Note: wj_parser has been moved and can't be used after this

    // Compile dependencies first (both use statements and mod declarations)
    for item in &program.items {
        // Handle use statements
        if let parser::Item::Use { path, alias: _, .. } = item {
            let module_path = path.join("::");

            // Handle item imports: ./main::Args -> compile ./main, not ./main::Args
            // Also handle braced imports: ./message::{A, B, C} -> compile ./message
            // Split at the last :: to separate module from item
            let module_to_compile = if module_path.contains("::") {
                // Check if this looks like a module::Item import or module::{...} import
                let parts: Vec<&str> = module_path.rsplitn(2, "::").collect();
                if parts.len() == 2 {
                    let potential_item = parts[0];
                    let module_part = parts[1];

                    // If the last part starts with uppercase or {, it's likely a type/braced import
                    // e.g., ./main::Args, ./types::Config, ./message::{A, B, C}
                    if potential_item
                        .chars()
                        .next()
                        .is_some_and(|c| c.is_uppercase() || c == '{')
                    {
                        module_part.to_string()
                    } else {
                        // It's a nested module path, compile the whole thing
                        module_path.clone()
                    }
                } else {
                    module_path.clone()
                }
            } else {
                module_path.clone()
            };

            // Compile both std::* and relative imports (./ or ../) and external crates
            module_compiler.compile_module(&module_to_compile, Some(input_path))?;
        }

        // Handle module declarations (pub mod math;)
        if let parser::Item::Mod { name, items, .. } = item {
            // Only process external module declarations (items.is_empty() means no inline body)
            if items.is_empty() {
                // Find the module file: either math.wj or math/mod.wj
                let parent_dir = input_path.parent().unwrap_or(Path::new("."));

                // Try math.wj first
                let module_file = parent_dir.join(format!("{}.wj", name));
                let module_dir_file = parent_dir.join(name).join("mod.wj");

                let module_path_to_compile = if module_file.exists() {
                    Some(module_file)
                } else if module_dir_file.exists() {
                    Some(module_dir_file)
                } else {
                    // Module file doesn't exist yet - might be empty directory
                    // This is OK, we'll just not compile it
                    None
                };

                if let Some(mod_path) = module_path_to_compile {
                    // Recursively compile the module
                    compile_file_with_compiler(
                        source_root,
                        &mod_path,
                        output_dir,
                        module_compiler,
                        is_multi_file_project,
                        store_program, // Pass through the store_program flag
                    )?;
                }
            }
        }
    }

    // Register traits and struct field types from this program into the global registry
    for item in &program.items {
        if let parser::Item::Trait { decl, .. } = item {
            module_compiler
                .trait_registry
                .insert(decl.name.clone(), decl.clone());
        }
        // CROSS-MODULE STRUCT FIELD TYPES: Register struct field types for imported type inference
        if let parser::Item::Struct { decl, .. } = item {
            let mut field_types = HashMap::new();
            for field in &decl.fields {
                field_types.insert(field.name.clone(), field.field_type.clone());
            }
            module_compiler
                .global_struct_field_types
                .insert(decl.name.clone(), field_types);
        }
    }

    // WINDJAMMER FIX: Use the SHARED analyzer from module_compiler
    // This ensures trait methods analyzed in file 1 are available when analyzing impl in file 2

    // Update analyzer's Copy structs registry
    module_compiler
        .analyzer
        .update_copy_structs(module_compiler.copy_structs_registry.clone());
    // Provide cross-file struct field types for nested field chain resolution
    module_compiler
        .analyzer
        .set_global_struct_field_types(module_compiler.global_struct_field_types.clone());

    // Register any newly discovered traits
    for trait_decl in module_compiler.trait_registry.values() {
        let dummy_program = parser::Program {
            items: vec![parser::Item::Trait {
                decl: trait_decl.clone(),
                location: parser::SourceLocation::default(),
            }],
        };
        module_compiler
            .analyzer
            .register_traits_from_program(&dummy_program)
            .map_err(|e| anyhow::anyhow!("register_traits_from_program: {}", e))?;
    }

    let (analyzed, signatures, analyzed_trait_methods) = module_compiler
        .analyzer
        .analyze_program_with_global_signatures(&program, &module_compiler.global_signatures)
        .map_err(|e| anyhow::anyhow!("Analysis error: {}", e))?;

    // THE WINDJAMMER WAY: Run linter after analysis
    // Single-file `wj build` uses store_program=true; lints must still run so stderr warnings work.
    let mut linter = linter::Linter::new();
    for analyzed_func in &analyzed {
        linter.lint_function(analyzed_func);
    }
    let diagnostics = linter.into_diagnostics();
    for diagnostic in &diagnostics {
        if diagnostic.level != linter::LintLevel::Allow {
            eprintln!("{}", diagnostic);
        }
    }

    // BUG FIX: Merge per-file signatures into global registry during PASS 1
    // This ensures extern function signatures (and all other signatures) are available
    // during PASS 2 regeneration for cross-file and same-file signature resolution
    if store_program {
        module_compiler.global_signatures.merge(&signatures);
    }

    crate::compilation_error_handling::check_top_level_mutability(input_path, &program)?;

    // THE WINDJAMMER WAY: During regeneration, use the GLOBAL analyzed trait methods
    // (which have been updated by finalize_trait_inference)
    if !store_program {
        // This is a regeneration pass - use the global inferred trait methods
        eprintln!("DEBUG REGEN: Using global trait methods for regeneration");
        eprintln!(
            "DEBUG REGEN: Global trait methods has {} traits",
            analyzed_trait_methods.len()
        );
        for (trait_name, methods) in &analyzed_trait_methods {
            eprintln!(
                "DEBUG REGEN:   GLOBAL Trait {} has {} methods",
                trait_name,
                methods.len()
            );
            for (method_name, method_analysis) in methods {
                eprintln!("DEBUG REGEN:     GLOBAL Method {} inferred:", method_name);
                for (param_name, ownership) in &method_analysis.inferred_ownership {
                    eprintln!(
                        "DEBUG REGEN:       BEFORE CLONE: {} = {:?}",
                        param_name, ownership
                    );
                }
            }
        }
        // Note: analyzed_trait_methods is already synchronized with module_compiler.analyzer
        // No need to clone - we're using the same HashMap that was returned from analyze_program
        eprintln!(
            "DEBUG REGEN: After clone, analyzed_trait_methods has {} traits",
            analyzed_trait_methods.len()
        );
        for (trait_name, methods) in &analyzed_trait_methods {
            for (method_name, method_analysis) in methods {
                eprintln!(
                    "DEBUG REGEN:     AFTER CLONE {}.{} self={:?}",
                    trait_name,
                    method_name,
                    method_analysis.inferred_ownership.get("self")
                );
            }
        }
    }

    // Infer trait bounds
    let mut inference_engine = inference::InferenceEngine::new();
    let mut inferred_bounds_map = std::collections::HashMap::new();
    for item in &program.items {
        if let parser::Item::Function { decl: func, .. } = item {
            let bounds = inference_engine.infer_function_bounds(func);
            if !bounds.is_empty() {
                inferred_bounds_map.insert(func.name.clone(), bounds);
            }
        }
    }

    match generate_main_rust_code(
        target,
        source_root,
        input_path,
        output_dir,
        module_compiler,
        is_multi_file_project,
        &program,
        &analyzed,
        &signatures,
        analyzed_trait_methods,
        inferred_bounds_map,
        &source,
    )? {
        MainCodegenOutcome::EarlySuccess => Ok((HashSet::new(), Vec::new())),
        MainCodegenOutcome::RustCode(rust_code) => write_single_file_outputs(
            target,
            source_root,
            input_path,
            output_dir,
            module_compiler,
            is_multi_file_project,
            &program,
            &signatures,
            rust_code,
        ),
    }
}
