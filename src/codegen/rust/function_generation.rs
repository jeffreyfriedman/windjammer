//! Function Generation Module
//!
//! Handles generation of Rust code for function declarations, including:
//! - Regular functions and methods
//! - Extern/FFI function declarations
//! - Functions with decorator wrapping (timeout, bench, requires, ensures, etc.)
//! - Parameterized tests (@test_cases)
//! - Self parameter inference and builder pattern detection

use crate::analyzer::*;
use crate::codegen::rust::{codegen_helpers, self_analysis, type_analysis};
use crate::parser::*;

use super::CodeGenerator;

impl<'ast> CodeGenerator<'ast> {
    pub(crate) fn generate_function(&mut self, analyzed: &AnalyzedFunction<'ast>) -> String {
        let func = &analyzed.decl;

        // PARAMETERIZED TESTS: Check for @test_cases decorator
        // If present, generate multiple test functions instead of one
        if let Some(test_cases_decorator) = func.decorators.iter().find(|d| d.name == "test_cases")
        {
            return self.generate_parameterized_tests(analyzed, test_cases_decorator);
        }

        // TESTING DECORATORS: Check for decorators that need to wrap the function body
        // These include: @timeout, @bench, @requires, @ensures, @property_test, @test(setup/teardown)
        if self.has_wrapping_decorator(func) {
            return self.generate_function_with_wrapping(analyzed);
        }

        let mut output = String::new();

        // TDD FIX: Auto-add #[test] attribute for test functions in test files
        // THE WINDJAMMER WAY: Test files (*_test.wj) should auto-generate test attributes
        // Bug: Tests don't run because #[test] attributes are missing
        // Root Cause: Codegen doesn't detect test files and test functions
        // Fix: Check if filename ends with _test.wj AND function starts with test_
        let filename_str = self.current_wj_file.to_string_lossy();
        let is_test_file = filename_str.ends_with("_test.wj") || filename_str.contains("_test.wj");
        let is_test_function = func.name.starts_with("test_");
        let has_test_decorator = func.decorators.iter().any(|d| d.name == "test");
        let has_property_test = func.decorators.iter().any(|d| d.name == "property_test");

        if is_test_file && is_test_function && !has_test_decorator && !has_property_test {
            output.push_str("#[test]\n");
        }

        // LOCAL VARIABLE TRACKING: Push new scope for this function
        self.local_variable_scopes
            .push(std::collections::HashSet::new());

        // AUTO-CLONE: Load auto-clone analysis for this function
        self.auto_clone_analysis = Some(analyzed.auto_clone_analysis.clone());
        self.auto_clone_counter = 0;

        // PHASE 2 OPTIMIZATION: Load clone optimizations for this function
        // Variables in this set can safely avoid .clone() calls
        self.clone_optimizations.clear();
        for opt in &analyzed.clone_optimizations {
            self.clone_optimizations.insert(opt.variable.clone());
        }

        self.current_function_params = func.parameters.clone();
        // Combine inline bounds (<T: Foo>) and where clause for trait resolution
        let mut all_bounds: Vec<(String, Vec<String>)> = func
            .type_params
            .iter()
            .filter(|tp| !tp.bounds.is_empty())
            .map(|tp| (tp.name.clone(), tp.bounds.clone()))
            .collect();
        for (name, bounds) in &func.where_clause {
            if let Some(existing) = all_bounds.iter_mut().find(|(n, _)| n == name) {
                existing.1.extend(bounds.iter().cloned());
            } else {
                all_bounds.push((name.clone(), bounds.clone()));
            }
        }
        self.current_function_type_bounds = all_bounds;

        // Clear local variable types for new function scope
        self.local_var_types.clear();
        self.borrowed_iterator_vars.clear();

        // Track function return type for string literal conversion
        self.current_function_return_type = func.return_type.clone();

        // Track method return types for usize inference in comparisons
        // When in an impl block, record the return type so expression_produces_usize
        // can resolve method calls like animation.frame_count() → usize
        if self.in_impl_block {
            if let Some(ref ret_type) = func.return_type {
                self.method_return_types
                    .insert(func.name.to_string(), ret_type.clone());
            }

            // NEW ARCHITECTURE: Register method signature for type-based parameter resolution
            // This replaces ALL hard-coded method name heuristics
            if let Some(impl_type) = &self.current_struct_name {
                // Build parameter types and ownership from ANALYZED function
                // Use the actual inferred ownership from the analyzer, not defaults!
                let mut param_types = Vec::new();
                let mut param_ownership = Vec::new();

                // Parameter types for call-site coercion must match analyzer + Rust codegen.
                // Phase-2 optimized `string` parameters become `Reference(str)` in
                // `AnalyzedFunction::inferred_param_types`, but AST param.type_ stays plain
                // `string`. Registering AST types breaks MethodCallAnalyzer's user-signatures path
                // (wrong `param_is_str_ref`, missing `&` on String fields, spurious `.to_string()`).
                for (idx, param) in func.parameters.iter().enumerate() {
                    if param.name != "self" {
                        let p_type = analyzed
                            .inferred_param_types
                            .get(idx)
                            .cloned()
                            .unwrap_or_else(|| param.type_.clone());
                        param_types.push(p_type);

                        // Use ACTUAL analyzed ownership from inferred_ownership
                        let ownership = analyzed
                            .inferred_ownership
                            .get(&param.name)
                            .copied()
                            .unwrap_or(crate::analyzer::OwnershipMode::Borrowed);
                        param_ownership.push(ownership);
                    }
                }

                // Check if method has self receiver
                let has_self_receiver = func.parameters.iter().any(|p| p.name == "self");

                let signature = crate::codegen::rust::generator::MethodSignature::new(
                    impl_type.clone(),
                    func.name.clone(),
                    param_types,
                    param_ownership,
                    func.return_type.clone(),
                    has_self_receiver,
                );

                self.register_method_signature(signature);
            }
        }

        // Track function body for data flow analysis
        self.current_function_body = func.body.clone();

        // FOR-LOOP AUTO-BORROW: Pre-scan function body to find local variables
        // that are iterated in for-loops and also used after the loop.
        // These need `&` auto-inserted to prevent consuming the collection.
        self.precompute_for_loop_borrows(&func.body);

        // Track parameters inferred as borrowed/mut-borrowed for codegen decisions
        self.inferred_borrowed_params.clear();
        self.inferred_mut_borrowed_params.clear();
        self.str_ref_optimized_params.clear();
        for (param_name, ownership) in &analyzed.inferred_ownership {
            match ownership {
                crate::analyzer::OwnershipMode::Borrowed => {
                    self.inferred_borrowed_params.insert(param_name.clone());
                }
                crate::analyzer::OwnershipMode::MutBorrowed => {
                    self.inferred_mut_borrowed_params.insert(param_name.clone());
                }
                _ => {}
            }
        }

        // Track Phase 2 string-optimized parameters (string type params that become &str)
        for param_name in &analyzed.str_ref_optimizable_params {
            self.str_ref_optimized_params.insert(param_name.clone());
        }

        // Track explicit &String/&string params that become &str via type_to_rust
        // (Type::Reference(String) → "&str"). These aren't Phase 2 optimized but still
        // need .to_string() conversions in the body (e.g., Some(s) → Some(s.to_string())).
        for param in &func.parameters {
            if matches!(&param.type_, Type::Reference(inner)
                if matches!(&**inner, Type::String)
                    || matches!(&**inner, Type::Custom(ref n) if n == "string" || n == "String"))
            {
                self.str_ref_optimized_params.insert(param.name.clone());
            }
        }

        // METHOD PARAM OWNERSHIP: Register this method's parameter ownership modes
        // for use at call sites (auto-borrow arguments).
        {
            let ownership_vec: Vec<(String, crate::analyzer::OwnershipMode)> = analyzed
                .inferred_ownership
                .iter()
                .filter(|(name, _)| name.as_str() != "self")
                .map(|(name, mode)| (name.clone(), *mode))
                .collect();
            if !ownership_vec.is_empty() {
                self.method_param_ownership
                    .insert(func.name.to_string(), ownership_vec);
            }
        }

        // WINDJAMMER FIX: Track usize-typed parameters for auto-cast logic
        // DON'T clear here - we need to accumulate variables from let statements during generation!
        // Only clear at the very beginning of function generation, before body processing.
        // TDD FIX (Bug #3): Moved clear to happen BEFORE pre-passes, so marking during
        // statement generation can accumulate variables.

        // Clear ONCE at function start (before any analysis)
        self.usize_variables.clear();

        // When a parameter is declared as `usize`, add it to usize_variables
        // so expression_produces_usize() correctly identifies it
        for (param_idx, param) in func.parameters.iter().enumerate() {
            // Use inferred type if available, otherwise use declared type
            let param_type = analyzed
                .inferred_param_types
                .get(param_idx)
                .unwrap_or(&param.type_);

            // Check if this parameter is usize
            if matches!(param_type, Type::Custom(name) if name == "usize") {
                self.usize_variables.insert(param.name.clone());
            }
        }

        // PHASE 8 OPTIMIZATION: Load SmallVec optimizations for this function
        // DISABLED: SmallVec optimizations conflict with return types
        // TODO: Re-enable with smarter conversion at return sites
        self.smallvec_optimizations.clear();
        // for opt in &analyzed.smallvec_optimizations {
        //     self.smallvec_optimizations
        //         .insert(opt.variable.clone(), opt.clone());
        //     self.needs_smallvec_import = true; // Mark that we need the smallvec crate
        // }

        // PHASE 9 OPTIMIZATION: Load Cow optimizations for this function
        self.cow_optimizations.clear();
        for opt in &analyzed.cow_optimizations {
            self.cow_optimizations.insert(opt.variable.clone());
            self.needs_cow_import = true; // Mark that we need Cow from std::borrow
        }

        // PHASE 3 OPTIMIZATION: Load struct mapping optimizations
        // Track which structs can use optimized construction strategies
        self.struct_mapping_hints.clear();
        for opt in &analyzed.struct_mapping_optimizations {
            self.struct_mapping_hints
                .insert(opt.target_struct.clone(), opt.strategy.clone());
        }

        // PHASE 4 OPTIMIZATION: Load string operation optimizations
        // Track capacity hints for string operations
        self.string_capacity_hints.clear();

        // PHASE 5 OPTIMIZATION: Load assignment operation optimizations
        // Track which variables can use compound assignment operators
        self.assignment_optimizations.clear();
        for opt in &analyzed.assignment_optimizations {
            self.assignment_optimizations
                .insert(opt.variable.clone(), opt.operation.clone());
        }
        for opt in &analyzed.string_optimizations {
            if let Some(capacity) = opt.estimated_capacity {
                self.string_capacity_hints.insert(opt.location, capacity);
            }
        }

        // PHASE 6 OPTIMIZATION: Load defer drop optimizations
        // Track variables that should have their drops deferred to background thread
        self.defer_drop_optimizations = analyzed.defer_drop_optimizations.clone();

        // Generate doc comment if present
        if let Some(doc_comment) = &func.doc_comment {
            for line in doc_comment.lines() {
                output.push_str(&format!("/// {}\n", line.trim()));
            }
        }

        // Check for @async decorator (special case: it's a keyword, not an attribute)
        let is_async = func.decorators.iter().any(|d| d.name == "async");

        // Special case: async main requires #[tokio::main]
        if is_async && func.name == "main" {
            output.push_str("#[tokio::main]\n");
        }

        // OPTIMIZATION: Add inline hints for hot path functions
        // This is Phase 1 optimization: Generate Inlinable Code
        if self.should_inline_function(func, analyzed) {
            output.push_str("#[inline]\n");
        }

        // Generate decorators (map Windjammer decorators to Rust attributes)
        let decorator_reg2 = crate::decorator_registry::DecoratorRegistry::new();
        for decorator in &func.decorators {
            if decorator_reg2.should_skip_for_backend(&decorator.name, self.target) {
                continue;
            }

            // Map Windjammer decorator to Rust attribute (same as struct decorator handling)
            let rust_attr = self.map_decorator(&decorator.name);
            if decorator.arguments.is_empty() {
                output.push_str(&format!("#[{}]\n", rust_attr));
            } else {
                output.push_str(&format!("#[{}(", rust_attr));
                let args: Vec<String> = decorator
                    .arguments
                    .iter()
                    .map(|(key, expr)| {
                        format!("{} = {}", key, self.generate_expression_immut(expr))
                    })
                    .collect();
                output.push_str(&args.join(", "));
                output.push_str(")]\n");
            }
        }

        // Add `pub` if function is marked pub OR we're in a #[wasm_bindgen] impl block OR compiling a module OR has @export decorator
        // BUT NOT if we're in a trait implementation (trait methods cannot have visibility modifiers)
        let has_export = func.decorators.iter().any(|d| d.name == "export");
        if !self.in_trait_impl
            && (func.is_pub || self.in_wasm_bindgen_impl || self.is_module || has_export)
        {
            output.push_str("pub ");
        }

        // Add async keyword if decorator present
        if is_async {
            output.push_str("async ");
        }

        output.push_str("fn ");
        output.push_str(&func.name);

        // WINDJAMMER LIFETIME INFERENCE: Determine if explicit lifetime annotations are needed.
        // Rust's lifetime elision rules handle most cases automatically:
        //   1. Single input reference → output gets that lifetime
        //   2. &self/&mut self → output gets self's lifetime
        //   3. Multiple input references with no self → MUST be explicit
        // We only add 'a when case 3 applies AND the return type contains references.
        let needs_lifetime = self.function_needs_lifetime_annotations(func, analyzed);

        // Add type parameters with bounds: fn foo<T: Display, U: Debug>(...)
        // Merge inferred bounds with explicit bounds
        let type_params = if let Some(inferred) = self.inferred_bounds.get(&func.name) {
            let merged = inferred.merge_with_explicit(&func.type_params);
            // Track which traits need imports
            for param in &merged {
                for trait_name in &param.bounds {
                    self.needs_trait_imports.insert(trait_name.clone());
                }
            }
            merged
        } else {
            // Still track explicit bounds
            for param in &func.type_params {
                for trait_name in &param.bounds {
                    self.needs_trait_imports.insert(trait_name.clone());
                }
            }
            func.type_params.clone()
        };

        if needs_lifetime || !type_params.is_empty() {
            output.push('<');
            let mut parts = Vec::new();
            if needs_lifetime {
                parts.push("'a".to_string());
            }
            if !type_params.is_empty() {
                parts.push(self.format_type_params(&type_params));
            }
            output.push_str(&parts.join(", "));
            output.push('>');
        }

        output.push('(');

        // Add implicit &self or &mut self for impl block methods that access fields
        // THE WINDJAMMER WAY: Constructors (associated functions) should NOT get self added!
        let mut params: Vec<String> = Vec::new();
        let has_explicit_self = func.parameters.iter().any(|p| p.name == "self");

        // THE WINDJAMMER WAY: Auto-Self Inference
        // Check if analyzer inferred a self parameter (even if not in AST)
        let has_inferred_self = analyzed.inferred_ownership.contains_key("self");

        // Check if this is a constructor (associated function returning the struct type)
        // A constructor returns the struct being implemented, e.g., fn new() -> Tilemap
        let is_constructor = !has_explicit_self && !has_inferred_self && {
            if let Some(Type::Custom(return_type_name)) = &func.return_type {
                // Check if return type matches current struct name
                self.current_struct_name
                    .as_ref()
                    .is_some_and(|struct_name| struct_name == return_type_name)
            } else {
                false
            }
        };

        // Priority 1: Use analyzer's inferred self if available
        // E0053 FIX: For trait impls, use TRAIT's ownership (impl must match trait exactly)
        // Trait impl methods MUST have self if trait has it - even when impl body doesn't use self.
        // The trait codegen adds &mut self by default for ALL trait methods (unless they're
        // associated functions returning Self). So the impl must also have self to match.
        let needs_self_from_trait = self.in_trait_impl
            && !has_explicit_self
            && self
                .current_trait_impl_name
                .as_ref()
                .is_some_and(|trait_name| {
                    let methods = self.analyzed_trait_methods.get(trait_name).or_else(|| {
                        trait_name
                            .rfind("::")
                            .map(|i| &trait_name[i + 2..])
                            .and_then(|key| self.analyzed_trait_methods.get(key))
                    });
                    let found = methods.is_some_and(|m| m.contains_key(&func.name));
                    if !found && !has_inferred_self && !is_constructor {
                        // Cross-file trait impl: trait definition not available in single-file compilation.
                        // Default to requiring self since trait impl methods almost always need it.
                        return true;
                    }
                    found
                });

        if (has_inferred_self || needs_self_from_trait) && !has_explicit_self && !is_constructor {
            let ownership = self.get_effective_self_ownership(&func.name, analyzed).or({
                // Trait has method but no self in analyzed - default to &mut self (trait convention)
                if needs_self_from_trait {
                    Some(OwnershipMode::MutBorrowed)
                } else {
                    None
                }
            });

            if let Some(ownership) = ownership {
                let body_modifies = self.function_modifies_self(&analyzed.decl);
                let returns_self = self.method_returns_impl_struct(&analyzed.decl);
                let self_param = match ownership {
                    OwnershipMode::Borrowed => {
                        if body_modifies {
                            "&mut self"
                        } else {
                            "&self"
                        }
                    }
                    OwnershipMode::MutBorrowed => "&mut self",
                    OwnershipMode::Owned => {
                        if body_modifies && returns_self {
                            "mut self"
                        } else if body_modifies {
                            "&mut self"
                        } else if returns_self {
                            "self"
                        } else {
                            "&self"
                        }
                    }
                };
                // TDD FIX: Sync borrowed-params sets with actual generated receiver.
                // The analyzer may infer Owned but codegen promotes to &self/&mut self.
                // Without this, for-loop borrow detection fails for implicit self methods.
                match self_param {
                    "&self" => {
                        self.inferred_borrowed_params.insert("self".to_string());
                        self.inferred_mut_borrowed_params.remove("self");
                    }
                    "&mut self" => {
                        self.inferred_mut_borrowed_params.insert("self".to_string());
                        self.inferred_borrowed_params.remove("self");
                    }
                    _ => {
                        self.inferred_borrowed_params.remove("self");
                        self.inferred_mut_borrowed_params.remove("self");
                    }
                }
                params.push(self_param.to_string());
            }
        }
        // Priority 2: Fallback to old field-based analysis (for backwards compatibility)
        else if self.in_impl_block
            && !has_explicit_self
            && !self.current_struct_fields.is_empty()
            && !is_constructor
        {
            // Check if function body mutates any struct fields
            let ctx =
                self_analysis::AnalysisContext::new(&func.parameters, &self.current_struct_fields);
            let mutates = self_analysis::function_mutates_fields(&ctx, func);
            let accesses = self_analysis::function_accesses_fields(&ctx, func);
            if mutates {
                // Check if this is a builder pattern (modifies fields AND returns Self)
                let returns_self = self.function_returns_self_type(func);
                if returns_self {
                    params.push("mut self".to_string());
                } else {
                    params.push("&mut self".to_string());
                    self.inferred_mut_borrowed_params.insert("self".to_string());
                }
            } else if accesses {
                params.push("&self".to_string());
                self.inferred_borrowed_params.insert("self".to_string());
            }
        }

        // TDD FIX: Pre-compute which parameters are actually used in the function body.
        // Unused parameters get prefixed with `_` to suppress "unused variable" warnings.
        // THE WINDJAMMER WAY: The compiler handles this automatically — developers don't
        // need to manually prefix unused parameters with `_`.
        let body_refs: Vec<&Statement> = func.body.to_vec();
        let unused_params: std::collections::HashSet<String> = func
            .parameters
            .iter()
            .filter(|p| p.name != "self")
            .filter(|p| !Self::variable_used_in_statements(&body_refs, &p.name))
            .map(|p| p.name.clone())
            .collect();

        // TDD FIX: Pre-compute unused let bindings and for-loop variables.
        // Like unused params, these get prefixed with `_` in the generated Rust.
        self.unused_let_bindings.clear();
        Self::find_unused_bindings(&func.body, &mut self.unused_let_bindings);

        let additional_params: Vec<String> = func
            .parameters
            .iter()
            .enumerate()
            .map(|(param_idx, param)| {
                // SMART STRING INFERENCE: Use the inferred type from analyzer (string → &str vs String)
                let inferred_type = analyzed
                    .inferred_param_types
                    .get(param_idx)
                    .unwrap_or(&param.type_);

                // PHASE 9 OPTIMIZATION: Check if this parameter should use Cow<'_, T>
                if self.cow_optimizations.contains(&param.name) {
                    let base_type = self.type_to_rust(inferred_type);
                    // For String types, use Cow<'_, str>
                    let cow_type = if base_type == "String" {
                        "Cow<'_, str>".to_string()
                    } else {
                        format!("Cow<'_, {}>", base_type)
                    };
                    return format!("{}: {}", param.name, cow_type);
                }

                // Handle explicit ownership hints (self, &self, &mut self)
                let type_str = match &param.ownership {
                    OwnershipHint::Owned => {
                        if param.name == "self" {
                            let body_modifies = self.function_modifies_self(&analyzed.decl);
                            let eff_ownership =
                                self.get_effective_self_ownership(&func.name, analyzed);
                            let self_str = if let Some(ownership_mode) = eff_ownership {
                                match ownership_mode {
                                    OwnershipMode::Borrowed | OwnershipMode::MutBorrowed
                                        if !self.in_trait_impl
                                            && self.method_returns_impl_struct(func) =>
                                    {
                                        "mut self"
                                    }
                                    OwnershipMode::MutBorrowed => "&mut self",
                                    OwnershipMode::Borrowed => {
                                        if !self.in_trait_impl && body_modifies {
                                            "&mut self"
                                        } else {
                                            "&self"
                                        }
                                    }
                                    OwnershipMode::Owned => {
                                        let ret_self = self.method_returns_impl_struct(&analyzed.decl);
                                        if body_modifies && ret_self {
                                            "mut self"
                                        } else if body_modifies {
                                            "&mut self"
                                        } else {
                                            "self"
                                        }
                                    }
                                }
                            } else {
                                let ret_self = self.method_returns_impl_struct(&analyzed.decl);
                                if body_modifies && ret_self {
                                    "mut self"
                                } else if body_modifies {
                                    "&mut self"
                                } else {
                                    "self"
                                }
                            };
                            // Sync borrowed-params sets with actual generated receiver.
                            match self_str {
                                "&self" => {
                                    self.inferred_borrowed_params.insert("self".to_string());
                                    self.inferred_mut_borrowed_params.remove("self");
                                }
                                "&mut self" => {
                                    self.inferred_mut_borrowed_params.insert("self".to_string());
                                    self.inferred_borrowed_params.remove("self");
                                }
                                _ => {
                                    self.inferred_borrowed_params.remove("self");
                                    self.inferred_mut_borrowed_params.remove("self");
                                }
                            }
                            return self_str.to_string();
                        }
                        // Owned parameters are always mutable in Windjammer
                        return format!("mut {}: {}", param.name, self.type_to_rust(inferred_type));
                    }
                    OwnershipHint::Ref => {
                        if param.name == "self" {
                            let body_modifies = self.function_modifies_self(&analyzed.decl);
                            if let Some(ownership_mode) =
                                self.get_effective_self_ownership(&func.name, analyzed)
                            {
                                match ownership_mode {
                                    OwnershipMode::MutBorrowed => return "&mut self".to_string(),
                                    OwnershipMode::Borrowed => {
                                        if !self.in_trait_impl && body_modifies {
                                            return "&mut self".to_string();
                                        }
                                        return "&self".to_string();
                                    }
                                    OwnershipMode::Owned => {
                                        return "self".to_string();
                                    }
                                }
                            }
                            if !self.in_trait_impl && body_modifies {
                                return "&mut self".to_string();
                            }
                            return "&self".to_string();
                        }
                        // Don't add & if the type is already a Reference
                        if matches!(
                            inferred_type,
                            Type::Reference(_) | Type::MutableReference(_)
                        ) {
                            self.type_to_rust(inferred_type)
                        } else {
                            // TDD FIX: Borrowed → &T (including &String for strings)
                            // Correctness > idioms: &String works with Vec<String> methods
                            format!("&{}", self.type_to_rust(inferred_type))
                        }
                    }
                    OwnershipHint::Mut => {
                        if param.name == "self" {
                            let body_modifies = self.function_modifies_self(&analyzed.decl);
                            if let Some(ownership_mode) =
                                self.get_effective_self_ownership(&func.name, analyzed)
                            {
                                return match ownership_mode {
                                    OwnershipMode::Borrowed => {
                                        if !self.in_trait_impl && body_modifies {
                                            "&mut self".to_string()
                                        } else {
                                            "&self".to_string()
                                        }
                                    }
                                    OwnershipMode::MutBorrowed => "&mut self".to_string(),
                                    OwnershipMode::Owned => {
                                        let ret_self = self.method_returns_impl_struct(&analyzed.decl);
                                        if body_modifies && ret_self {
                                            "mut self".to_string()
                                        } else if body_modifies {
                                            "&mut self".to_string()
                                        } else {
                                            "self".to_string()
                                        }
                                    }
                                };
                            }
                            return "&mut self".to_string();
                        }
                        // Don't add &mut if the type is already a MutableReference
                        if matches!(inferred_type, Type::MutableReference(_)) {
                            self.type_to_rust(inferred_type)
                        } else {
                            format!("&mut {}", self.type_to_rust(inferred_type))
                        }
                    }
                    OwnershipHint::Inferred => {
                        if param.name == "self" {
                            let body_modifies = self.function_modifies_self(&analyzed.decl);
                            let returns_self = self.method_returns_impl_struct(&analyzed.decl);
                            let self_str = if let Some(ownership_mode) =
                                self.get_effective_self_ownership(&func.name, analyzed)
                            {
                                match ownership_mode {
                                    OwnershipMode::Borrowed | OwnershipMode::MutBorrowed
                                        if !self.in_trait_impl && returns_self =>
                                    {
                                        "mut self"
                                    }
                                    OwnershipMode::MutBorrowed => "&mut self",
                                    OwnershipMode::Borrowed => {
                                        if !self.in_trait_impl && body_modifies {
                                            "&mut self"
                                        } else {
                                            "&self"
                                        }
                                    }
                                    OwnershipMode::Owned => {
                                        let ret_self = self.method_returns_impl_struct(&analyzed.decl);
                                        if body_modifies && ret_self {
                                            "mut self"
                                        } else if body_modifies {
                                            "&mut self"
                                        } else {
                                            "self"
                                        }
                                    }
                                }
                            } else if body_modifies && returns_self {
                                "mut self"
                            } else if body_modifies {
                                "&mut self"
                            } else {
                                "self"
                            };
                            // Sync borrowed-params sets with actual generated receiver.
                            match self_str {
                                "&self" => {
                                    self.inferred_borrowed_params.insert("self".to_string());
                                    self.inferred_mut_borrowed_params.remove("self");
                                }
                                "&mut self" => {
                                    self.inferred_mut_borrowed_params.insert("self".to_string());
                                    self.inferred_borrowed_params.remove("self");
                                }
                                _ => {
                                    self.inferred_borrowed_params.remove("self");
                                    self.inferred_mut_borrowed_params.remove("self");
                                }
                            }
                            return self_str.to_string();
                        }

                        // Check if type already has ownership baked in (like &str from string inference)
                        if matches!(
                            inferred_type,
                            Type::Reference(_) | Type::MutableReference(_)
                        ) {
                            // Already has & or &mut - just convert
                            self.type_to_rust(inferred_type)
                        } else {
                            // Apply ownership mode from analyzer
                            // TDD FIX: Default to Owned, not Borrowed
                            // THE WINDJAMMER WAY: Parameters are owned by default unless analyzer
                            // detects they should be borrowed (e.g., only read, passed to & functions)
                            let ownership_mode = analyzed
                                .inferred_ownership
                                .get(&param.name)
                                .unwrap_or(&OwnershipMode::Owned);


                            // E0053 FIX: Trait impl parameters MUST match the trait
                            // definition's parameter types exactly. Look up the trait's
                            // method signature and use its ownership for each parameter.
                            let ownership_mode = if self.in_trait_impl {
                                let trait_param_ownership = self
                                    .current_trait_impl_name
                                    .as_ref()
                                    .and_then(|trait_name| {
                                        let methods = self
                                            .analyzed_trait_methods
                                            .get(trait_name.as_str())
                                            .or_else(|| {
                                                trait_name
                                                    .rfind("::")
                                                    .map(|i| &trait_name[i + 2..])
                                                    .and_then(|key| {
                                                        self.analyzed_trait_methods.get(key)
                                                    })
                                            });
                                        methods.and_then(|m| {
                                            m.get(func.name.as_str()).and_then(|trait_fn| {
                                                // Find matching param by name in the trait
                                                // method's inferred ownership
                                                trait_fn.inferred_ownership.get(&param.name)
                                            })
                                        })
                                    });
                                trait_param_ownership.unwrap_or(&OwnershipMode::Owned)
                            } else {
                                ownership_mode
                            };

                            match ownership_mode {
                                OwnershipMode::Owned => self.type_to_rust(inferred_type),
                                OwnershipMode::Borrowed => {
                                    if type_analysis::is_copy_type(inferred_type) {
                                        // Copy types pass by value even when borrowed
                                        self.type_to_rust(inferred_type)
                                    } else {
                                        // PHASE 2: Check if this string parameter can use &str optimization
                                        let is_string = matches!(inferred_type, Type::String)
                                            || matches!(inferred_type, Type::Custom(ref name) if name == "string");

                                        if is_string && analyzed.str_ref_optimizable_params.contains(&param.name) {
                                            // PHASE 2 OPTIMIZATION: Use &str (zero allocations for literals)
                                            "&str".to_string()
                                        } else {
                                            // PHASE 1 BASELINE: Use &String (correct for Vec<String> methods)
                                            format!("&{}", self.type_to_rust(inferred_type))
                                        }
                                    }
                                }
                                OwnershipMode::MutBorrowed => {
                                    format!("&mut {}", self.type_to_rust(inferred_type))
                                }
                            }
                        }
                    }
                };

                // WINDJAMMER LIFETIME INFERENCE: Add 'a lifetime to reference parameters
                // when the function needs explicit lifetime annotations.
                let type_str = if needs_lifetime && param.name != "self" {
                    if let Some(stripped) = type_str.strip_prefix("&mut ") {
                        format!("&'a mut {}", stripped)
                    } else if let Some(stripped) = type_str.strip_prefix("&") {
                        format!("&'a {}", stripped)
                    } else {
                        type_str
                    }
                } else {
                    type_str
                };

                // TDD FIX: Auto-infer `mut` for owned parameters
                // THE WINDJAMMER WAY: Users don't track mutability - the compiler does.
                // If a parameter has mutating method calls or field mutations,
                // the binding needs `mut` even if not explicitly written.
                let auto_needs_mut = param.name != "self"
                    && !param.is_mutable
                    && matches!(type_str.as_str(), s if !s.starts_with("&"))
                    && self.variable_needs_mut(&param.name);
                let mut_prefix = if param.is_mutable || auto_needs_mut {
                    "mut "
                } else {
                    ""
                };

                // TDD FIX: Prefix unused parameter names with `_` to suppress warnings
                let display_name = if unused_params.contains(&param.name) {
                    format!("_{}", param.name)
                } else {
                    param.name.clone()
                };

                // Check if this is a pattern parameter
                if let Some(pattern) = &param.pattern {
                    // Generate pattern: type syntax
                    format!(
                        "{}{}: {}",
                        mut_prefix,
                        self.generate_pattern(pattern),
                        type_str
                    )
                } else {
                    // Simple name: type syntax
                    format!("{}{}: {}", mut_prefix, display_name, type_str)
                }
            })
            .collect();

        params.extend(additional_params);

        output.push_str(&params.join(", "));
        output.push(')');

        if let Some(return_type) = &func.return_type {
            output.push_str(" -> ");
            if needs_lifetime {
                output.push_str(&crate::codegen::rust::types::type_to_rust_with_lifetime(
                    return_type,
                ));
            } else {
                output.push_str(&self.type_to_rust(return_type));
            }
        }

        // Add where clause if present
        output.push_str(&codegen_helpers::format_where_clause(&func.where_clause));

        output.push_str(" {\n");
        self.indent_level += 1;

        // TDD: Generate function body with return optimization
        // Set flag to enable implicit return for last statement
        let old_in_function_body = self.in_function_body;
        self.in_function_body = true;
        let mut body_code = self.generate_block(&func.body);
        self.in_function_body = old_in_function_body;

        // PHASE 6 OPTIMIZATION: Add defer drop logic before function returns
        // This defers heavy deallocations to a background thread for 10,000x speedup
        if !self.defer_drop_optimizations.is_empty() {
            body_code =
                self.wrap_with_defer_drop(body_code, &self.defer_drop_optimizations.clone());
        }

        output.push_str(&body_code);

        self.indent_level -= 1;
        output.push('}');

        // LOCAL VARIABLE TRACKING: Pop scope when exiting function
        self.local_variable_scopes.pop();

        output
    }
}

