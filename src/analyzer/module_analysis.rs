//! Program-level analysis: module items, multi-pass convergence, and per-pass registry updates.

use std::collections::HashSet;
use std::sync::Arc;

use crate::parser::*;

use super::{AnalyzedFunction, Analyzer, OwnershipMode, ProgramAnalysisResult, SignatureRegistry};

impl<'ast> Analyzer<'ast> {
    /// Register trait definitions from an external program (e.g., imported module)
    /// This allows the analyzer to use trait signatures when analyzing impl blocks
    /// in files that import traits from other modules.
    ///
    /// Also analyzes each trait method into `analyzed_trait_methods` when missing, so
    /// impl-only files compiled **before** the trait's source file still see the contract
    /// (receiver + parameter ownership/types). Without this, `trait_method_receiver_ownership`
    /// returns nothing and Rust emits E0053/E0186.
    pub fn register_traits_from_program(&mut self, program: &Program<'ast>) -> Result<(), String> {
        let empty_registry = SignatureRegistry::new();
        for item in &program.items {
            if let Item::Trait { decl, .. } = item {
                self.trait_definitions
                    .insert(decl.name.clone(), decl.clone());

                let mut to_add: Vec<(String, AnalyzedFunction<'ast>)> = Vec::new();
                for method in &decl.methods {
                    let already = self
                        .analyzed_trait_methods
                        .get(&decl.name)
                        .map(|m| m.contains_key(&method.name))
                        .unwrap_or(false);
                    if already {
                        continue;
                    }

                    // Skip body analysis for abstract trait methods (body = None)
                    // Only analyze trait methods with default implementations
                    if method.body.is_none() {
                        // For abstract trait methods, create a minimal FunctionDecl with empty body
                        // This avoids dereferencing invalid &'ast Statement references
                        let func = FunctionDecl {
                            name: method.name.clone(),
                            is_pub: true,
                            is_extern: false,
                            type_params: vec![],
                            where_clause: vec![],
                            decorators: vec![],
                            is_async: method.is_async,
                            parameters: method.parameters.clone(),
                            return_type: method.return_type.clone(),
                            return_decorators: Vec::new(),
                            body: vec![], // Empty body - no statements to dereference
                            parent_type: None,
                            impl_trait: None,
                            doc_comment: method.doc_comment.clone(),
                        };

                        // Analyze as trait method - this will infer ownership without walking body
                        let analyzed_func = self.analyze_trait_method(
                            &func,
                            &empty_registry,
                            Some(decl.name.as_str()),
                        )?;
                        to_add.push((method.name.clone(), analyzed_func));
                    } else {
                        // Trait method with default implementation - analyze fully
                        let func = FunctionDecl {
                            name: method.name.clone(),
                            is_pub: true,
                            is_extern: false,
                            type_params: vec![],
                            where_clause: vec![],
                            decorators: vec![],
                            is_async: method.is_async,
                            parameters: method.parameters.clone(),
                            return_type: method.return_type.clone(),
                            return_decorators: Vec::new(),
                            body: method.body.clone().unwrap_or_default(),
                            parent_type: None,
                            impl_trait: None,
                            doc_comment: method.doc_comment.clone(),
                        };
                        let analyzed_func = self.analyze_trait_method(
                            &func,
                            &empty_registry,
                            Some(decl.name.as_str()),
                        )?;
                        to_add.push((method.name.clone(), analyzed_func));
                    }
                }
                let entry = self
                    .analyzed_trait_methods
                    .entry(decl.name.clone())
                    .or_default();
                for (name, analyzed_func) in to_add {
                    entry.insert(name, analyzed_func);
                }
            }
        }
        Ok(())
    }
    /// Analyze a program with pre-populated signatures from previously compiled files.
    /// This enables cross-file passthrough ownership inference (e.g., Merchant::add_item
    /// can look up Inventory::add_item's ownership when they're in separate files).
    pub fn analyze_program_with_global_signatures(
        &mut self,
        program: &Program<'ast>,
        global_signatures: &SignatureRegistry,
    ) -> Result<ProgramAnalysisResult<'ast>, String> {
        let global_arc = std::sync::Arc::new(global_signatures.clone());
        self.analyze_program_with_global_arc(program, &global_arc)
    }

