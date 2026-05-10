//! Single-file compilation module
//!
//! This module handles compilation of individual .wj files, including:
//! - Single-file compilation with ModuleCompiler
//! - Recursion guards for circular dependencies
//! - Module use statement processing
//! - Path normalization and canonicalization
//! - Imported module tracking

use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

// Import compiler internals (ModuleCompiler is defined in this file)
use crate::{CompilationTarget, lexer, parser, parser_impl, project_paths};
use crate::{analyzer, codegen, component_analyzer, errors, inference, linter, type_inference};
pub struct ModuleCompiler {
    pub compiled_modules: HashMap<String, String>, // module path -> generated Rust code
    pub target: CompilationTarget,
    pub enable_lint: bool, // Run Rust leakage linter (W0001-W0004)
    pub stdlib_path: PathBuf,
    pub source_roots: Vec<PathBuf>, // Additional source roots (e.g., ../windjammer-game-core/src)
    pub imported_stdlib_modules: HashSet<String>, // Track which stdlib modules are used
    pub external_crates: Vec<String>, // Track external crates (e.g., windjammer_ui)
    pub trait_registry: HashMap<String, parser::TraitDecl<'static>>, // Global trait registry for cross-file trait resolution
    pub copy_structs_registry: HashSet<String>, // Global Copy struct registry for proper Copy detection across files
    pub analyzer: analyzer::Analyzer<'static>, // WINDJAMMER FIX: Shared analyzer for cross-file trait analysis
    // THE WINDJAMMER WAY: Track ALL programs for cross-file trait inference
    pub all_programs: Vec<parser::Program<'static>>, // All parsed programs from all files
    // ARENA FIX: Keep parsers alive to prevent use-after-free
    pub _parsers: Vec<parser::Parser>, // Parsers that own the arenas for all_programs
    pub _trait_parsers: Vec<parser_impl::Parser>, // ARENA FIX: Parsers for trait_registry
    // RECURSION GUARD: Track files currently being compiled to prevent circular dependencies
    // Use String instead of PathBuf for Windows UNC path compatibility
    pub compiling_files: HashSet<String>, // Normalized path strings in the current compilation chain
    // BUG #8 FIX: Global signature registry for cross-file method signature resolution
    // This enables correct argument passing for methods defined in other modules
    pub global_signatures: analyzer::SignatureRegistry, // All method signatures from all files
    // CROSS-MODULE STRUCT FIELD TYPES: Track all struct field types across files
    // Enables type inference for field accesses on imported structs (e.g., stack.quantity → i32)
    // Without this, Copy-type fields on cross-module structs get unnecessary .clone()
    pub global_struct_field_types: HashMap<String, HashMap<String, parser::Type>>,
}

#[allow(dead_code)]
impl ModuleCompiler {
    pub fn new(target: CompilationTarget, enable_lint: bool) -> Self {
        // Check for WINDJAMMER_STDLIB env var, otherwise use ./std
        let stdlib_path = std::env::var("WINDJAMMER_STDLIB")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("./std"));

