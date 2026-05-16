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
use crate::{analyzer, codegen, type_inference};
use crate::{lexer, parser, parser_impl, CompilationTarget};
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

    pub(crate) fn compile_module(
        &mut self,
        module_path: &str,
        source_file: Option<&Path>,
    ) -> Result<()> {
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

        crate::compilation_error_handling::check_module_mutability(
            module_path,
            &file_path,
            &program,
        )?;

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

    pub(crate) fn get_compiled_modules(&self) -> Vec<String> {
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

pub use crate::file_compilation_pipeline::{compile_file, compile_file_with_compiler};