    /// Like [`analyze_program_with_global_signatures`] but shares the global registry via
    /// `Arc` — avoids cloning the full crate registry for every file in library builds.
    pub fn analyze_program_with_global_arc(
        &mut self,
        program: &Program<'ast>,
        global_signatures: &std::sync::Arc<SignatureRegistry>,
    ) -> Result<ProgramAnalysisResult<'ast>, String> {
        // THE PROPER SOLUTION: Multi-pass ownership analysis
        // Iterate until convergence - no workarounds, no heuristics, just correctness

        // PHASE -1: LANGUAGE DESIGN CHECK - Prohibit Rust-specific `.as_str()`
        // Windjammer compiler should handle string conversions automatically.
        // Users shouldn't need to know about Rust's &str vs String distinction.
        self.check_forbidden_rust_patterns(program)?;

        // PHASE 0: Collect all enum, struct, and trait definitions
        // This must happen before any function analysis
        for item in &program.items {
            match item {
                Item::Enum { decl, .. } => {
                    // Fieldless enums (unit variants only) are Copy by default
                    use crate::parser::ast::EnumVariantData;
                    let is_copy = decl
                        .variants
                        .iter()
                        .all(|v| matches!(v.data, EnumVariantData::Unit));
                    if is_copy {
                        self.copy_enums.insert(decl.name.clone());
                    }
                }
                Item::Trait { decl, .. } => {
                    // Store trait definition for later lookup
                    self.trait_definitions
                        .insert(decl.name.clone(), decl.clone());
                }
                _ => {}
            }
        }