        Self {
            compiled_modules: HashMap::new(),
            target,
            enable_lint,
            stdlib_path,
            source_roots: Vec::new(),
            imported_stdlib_modules: HashSet::new(),
            external_crates: Vec::new(),
            trait_registry: HashMap::new(),
            copy_structs_registry: HashSet::new(),
            analyzer: analyzer::Analyzer::new(), // WINDJAMMER FIX: Shared analyzer instance
            all_programs: Vec::new(),            // THE WINDJAMMER WAY: Track all programs
            _parsers: Vec::new(),                // ARENA FIX: Keep parsers alive
            _trait_parsers: Vec::new(),          // ARENA FIX: Keep trait parsers alive
            compiling_files: HashSet::new(),     // RECURSION GUARD: Track compilation chain
            global_signatures: analyzer::SignatureRegistry::new(), // BUG #8 FIX: Global signatures
            global_struct_field_types: HashMap::new(), // Cross-module struct field types
        }
    }

    pub fn add_source_root(&mut self, path: PathBuf) {
        self.source_roots.push(path);
    }

    /// THE WINDJAMMER WAY: Run cross-file trait inference after all files are analyzed
    /// This ensures trait signatures are inferred from ALL implementations across the project
    pub fn finalize_trait_inference(&mut self) -> Result<()> {
        // Create a merged program with ALL items from ALL files
        let mut all_items = Vec::new();
        for program in &self.all_programs {
            all_items.extend(program.items.clone());
        }

        let merged_program = parser::Program { items: all_items };

        // Run the cross-file trait inference
        self.analyzer
            .infer_trait_signatures_from_impls(&merged_program)
            .map_err(|e| anyhow::anyhow!("Trait inference error: {}", e))?;

        Ok(())
    }

    fn compile_module(&mut self, module_path: &str, source_file: Option<&Path>) -> Result<()> {
        // Skip if already compiled
        if self.compiled_modules.contains_key(module_path) {
            return Ok(());
        }

        // Skip stdlib modules - they're implemented in windjammer-runtime
        if module_path.starts_with("std::") {
            // Track that we used this stdlib module
            let module_name = module_path.strip_prefix("std::").unwrap().to_string();
            self.imported_stdlib_modules.insert(module_name);

            // Mark as compiled (no code generated, handled by runtime)
            self.compiled_modules
                .insert(module_path.to_string(), String::new());
            return Ok(());
        }

        // Resolve module path to file path
        let file_path = self.resolve_module_path(module_path, source_file)?;

        // Check if this is a source root module (marked by __source_root__ prefix)
        // These are modules from configured source roots that shouldn't be recursively compiled
        // when building individual files (they'll be in the same Rust module)
        if file_path
            .to_str()
            .is_some_and(|s| s.starts_with("__source_root__::"))
        {
            // Mark as compiled but don't generate code (will be compiled separately)
            self.compiled_modules
                .insert(module_path.to_string(), String::new());
            return Ok(());
        }

        // Check if this is an external crate (marked by __external__ prefix)
        if file_path
            .to_str()
            .is_some_and(|s| s.starts_with("__external__::"))
        {
            // External crate - extract crate name and mark as external dependency
            let crate_name = file_path
                .to_str()
                .unwrap()
                .strip_prefix("__external__::")
                .unwrap()
                .replace(".*", "") // Remove glob imports
                .split("::{") // Remove braced imports (::{ syntax)
                .next()
                .unwrap()
                .split(".{") // Remove braced imports (.{ syntax)
                .next()
                .unwrap()
                .split("::") // Take first segment
                .next()
                .unwrap()
                .replace('_', "-"); // Convert underscores to hyphens for Cargo.toml

            // Add to external crates if not already present
            if !self.external_crates.contains(&crate_name) {
                self.external_crates.push(crate_name.clone());
            }

            // Mark as compiled (external, no code generated)
            self.compiled_modules
                .insert(module_path.to_string(), String::new());
            return Ok(());
        }

        // Read and parse module
        let source = std::fs::read_to_string(&file_path)
            .map_err(|e| anyhow::anyhow!("Failed to read module {}: {}", module_path, e))?;

        let mut lexer = lexer::Lexer::new(&source);
        let tokens = lexer.tokenize_with_locations();
        let mut parser = parser::Parser::new(tokens);
        let program = parser
            .parse()
            .map_err(|e| anyhow::anyhow!("Parse error in {}: {}", module_path, e))?;

        // LANGUAGE DESIGN CHECK: Prohibit Rust-specific patterns (.as_str())
        // This must happen immediately after parsing, before any other processing
        {
            eprintln!(
                "🔍 LANGUAGE CHECK (compile_module): Scanning {} for .as_str()",
                module_path
            );
            let checker_analyzer = analyzer::Analyzer::new();
            checker_analyzer
                .check_forbidden_rust_patterns(&program)
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            eprintln!(
                "✅ LANGUAGE CHECK (compile_module): No .as_str() found in {}",
                module_path
            );
        }

        // Mark as "being compiled" to prevent infinite recursion
        // We'll update this with the actual code later
        self.compiled_modules
            .insert(module_path.to_string(), String::new());

        // Recursively compile dependencies
        for item in &program.items {
            if let parser::Item::Use { path, alias: _, .. } = item {
                let dep_path = path.join("::");

                // Handle item imports: ./main::Args -> compile ./main, not ./main::Args
                // Also handle braced imports: ./message::{A, B, C} -> compile ./message
                // Split at the last :: to separate module from item
                let module_to_compile = if dep_path.contains("::") {
                    // Check if this looks like a module::Item import or module::{...} import
                    let parts: Vec<&str> = dep_path.rsplitn(2, "::").collect();
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
                            dep_path.clone()
                        }
                    } else {
                        dep_path.clone()
                    }
                } else {
                    dep_path.clone()
                };

                // Pass the current file's path for resolving relative imports
                self.compile_module(&module_to_compile, Some(&file_path))?;
            }
        }

        // Register traits from this program into the global registry
        for item in &program.items {
            if let parser::Item::Trait { decl, .. } = item {
                self.trait_registry.insert(decl.name.clone(), decl.clone());
            }
        }

        // WINDJAMMER FIX: Use the SHARED analyzer for cross-file trait analysis
        // This ensures trait methods analyzed in file 1 are available when analyzing impl in file 2

        // Update analyzer's Copy structs registry (in case new Copy structs were discovered)
        self.analyzer
            .update_copy_structs(self.copy_structs_registry.clone());
        // Provide cross-file struct field types for nested field chain resolution
        self.analyzer
            .set_global_struct_field_types(self.global_struct_field_types.clone());

        // Register any newly discovered traits into the analyzer
        for trait_decl in self.trait_registry.values() {
            let dummy_program = parser::Program {
                items: vec![parser::Item::Trait {
                    decl: trait_decl.clone(),
                    location: parser::SourceLocation::default(),
                }],
            };
            self.analyzer
                .register_traits_from_program(&dummy_program)
                .map_err(|e| anyhow::anyhow!("register_traits_from_program: {}", e))?;
        }

        // THE WINDJAMMER WAY: Store this program for cross-file trait inference
        self.all_programs.push(program.clone());

        let (analyzed, signatures, analyzed_trait_methods) = self
            .analyzer
            .analyze_program_with_global_signatures(&program, &self.global_signatures)
            .map_err(|e| anyhow::anyhow!("Analysis error: {}", e))?;

        // EXPLICIT MUTABILITY: Enforce immutable-by-default `let` semantics
        // Users must write `let mut x` when mutation is intended (like Rust/Swift/Kotlin)
        // This prevents accidental state mutation bugs
        let mut mut_checker = errors::MutabilityChecker::new(file_path.clone());
        let mut has_mut_errors = false;
        for item in &program.items {
            match item {
                parser::Item::Function { decl, .. } => {
                    let mut_errors = mut_checker.check_function(decl);
                    if !mut_errors.is_empty() {
                        has_mut_errors = true;
                        for error in &mut_errors {
                            eprintln!("{}", error.format_error());
                        }
                    }
                }
                parser::Item::Impl { block, .. } => {
                    for func_decl in &block.functions {
                        let mut_errors = mut_checker.check_function(func_decl);
                        if !mut_errors.is_empty() {
                            has_mut_errors = true;
                            for error in &mut_errors {
                                eprintln!("{}", error.format_error());
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        if has_mut_errors {
            anyhow::bail!(
                "Compilation failed: mutability errors detected in module '{}'",
                module_path
            );
        }

        // WINDJAMMER PHILOSOPHY: Expression-level float type inference
        // Run constraint-based type inference BEFORE codegen to prevent f32/f64 mixing
        let mut float_inference = type_inference::FloatInference::new();
        float_inference.set_global_struct_field_types(&self.global_struct_field_types);
        float_inference.set_debug_source(&source);
        float_inference.infer_program(&program);

        if !float_inference.errors.is_empty() {
            eprintln!(
                "🚨 Float type inference errors in module {} (file {:?}):",
                module_path, file_path
            );
            for error in &float_inference.errors {
                eprintln!("  {}", error);
            }
            return Err(anyhow::anyhow!(
                "Type inference failed with {} error(s)",
                float_inference.errors.len()
            ));
        }

        // MODULE-SCOPED SIGNATURE RESOLUTION:
        // Create a per-file registry that starts with global signatures (for cross-module lookups)
        // then overlays per-file signatures (for local type priority).
        // This prevents name collisions when two modules define types with the same name
        // (e.g., narrative::Quest::new vs quest::Quest::new).
        let mut per_file_registry = self.global_signatures.clone();
        per_file_registry.merge(&signatures);

        let mut generator = codegen::CodeGenerator::new_for_module(per_file_registry, self.target);
        generator.set_float_inference(float_inference);
        generator.set_analyzed_trait_methods(analyzed_trait_methods);
        // CROSS-MODULE STRUCT FIELD TYPES: Pre-populate for type inference on imported structs
        generator.set_global_struct_field_types(self.global_struct_field_types.clone());
        // USER-DEFINED COPY TYPES: Enable Copy detection for @derive(Copy) structs/enums
        generator.set_copy_types_registry(self.copy_structs_registry.clone());
        let rust_code = generator.generate_program(&program, &analyzed);

        // THEN merge into global for future files' cross-module lookups
        self.global_signatures.merge(&signatures);

        // Extract module name from path
        // For "std::json" -> "json"
        // For "./utils" -> "utils"
        let module_name = if module_path.starts_with("std::") {
            module_path.strip_prefix("std::").unwrap().to_string()
        } else {
            // For relative paths, use the last component
            module_path
                .trim_start_matches("./")
                .trim_start_matches("../")
                .split('/')
                .next_back()
                .unwrap_or(module_path)
                .to_string()
        };

        // Track stdlib imports for Cargo.toml generation
        if module_path.starts_with("std::") {
            self.imported_stdlib_modules.insert(module_name.clone());
        }

        // Wrap in pub mod
        let wrapped = format!("pub mod {} {{\n{}\n}}\n", module_name, rust_code);

        self.compiled_modules
            .insert(module_path.to_string(), wrapped);
        Ok(())
    }

    fn resolve_module_path(
        &self,
        module_path: &str,
        source_file: Option<&Path>,
    ) -> Result<PathBuf> {
        if module_path.starts_with("std::") {
            // Stdlib module: std::json -> ./std/json.wj
            let module_name = module_path.strip_prefix("std::").unwrap();
            let mut path = self.stdlib_path.clone();
            path.push(format!("{}.wj", module_name));

            if !path.exists() {
                return Err(anyhow::anyhow!(
                    "Stdlib module not found: {} (looked in {:?})",
                    module_path,
                    path
                ));
            }

            Ok(path)
        } else if module_path.starts_with("./") || module_path.starts_with("../") {
            // Relative import: ./utils -> ./utils.wj or ./utils/mod.wj
            let source_dir = source_file.and_then(|f| f.parent()).ok_or_else(|| {
                anyhow::anyhow!("Cannot resolve relative import without source file")
            })?;

            // Strip ./ or ../
            let rel_path = module_path
                .trim_start_matches("./")
                .trim_start_matches("../");
            let mut candidate = source_dir.to_path_buf();

            // Handle ../ by going up directories
            if module_path.starts_with("../") {
                candidate = candidate
                    .parent()
                    .ok_or_else(|| anyhow::anyhow!("Cannot go above root directory"))?
                    .to_path_buf();
            }

            // Try direct file first: utils.wj
            candidate.push(format!("{}.wj", rel_path));
            if candidate.exists() {
                return Ok(candidate);
            }

            // Try directory module: utils/mod.wj
            candidate.pop();
            candidate.push(rel_path);
            candidate.push("mod.wj");
            if candidate.exists() {
                return Ok(candidate);
            }

            Err(anyhow::anyhow!(
                "User module not found: {} (looked in {:?} and {:?})",
                module_path,
                source_dir.join(format!("{}.wj", rel_path)),
                source_dir.join(rel_path).join("mod.wj")
            ))
        } else {
            // Absolute module path (e.g., math, rendering, physics)

            // THE WINDJAMMER WAY: Check the current file's directory FIRST
            // This allows "use texture_atlas::Foo" to work when texture_atlas.wj
            // is in the same directory as the importing file
            // We check this BEFORE source_roots to prioritize same-directory imports
            if let Some(source_file) = source_file {
                if let Some(source_dir) = source_file.parent() {
                    // Try direct file in same directory: source_dir/texture_atlas.wj
                    let mut candidate = source_dir.to_path_buf();
                    candidate.push(format!("{}.wj", module_path));
                    if candidate.exists() {
                        // Found in same directory as source file!
                        // Return the real path to compile it alongside
                        return Ok(candidate);
                    }

                    // Try directory module in same directory: source_dir/texture_atlas/mod.wj
                    let mut candidate = source_dir.to_path_buf();
                    candidate.push(module_path);
                    candidate.push("mod.wj");
                    if candidate.exists() {
                        return Ok(candidate);
                    }
                }
            }

            // Check if this exists in any of the configured source roots
            for source_root in &self.source_roots {
                // Try direct file: source_root/math.wj
                let mut candidate = source_root.clone();
                candidate.push(format!("{}.wj", module_path));
                if candidate.exists() {
                    // Found in source root - treat as external module
                    // When compiling individual files from source roots, cross-module
                    // dependencies should be treated as external (will be in same Rust module)
                    return Ok(PathBuf::from(format!("__source_root__::{}", module_path)));
                }

                // Try directory module: source_root/math/mod.wj
                let mut candidate = source_root.clone();
                candidate.push(module_path);
                candidate.push("mod.wj");
                if candidate.exists() {
                    // Found in source root - treat as external module
                    return Ok(PathBuf::from(format!("__source_root__::{}", module_path)));
                }
            }

            // Not found in source roots or current directory - treat as external crate
            // External crate imports (e.g., windjammer_ui, external_crate)
            // These are treated as Rust crate dependencies and passed through to generated code
            // Mark as external by returning a special "external" path
            Ok(PathBuf::from(format!("__external__::{}", module_path)))
        }
    }

    fn get_compiled_modules(&self) -> Vec<String> {
        // Return modules in arbitrary order (should topologically sort in future)
        self.compiled_modules.values().cloned().collect()
    }

    fn get_cargo_dependencies(&self) -> Vec<String> {
        // Map stdlib module names to their Rust crate dependencies
        let mut deps = Vec::new();

        for module in &self.imported_stdlib_modules {
            match module.as_str() {
                "json" => {
                    deps.push("serde = { version = \"1.0\", features = [\"derive\"] }".to_string());
                    deps.push("serde_json = \"1.0\"".to_string());
                }
                "csv" => {
                    deps.push("csv = \"1.3\"".to_string());
                }
                "http" => {
                    // HTTP client (reqwest)
                    deps.push(
                        "reqwest = { version = \"0.11\", features = [\"json\"] }".to_string(),
                    );
                    // HTTP server (axum)
                    deps.push("axum = \"0.7\"".to_string());
                    deps.push("tokio = { version = \"1\", features = [\"full\"] }".to_string());
                }
                "time" => {
                    deps.push("chrono = \"0.4\"".to_string());
                }
                "log" => {
                    deps.push("log = \"0.4\"".to_string());
                    deps.push("env_logger = \"0.11\"".to_string());
                }
                "regex" => {
                    deps.push("regex = \"1.10\"".to_string());
                }
                "cli" => {
                    deps.push("clap = { version = \"4.5\", features = [\"derive\"] }".to_string());
                }
                "db" => {
                    deps.push("sqlx = { version = \"0.7\", features = [\"runtime-tokio-native-tls\", \"postgres\", \"sqlite\", \"mysql\"] }".to_string());
                    deps.push("tokio = { version = \"1\", features = [\"full\"] }".to_string());
                }
                "random" => {
                    deps.push("rand = \"0.8\"".to_string());
                }
                "crypto" => {
                    deps.push("sha2 = \"0.10\"".to_string());
                    deps.push("bcrypt = \"0.15\"".to_string());
                    deps.push("base64 = \"0.21\"".to_string());
                }
                "process" => {
                    // Uses std::process, no extra deps
                }
                "env" => {
                    // Uses std::env, no extra deps
                }
                "async" => {
                    deps.push("tokio = { version = \"1\", features = [\"full\"] }".to_string());
                }
                // fs, strings, math use std library (no extra deps)
                _ => {}
            }
        }

        deps.sort();
        deps.dedup();
        deps
    }
}


pub fn compile_file(
    input_path: &Path,
    output_dir: &Path,
    target: CompilationTarget,
) -> Result<(HashSet<String>, Vec<String>)> {
    let mut module_compiler = ModuleCompiler::new(target, true);
    // For single-file compilation, use parent directory as source root
    let source_root = input_path.parent().unwrap_or(Path::new("."));
    let is_multi_file = false; // Single file compilation
    compile_file_with_compiler(
        source_root,
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
    let mut parser = parser::Parser::new_with_source(
        tokens,
        input_path.to_string_lossy().to_string(),
        source.clone(),
    );
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
    crate::metadata::merge_wj_meta_signatures_from_dir(
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
    module_compiler._parsers.push(parser);
    // Note: parser has been moved and can't be used after this

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

    // EXPLICIT MUTABILITY: Check for mut errors with helpful error messages
    // Checks both top-level functions AND methods inside impl blocks
    let mut mut_checker = errors::MutabilityChecker::new(input_path.to_path_buf());
    let mut has_mut_errors = false;
    for item in &program.items {
        match item {
            parser::Item::Function { decl, .. } => {
                let mut_errors = mut_checker.check_function(decl);
                if !mut_errors.is_empty() {
                    has_mut_errors = true;
                    for error in &mut_errors {
                        eprintln!("{}", error.format_error());
                    }
                }
            }
            parser::Item::Impl { block, .. } => {
                for func_decl in &block.functions {
                    let mut_errors = mut_checker.check_function(func_decl);
                    if !mut_errors.is_empty() {
                        has_mut_errors = true;
                        for error in &mut_errors {
                            eprintln!("{}", error.format_error());
                        }
                    }
                }
            }
            _ => {}
        }
    }

    if has_mut_errors {
        anyhow::bail!("Compilation failed: mutability errors detected");
    }

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

    // Generate code for main file
    let rust_code = if target == CompilationTarget::Go {
        // Go backend: use the new Go codegen
        use codegen::backend::{CodegenConfig, Target};
        let config = CodegenConfig {
            target: Target::Go,
            output_dir: output_dir.to_path_buf(),
            ..Default::default()
        };

        let output = codegen::generate(&program, Target::Go, Some(config))
            .map_err(|e| anyhow::anyhow!("Go codegen error: {}", e))?;

        // Write main.go
        let output_file = output_dir.join("main.go");
        std::fs::write(&output_file, &output.source)?;

        // Write additional files (go.mod)
        for (filename, content) in &output.additional_files {
            let file_path = output_dir.join(filename);
            std::fs::write(file_path, content)?;
        }

        return Ok((HashSet::new(), Vec::new()));
    } else if target == CompilationTarget::Wasm {
        // Check if program has components
        use component_analyzer::ComponentAnalyzer;
        let mut comp_analyzer = ComponentAnalyzer::new();
        let has_components = comp_analyzer.analyze(&program.items).is_ok()
            && comp_analyzer.all_components().next().is_some();

        if has_components {
            // Use new backend system for component-based WASM
            use codegen::backend::{CodegenConfig, Target};
            let config = CodegenConfig {
                target: Target::WebAssembly,
                output_dir: output_dir.to_path_buf(),
                ..Default::default()
            };

            let output = codegen::generate(&program, Target::WebAssembly, Some(config))
                .map_err(|e| anyhow::anyhow!("Component codegen error: {}", e))?;

            // Write main.rs
            let output_file = output_dir.join("lib.rs"); // WASM uses lib.rs
            std::fs::write(output_file, &output.source)?;

            // Write additional files (Cargo.toml, index.html)
            for (filename, content) in &output.additional_files {
                let file_path = output_dir.join(filename);
                std::fs::write(file_path, content)?;
            }

            // Return empty to signal we've handled everything
            return Ok((HashSet::new(), Vec::new()));
        } else {
            // WINDJAMMER PHILOSOPHY: Expression-level float type inference (WASM target)
            let mut float_inference = type_inference::FloatInference::new();
            float_inference.set_source_root(source_root);
            float_inference
                .set_global_struct_field_types(&module_compiler.global_struct_field_types);
            float_inference.set_debug_source(&source);
            float_inference.infer_program(&program);

            if !float_inference.errors.is_empty() {
                eprintln!(
                    "🚨 Float type inference errors in {}:",
                    input_path.display()
                );
                for error in &float_inference.errors {
                    eprintln!("  {}", error);
                }
                return Err(anyhow::anyhow!(
                    "Type inference failed with {} error(s)",
                    float_inference.errors.len()
                ));
            }

            // Use old generator for non-component WASM
            // MODULE-SCOPED SIGNATURE RESOLUTION:
            // Always start with global signatures (for cross-module lookups),
            // then overlay per-file signatures (for local type priority).
            // This prevents name collisions when two modules define types with the same name.
            let mut generator_signatures = module_compiler.global_signatures.clone();
            generator_signatures.merge(&signatures);

            let mut generator = if is_multi_file_project {
                codegen::CodeGenerator::new_for_module(generator_signatures, target)
            } else {
                codegen::CodeGenerator::new(generator_signatures, target)
            };
            generator.set_float_inference(float_inference);
            generator.set_inferred_bounds(inferred_bounds_map);
            generator.set_analyzed_trait_methods(analyzed_trait_methods);
            // CROSS-MODULE STRUCT FIELD TYPES: Pre-populate for type inference on imported structs
            generator
                .set_global_struct_field_types(module_compiler.global_struct_field_types.clone());
            // USER-DEFINED COPY TYPES: Enable Copy detection for @derive(Copy) structs/enums
            generator.set_copy_types_registry(module_compiler.copy_structs_registry.clone());

            // Set source file for error mapping
            generator.set_source_file(input_path);
            let output_file_path =
                project_paths::get_relative_output_path(source_root, input_path, output_dir)?;
            // Create parent directories if needed
            if let Some(parent) = output_file_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            generator.set_output_file(&output_file_path);

            // Set workspace root for relative paths in source maps
            // Use the current working directory as the workspace root for portability
            // This ensures both source and output paths can be relative
            if let Ok(cwd) = std::env::current_dir() {
                generator.set_workspace_root(cwd);
            }

            let result = generator.generate_program(&program, &analyzed);

            // Save source map for error mapping (now with relative paths)
            let source_map_path = output_file_path.with_extension("rs.map");
            if let Err(e) = generator.get_source_map().save_to_file(&source_map_path) {
                eprintln!("Warning: Failed to save source map: {}", e);
            }

            result
        }
    } else {
        // WINDJAMMER PHILOSOPHY: Expression-level float type inference (Rust target)
        // Run constraint-based type inference BEFORE codegen to prevent f32/f64 mixing
        let mut float_inference = type_inference::FloatInference::new();
        float_inference.set_source_root(source_root);
        float_inference.set_global_struct_field_types(&module_compiler.global_struct_field_types);
        float_inference.set_debug_source(&source);
        float_inference.infer_program(&program);

        if !float_inference.errors.is_empty() {
            eprintln!(
                "🚨 Float type inference errors in {}:",
                input_path.display()
            );
            for error in &float_inference.errors {
                eprintln!("  {}", error);
            }
            return Err(anyhow::anyhow!(
                "Type inference failed with {} error(s)",
                float_inference.errors.len()
            ));
        }

        // TDD: Integer literal type inference (i32, i64, u32, etc.)
        let mut int_inference = type_inference::IntInference::new();
        int_inference.set_global_struct_field_types(&module_compiler.global_struct_field_types);
        int_inference.infer_program(&program);
        if !int_inference.errors.is_empty() {
            eprintln!("🚨 Int type inference errors in {}:", input_path.display());
            for error in &int_inference.errors {
                eprintln!("  {}", error);
            }
            return Err(anyhow::anyhow!(
                "Int type inference failed with {} error(s)",
                int_inference.errors.len()
            ));
        }

        // Use old generator for Rust target
        // MODULE-SCOPED SIGNATURE RESOLUTION:
        // Always start with global signatures (for cross-module lookups),
        // then overlay per-file signatures (for local type priority).
        let mut generator_signatures = module_compiler.global_signatures.clone();
        generator_signatures.merge(&signatures);
        let mut generator = if is_multi_file_project {
            codegen::CodeGenerator::new_for_module(generator_signatures, target)
        } else {
            codegen::CodeGenerator::new(generator_signatures, target)
        };
        generator.set_float_inference(float_inference);
        generator.set_int_inference(int_inference);
        generator.set_inferred_bounds(inferred_bounds_map);
        generator.set_analyzed_trait_methods(analyzed_trait_methods);
        // CROSS-MODULE STRUCT FIELD TYPES: Pre-populate for type inference on imported structs
        generator.set_global_struct_field_types(module_compiler.global_struct_field_types.clone());
        // USER-DEFINED COPY TYPES: Enable Copy detection for @derive(Copy) structs/enums
        generator.set_copy_types_registry(module_compiler.copy_structs_registry.clone());

        // Set source file for error mapping
        generator.set_source_file(input_path);
        let output_file_path =
            project_paths::get_relative_output_path(source_root, input_path, output_dir)?;
        // Create parent directories if needed
        if let Some(parent) = output_file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        generator.set_output_file(&output_file_path);

        // Set workspace root for relative paths in source maps
        // Use the current working directory as the workspace root for portability
        // This ensures both source and output paths can be relative
        if let Ok(cwd) = std::env::current_dir() {
            generator.set_workspace_root(cwd);
        }

        let result = generator.generate_program(&program, &analyzed);

        // Save source map for error mapping (now with relative paths)
        let source_map_path = output_file_path.with_extension("rs.map");
        if let Err(e) = generator.get_source_map().save_to_file(&source_map_path) {
            eprintln!("Warning: Failed to save source map: {}", e);
        }

        result
    };

    // THE WINDJAMMER WAY: Don't inline modules in multi-file projects!
    // The module system (lib.rs + mod.rs) handles module structure.
    // Only inline modules for single-file compilation (legacy behavior).
    //
    // BUGFIX: Also don't inline if the program declares any modules (pub mod foo;)
    // This handles the case where a single mod.wj file has submodules that
    // should be compiled as separate files, not inlined.
    let has_module_declarations = program
        .items
        .iter()
        .any(|item| matches!(item, parser::Item::Mod { items, .. } if items.is_empty()));

    let combined_code = if is_multi_file_project || has_module_declarations {
        // Multi-file project OR module with submodules: Don't inline modules
        // The module system handles everything via lib.rs/mod.rs
        rust_code
    } else {
        // Single-file with no module declarations: Inline compiled modules (legacy)
        let module_code = module_compiler.get_compiled_modules().join("\n");
        if module_code.is_empty() {
            rust_code
        } else {
            format!("{}\n\n{}", module_code, rust_code)
        }
    };

    // Write output (preserving directory structure)
    let output_file = project_paths::get_relative_output_path(source_root, input_path, output_dir)?;

    // Bug #2B FIX: Prevent lib.rs generation in subdirectories
    // --------------------------------------------------------
    // lib.rs should only exist at the crate root, not in subdirectories like src/components/generated/.
    // If we're compiling lib.wj and the output directory is a subdirectory (contains ".../src/..."),
    // skip writing lib.rs entirely.
    //
    // Detection heuristic (matches generate_nested_module_structure):
    // - If output path contains ".../src/..." with more components after src, it's a subdirectory
    let is_lib_file = input_path
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s == "lib.wj")
        .unwrap_or(false);

    let is_output_subdirectory = {
        let components: Vec<_> = output_dir.components().collect();
        let mut found_src = false;
        for (i, component) in components.iter().enumerate() {
            if let std::path::Component::Normal(name) = component {
                if name.to_string_lossy() == "src" && i + 1 < components.len() {
                    found_src = true;
                    break;
                }
            }
        }
        found_src
    };

    if is_lib_file && is_output_subdirectory {
        // Skip writing lib.rs in subdirectories
        eprintln!(
            "⏭️  SKIPPING lib.rs generation in subdirectory: {}",
            output_dir.display()
        );
        eprintln!("   lib.rs should only exist at crate root, not in subdirectories");
        // Return empty dependencies and traits (nothing was written)
        return Ok((HashSet::new(), Vec::new()));
    }

    // Create parent directories if needed
    if let Some(parent) = output_file.parent() {
        std::fs::create_dir_all(parent)?;
    }

    eprintln!(
        "📝 WRITING FILE: {} ({} bytes)",
        output_file.display(),
        combined_code.len()
    );
    if combined_code.is_empty() {
        eprintln!("    🚨 WARNING: Writing EMPTY file!");
        eprintln!("    rust_code length: check generator output");
    }

    // Write file with explicit control over flush and sync
    // This ensures the write completes before we return, preventing race conditions
    // where tests read the file before the OS has flushed buffers
    {
        use std::io::Write;
        let mut file = std::fs::File::create(&output_file)?;
        file.write_all(combined_code.as_bytes())?;
        file.flush()?; // Ensure data is written to OS buffers

        // On Linux, also force OS to write buffers to disk (Ubuntu CI has aggressive caching)
        // On macOS/Windows, flush() is sufficient
        #[cfg(target_os = "linux")]
        {
            file.sync_all()?; // Sync file data AND metadata
            drop(file); // Close file handle before syncing directory

            // CRITICAL: On Linux, we must also sync the PARENT DIRECTORY
            // to ensure the directory entry is persisted. Without this,
            // a crash could leave the directory without the file entry.
            // Ubuntu CI appears to have very aggressive caching.
            if let Some(parent) = output_file.parent() {
                let dir = std::fs::File::open(parent)?;
                dir.sync_all()?;
            }
        }

        #[cfg(not(target_os = "linux"))]
        drop(file); // Close file handle on non-Linux systems
    }

    // TDD FIX: Emit metadata file for cross-module type inference
    if target == CompilationTarget::Rust {
        use crate::metadata::{metadata_function_sig_from_analyzer, ModuleMetadata};

        let module_path = input_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        let mut meta = ModuleMetadata::new(module_path.to_string());

        // Function signatures: use analyzer registry (includes inferred param ownership)
        for item in &program.items {
            match item {
                parser::Item::Function { decl, .. } => {
                    if let Some(sig) = signatures.get_signature(&decl.name) {
                        meta.functions.insert(
                            decl.name.clone(),
                            metadata_function_sig_from_analyzer(sig, false, None),
                        );
                    }
                }
                parser::Item::Impl { block, .. } => {
                    let type_name = &block.type_name;
                    for func_decl in &block.functions {
                        let full_name = format!("{}::{}", type_name, func_decl.name);
                        if let Some(sig) = signatures.get_signature(&full_name) {
                            meta.functions.insert(
                                full_name,
                                metadata_function_sig_from_analyzer(
                                    sig,
                                    true,
                                    Some(type_name.clone()),
                                ),
                            );
                        }
                    }
                }
                parser::Item::Struct { decl, .. } => {
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

        let meta_path = crate::metadata::meta_cache_path(input_path);
        if let Some(parent) = meta_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let meta_json = serde_json::to_string_pretty(&meta)?;
        std::fs::write(&meta_path, &meta_json)?;

        eprintln!(
            "📋 METADATA: {} ({} functions, {} structs)",
            meta_path.display(),
            meta.functions.len(),
            meta.structs.len()
        );
    }

    // Return the set of imported stdlib modules and external crates for Cargo.toml generation
    Ok((
        module_compiler.imported_stdlib_modules.clone(),
        module_compiler.external_crates.clone(),
    ))
}