        // PHASE 0b: Struct Copy registry — fixed-point to match codegen and main.rs PASS 0.
        // Single forward pass fails when struct A references Copy struct B but B is declared
        // later in the file; empty structs must be Copy (same as Rust / trait_derivation).
        let mut struct_infos: Vec<(String, Vec<Type>)> = Vec::new();
        let mut explicit_non_copy: HashSet<String> = HashSet::new();
        for item in &program.items {
            if let Item::Struct { decl, .. } = item {
                let has_derive = decl.decorators.iter().any(|d| d.name == "derive");
                let has_copy_derive = decl.decorators.iter().any(|decorator| {
                    decorator.name == "derive"
                        && decorator.arguments.iter().any(|(_, arg)| {
                            if let crate::parser::ast::Expression::Identifier { name, .. } = arg {
                                name == "Copy"
                            } else {
                                false
                            }
                        })
                });
                if has_copy_derive {
                    Arc::make_mut(&mut self.copy_structs).insert(decl.name.clone());
                } else if has_derive {
                    explicit_non_copy.insert(decl.name.clone());
                }
                struct_infos.push((
                    decl.name.clone(),
                    decl.fields.iter().map(|f| f.field_type.clone()).collect(),
                ));
            }
        }
        // Populate struct field type registry from this file's struct definitions so
        // string-field storage analysis works before cross-file metadata is available.
        {
            use std::collections::HashMap;
            let field_map = Arc::make_mut(&mut self.global_struct_field_types);
            for item in &program.items {
                if let Item::Struct { decl, .. } = item {
                    let mut fields = HashMap::new();
                    for f in &decl.fields {
                        fields.insert(f.name.clone(), f.field_type.clone());
                    }
                    field_map.insert(decl.name.clone(), fields);
                }
            }
        }
        const MAX_COPY_STRUCT_PASSES: usize = 64;
        for _ in 0..MAX_COPY_STRUCT_PASSES {
            let mut changed = false;
            for (name, field_types) in &struct_infos {
                if self.copy_structs.contains(name) || explicit_non_copy.contains(name) {
                    continue;
                }
                let all_copy =
                    field_types.is_empty() || field_types.iter().all(|ft| self.is_copy_type(ft));
                if all_copy {
                    Arc::make_mut(&mut self.copy_structs).insert(name.clone());
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }

        // MULTI-PASS OWNERSHIP INFERENCE
        // Continue analyzing until ownership signatures stabilize (convergence)
        const MAX_PASSES: usize = 10; // Safety limit to prevent infinite loops

        let mut registry = SignatureRegistry::layered(std::sync::Arc::clone(global_signatures));
        let mut pass_number = 1;

        if self.ownership_preconverged {
            let (new_analyzed, new_registry) = self.analyze_program_pass(program, &registry)?;
            self.infer_trait_signatures_from_impls(program, &new_registry)?;
            return Ok((
                new_analyzed,
                new_registry,
                self.analyzed_trait_methods.clone(),
            ));
        }

        loop {
            let (new_analyzed, new_registry) = self.analyze_program_pass(program, &registry)?;

            // Check for convergence: did any signatures change?
            let converged = self.signatures_converged(&registry, &new_registry);

            if converged {
                self.infer_trait_signatures_from_impls(program, &new_registry)?;
                return Ok((
                    new_analyzed,
                    new_registry,
                    self.analyzed_trait_methods.clone(),
                ));
            }

            if pass_number >= MAX_PASSES {
                eprintln!(
                    "⚠️  Warning: Ownership analysis did not converge after {} passes",
                    MAX_PASSES
                );
                eprintln!("    Using last known signatures (may be suboptimal)");
                self.infer_trait_signatures_from_impls(program, &new_registry)?;
                return Ok((
                    new_analyzed,
                    new_registry,
                    self.analyzed_trait_methods.clone(),
                ));
            }

            // Update registry for next pass
            registry = new_registry;
            pass_number += 1;
        }
    }
    /// Helper: Check if two signature registries have converged (no changes)
    pub(crate) fn signatures_converged(
        &self,
        old: &SignatureRegistry,
        new: &SignatureRegistry,
    ) -> bool {
        // If sizes differ, not converged
        if old.signatures.len() != new.signatures.len() {
            return false;
        }

        // Compare each signature
        for (name, new_sig) in &new.signatures {
            match old.signatures.get(name) {
                None => return false, // New function appeared
                Some(old_sig) => {
                    // Compare parameter ownership modes
                    if old_sig.param_ownership.len() != new_sig.param_ownership.len() {
                        return false;
                    }

                    for (old_ownership, new_ownership) in
                        old_sig.param_ownership.iter().zip(&new_sig.param_ownership)
                    {
                        if old_ownership != new_ownership {
                            return false;
                        }
                    }

                    // Compare return type ownership
                    if old_sig.return_ownership != new_sig.return_ownership {
                        return false;
                    }
                }
            }
        }

        true // All signatures match
    }
    /// Helper: Single pass of program analysis
    /// Uses the provided registry to infer ownership, returns updated analysis and registry
    pub(crate) fn analyze_program_pass(
        &mut self,
        program: &Program<'ast>,
        existing_registry: &SignatureRegistry,
    ) -> Result<(Vec<AnalyzedFunction<'ast>>, SignatureRegistry), String> {
        let mut analyzed = Vec::new();
        let mut registry = existing_registry.clone();

        // NOTE: Trait signature inference is now done GLOBALLY after all files are compiled
        // See ModuleCompiler::finalize_trait_inference() in main.rs
        // (We no longer call infer_trait_signatures_from_impls here for single files)

        for item in &program.items {
            match item {
                Item::Function { decl: func, .. } => {
                    let mut analyzed_func = self.analyze_function(func, &registry)?;

                    if !self.convergence_only {
                        // PHASE 7: Detect const/static optimizations
                        analyzed_func.const_static_optimizations =
                            self.detect_const_static_opportunities(&analyzed_func);

                        // PHASE 8: Detect SmallVec optimizations
                        analyzed_func.smallvec_optimizations =
                            self.detect_smallvec_opportunities(func);

                        // PHASE 9: Detect Cow optimizations
                        analyzed_func.cow_optimizations = self.detect_cow_opportunities(func);

                        analyzed_func.cache_locality = self.analyze_cache_locality(program, func);
                    }

                    let signature = self.build_signature(&analyzed_func);
                    registry.add_function(func.name.clone(), signature);
                    analyzed.push(analyzed_func);
                }
                Item::Impl {
                    block: impl_block, ..
                } => {
                    // TDD FIX: Multi-pass fixed-point iteration for transitive mutability inference
                    //
                    // Problem: Single-pass analysis fails for multi-level call chains:
                    //   update() calls poll_input() which calls keyboard.update_key(&mut self)
                    //   Single pass: update(&self) ❌ (wrong!)
                    //   Multi-pass: update(&mut self) ✅ (correct!)
                    //
                    // Solution: Iterate until no signatures change (fixed-point)
                    let mut analyzed_funcs: std::collections::HashMap<
                        String,
                        AnalyzedFunction<'ast>,
                    > = std::collections::HashMap::new();
                    let mut local_registry = registry.clone();

                    // Pass 1: Initial analysis (direct mutations only)
                    for func in &impl_block.functions {
                        let analyzed_func = if let Some(trait_name) = &impl_block.trait_name {
                            self.analyze_trait_impl_function(
                                func,
                                trait_name,
                                impl_block,
                                program,
                                &local_registry,
                            )?
                        } else {
                            self.analyze_function_in_impl(
                                func,
                                impl_block,
                                program,
                                &local_registry,
                            )?
                        };
                        analyzed_funcs.insert(func.name.clone(), analyzed_func);
                    }

                    // Pass 2-N: Fixed-point iteration (propagate transitive mutations)
                    let mut changed = true;
                    let mut iteration = 0;
                    const MAX_ITERATIONS: usize = 10; // Safety limit

                    while changed && iteration < MAX_ITERATIONS {
                        changed = false;
                        iteration += 1;

                        // Update local registry with current analyzed signatures
                        for (name, analyzed_func) in &analyzed_funcs {
                            let signature = self.build_signature(analyzed_func);
                            let qualified_name = format!("{}::{}", impl_block.type_name, name);
                            local_registry.add_function(qualified_name, signature.clone());
                            local_registry.add_function(name.clone(), signature);
                        }

                        // Re-analyze all methods with updated registry
                        for func in &impl_block.functions {
                            let new_analyzed = if let Some(trait_name) = &impl_block.trait_name {
                                self.analyze_trait_impl_function(
                                    func,
                                    trait_name,
                                    impl_block,
                                    program,
                                    &local_registry,
                                )?
                            } else {
                                self.analyze_function_in_impl(
                                    func,
                                    impl_block,
                                    program,
                                    &local_registry,
                                )?
                            };

                            // Check if self ownership changed
                            let old_analyzed = &analyzed_funcs[&func.name];
                            let old_self_ownership = old_analyzed
                                .inferred_ownership
                                .get("self")
                                .copied()
                                .unwrap_or(OwnershipMode::Owned);
                            let new_self_ownership = new_analyzed
                                .inferred_ownership
                                .get("self")
                                .copied()
                                .unwrap_or(OwnershipMode::Owned);

                            if old_self_ownership != new_self_ownership {
                                analyzed_funcs.insert(func.name.clone(), new_analyzed);
                                changed = true;
                            }
                        }
                    }

                    // Process all analyzed functions (after fixed-point convergence)
                    let is_trait_impl = impl_block.trait_name.is_some();
                    for func in &impl_block.functions {
                        let analyzed_func_opt = analyzed_funcs.remove(&func.name);
                        if analyzed_func_opt.is_none() {
                            // Duplicate function name in impl block -- skip the second
                            // occurrence. The first definition wins (already processed).
                            continue;
                        }
                        let mut analyzed_func = analyzed_func_opt.unwrap();

                        if !self.convergence_only {
                            // PHASE 7: Detect const/static optimizations
                            analyzed_func.const_static_optimizations =
                                self.detect_const_static_opportunities(&analyzed_func);

                            // PHASE 8: Detect SmallVec optimizations
                            analyzed_func.smallvec_optimizations =
                                self.detect_smallvec_opportunities(func);

                            // PHASE 9: Detect Cow optimizations
                            analyzed_func.cow_optimizations = self.detect_cow_opportunities(func);

                            analyzed_func.cache_locality =
                                self.analyze_cache_locality(program, func);
                        }

                        let signature = self.build_signature(&analyzed_func);

                        let qualified_name = format!("{}::{}", impl_block.type_name, func.name);
                        if is_trait_impl {
                            // Trait impl methods: don't overwrite a direct impl's entry.
                            // Callers like `obj.method()` resolve to the direct impl in Rust,
                            // so the registry's Type::method entry must reflect the direct impl's
                            // signature (parameter types and ownership).
                            if registry.get_signature(&qualified_name).is_none() {
                                registry.add_function(qualified_name.clone(), signature.clone());
                            }
                            // Also register under Trait::method for trait-based lookups.
                            if let Some(trait_name) = &impl_block.trait_name {
                                let trait_qualified = format!("{}::{}", trait_name, func.name);
                                registry.add_function(trait_qualified, signature.clone());
                            }
                        } else {
                            // Direct impl methods always take priority in the registry.
                            registry.add_function(qualified_name.clone(), signature.clone());
                        }
                        // Generic type base name registration
                        if let Some(base_name) = impl_block.type_name.split('<').next() {
                            if base_name != impl_block.type_name {
                                let base_qualified = format!("{}::{}", base_name, func.name);
                                if !is_trait_impl
                                    || registry.get_signature(&base_qualified).is_none()
                                {
                                    registry.add_function(base_qualified, signature.clone());
                                }
                            }
                        }
                        // Direct impl methods must not register bare names — homonyms like
                        // `check_collision` would overwrite imported free functions in the global registry.
                        if is_trait_impl && registry.get_signature(&func.name).is_none() {
                            registry.add_function(func.name.clone(), signature);
                        }

                        analyzed.push(analyzed_func);
                    }
                }
                Item::Trait { decl, .. } => {
                    // THE WINDJAMMER WAY: Analyze ALL trait methods, not just default impls.
                    // Abstract methods need ownership inference too - the compiler must set
                    // the correct self convention (&self, &mut self) even without a body.
                    // This is refined later by infer_trait_signatures_from_impls.
                    for method in &decl.methods {
                        // Convert TraitMethod to FunctionDecl for analysis
                        let func = FunctionDecl {
                            name: method.name.clone(),
                            is_pub: true, // Trait methods are public
                            is_extern: false,
                            type_params: vec![],
                            where_clause: vec![],
                            decorators: vec![],
                            is_async: method.is_async,
                            parameters: method.parameters.clone(),
                            return_type: method.return_type.clone(),
                            return_decorators: Vec::new(),
                            body: method.body.clone().unwrap_or_default(),
                            parent_type: None,
                            impl_trait: None,
                            doc_comment: method.doc_comment.clone(),
                        };

                        // Trait methods (both abstract and default) should use &self or &mut self
                        // to work with unsized types. The Windjammer way: make it work!
                        let mut analyzed_func =
                            self.analyze_trait_method(&func, &registry, Some(decl.name.as_str()))?;

                        if !self.convergence_only {
                            // PHASE 7: Detect const/static optimizations
                            analyzed_func.const_static_optimizations =
                                self.detect_const_static_opportunities(&analyzed_func);

                            // PHASE 8: Detect SmallVec optimizations
                            analyzed_func.smallvec_optimizations =
                                self.detect_smallvec_opportunities(&func);

                            // PHASE 9: Detect Cow optimizations
                            analyzed_func.cow_optimizations = self.detect_cow_opportunities(&func);

                            analyzed_func.cache_locality =
                                self.analyze_cache_locality(program, &func);
                        }

                        // THE WINDJAMMER WAY: Store analyzed trait method for trait impl matching
                        // BUT: Don't overwrite if cross-file inference has already set it!
                        // (finalize_trait_inference runs globally and sets the most permissive signature)
                        let trait_methods = self
                            .analyzed_trait_methods
                            .entry(decl.name.clone())
                            .or_default();

                        // Merge: if the impl body infers a stronger ownership
                        // than the abstract trait stub, upgrade the trait entry.
                        if let Some(existing) = trait_methods.get(&func.name) {
                            let existing_self = existing.inferred_ownership.get("self").copied();
                            let new_self = analyzed_func.inferred_ownership.get("self").copied();
                            let should_upgrade = matches!(
                                (existing_self, new_self),
                                (None, Some(_))
                                    | (
                                        Some(OwnershipMode::Borrowed),
                                        Some(OwnershipMode::MutBorrowed | OwnershipMode::Owned)
                                    )
                                    | (
                                        Some(OwnershipMode::MutBorrowed),
                                        Some(OwnershipMode::Owned)
                                    )
                            );
                            if should_upgrade {
                                trait_methods.insert(func.name.clone(), analyzed_func.clone());
                            }
                        } else {
                            trait_methods.insert(func.name.clone(), analyzed_func.clone());
                        }

                        // Add trait methods to analyzed list so codegen can access ownership info
                        // They won't be generated as standalone functions (codegen skips trait methods)
                        let signature = self.build_signature(&analyzed_func);
                        registry.add_function(func.name.clone(), signature.clone());
                        // Also register as TraitName::method for cross-file meta lookup
                        let qualified_name = format!("{}::{}", decl.name, func.name);
                        registry.add_function(qualified_name, signature);
                        analyzed.push(analyzed_func);
                    }
                }
                Item::Static { mutable, value, .. } => {
                    // Analyze static declarations for const promotion
                    if !mutable && self.is_const_evaluable(value) {
                        // This static can be promoted to const
                        // Store in a global optimization list (TODO: add to Program-level analysis)
                    }
                }
                Item::Mod { items, .. } => {
                    // Recursively analyze items inside inline modules
                    // NOTE: We analyze them for signature registry, but don't add them
                    // to the top-level analyzed list since they'll be generated inside their modules
                    for item in items {
                        match item {
                            Item::Function { decl: func, .. } => {
                                let mut analyzed_func = self.analyze_function(func, &registry)?;
                                analyzed_func.const_static_optimizations =
                                    self.detect_const_static_opportunities(&analyzed_func);
                                analyzed_func.smallvec_optimizations =
                                    self.detect_smallvec_opportunities(func);
                                analyzed_func.cow_optimizations =
                                    self.detect_cow_opportunities(func);
                                analyzed_func.cache_locality =
                                    self.analyze_cache_locality(program, func);
                                let signature = self.build_signature(&analyzed_func);
                                registry.add_function(func.name.clone(), signature);
                                // Add to analyzed list for codegen to access (but marked as in-module)
                                analyzed.push(analyzed_func);
                            }
                            Item::Impl {
                                block: impl_block, ..
                            } => {
                                // TDD FIX: Multi-pass fixed-point iteration (same as top-level impl blocks)
                                let mut analyzed_funcs: std::collections::HashMap<
                                    String,
                                    AnalyzedFunction<'ast>,
                                > = std::collections::HashMap::new();
                                let mut local_registry = registry.clone();

                                // Pass 1: Initial analysis
                                for func in &impl_block.functions {
                                    let analyzed_func =
                                        if let Some(trait_name) = &impl_block.trait_name {
                                            self.analyze_trait_impl_function(
                                                func,
                                                trait_name,
                                                impl_block,
                                                program,
                                                &local_registry,
                                            )?
                                        } else {
                                            self.analyze_function_in_impl(
                                                func,
                                                impl_block,
                                                program,
                                                &local_registry,
                                            )?
                                        };
                                    analyzed_funcs.insert(func.name.clone(), analyzed_func);
                                }

                                // Pass 2-N: Fixed-point iteration
                                let mut changed = true;
                                let mut iteration = 0;
                                const MAX_ITERATIONS: usize = 10;

                                while changed && iteration < MAX_ITERATIONS {
                                    changed = false;
                                    iteration += 1;

                                    // Update registry
                                    for (name, analyzed_func) in &analyzed_funcs {
                                        let signature = self.build_signature(analyzed_func);
                                        local_registry.add_function(name.clone(), signature);
                                    }

                                    // Re-analyze
                                    for func in &impl_block.functions {
                                        let new_analyzed =
                                            if let Some(trait_name) = &impl_block.trait_name {
                                                self.analyze_trait_impl_function(
                                                    func,
                                                    trait_name,
                                                    impl_block,
                                                    program,
                                                    &local_registry,
                                                )?
                                            } else {
                                                self.analyze_function_in_impl(
                                                    func,
                                                    impl_block,
                                                    program,
                                                    &local_registry,
                                                )?
                                            };

                                        // Check if ownership changed
                                        let old_analyzed = &analyzed_funcs[&func.name];
                                        let old_self = old_analyzed
                                            .inferred_ownership
                                            .get("self")
                                            .copied()
                                            .unwrap_or(OwnershipMode::Owned);
                                        let new_self = new_analyzed
                                            .inferred_ownership
                                            .get("self")
                                            .copied()
                                            .unwrap_or(OwnershipMode::Owned);

                                        if old_self != new_self {
                                            analyzed_funcs.insert(func.name.clone(), new_analyzed);
                                            changed = true;
                                        }
                                    }
                                }

                                // Process converged results
                                let is_trait_impl = impl_block.trait_name.is_some();
                                for func in &impl_block.functions {
                                    let mut analyzed_func = analyzed_funcs
                                        .remove(&func.name)
                                        .expect("Function should exist");

                                    analyzed_func.const_static_optimizations =
                                        self.detect_const_static_opportunities(&analyzed_func);
                                    analyzed_func.smallvec_optimizations =
                                        self.detect_smallvec_opportunities(func);
                                    analyzed_func.cow_optimizations =
                                        self.detect_cow_opportunities(func);

                                    analyzed_func.cache_locality =
                                        self.analyze_cache_locality(program, func);

                                    let signature = self.build_signature(&analyzed_func);
                                    let qualified_name =
                                        format!("{}::{}", impl_block.type_name, func.name);
                                    if is_trait_impl {
                                        if registry.get_signature(&qualified_name).is_none() {
                                            registry
                                                .add_function(qualified_name, signature.clone());
                                        }
                                        if let Some(trait_name) = &impl_block.trait_name {
                                            let trait_qualified =
                                                format!("{}::{}", trait_name, func.name);
                                            registry
                                                .add_function(trait_qualified, signature.clone());
                                        }
                                    } else {
                                        registry.add_function(qualified_name, signature.clone());
                                    }
                                    if is_trait_impl
                                        && registry.get_signature(&func.name).is_none()
                                    {
                                        registry.add_function(func.name.clone(), signature);
                                    }
                                    analyzed.push(analyzed_func);
                                }
                            }
                            // Could recursively handle nested modules here
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }

        Ok((analyzed, registry))
    }
}
