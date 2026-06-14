//! Self receiver, trait ownership, lifetime, and #[inline] heuristics for function codegen.

use crate::analyzer::*;
use crate::codegen::rust::ast_utilities;
use crate::parser::*;

use super::CodeGenerator;

impl<'ast> CodeGenerator<'ast> {
    pub(super) fn method_returns_impl_struct(&self, func: &FunctionDecl) -> bool {
        use crate::parser::Type;
        let returns_self_type = match &func.return_type {
            Some(Type::Custom(name)) => {
                self.current_struct_name.as_ref().is_some_and(|s| s == name)
            }
            _ => false,
        };
        if !returns_self_type {
            return false;
        }

        // Builder / consuming pattern: method flows `self` through or mutates while building.
        // Read-only factories (snapshot, clone, etc.) construct a NEW instance from cloned
        // fields and must use `&self` — not a hardcoded name list.
        if self.function_returns_self_type(func) {
            return true;
        }
        if super::self_analysis::function_flows_self_through_local(func) {
            return true;
        }
        if self.function_modifies_self(func) {
            return true;
        }
        false
    }

    pub(super) fn function_modifies_self_or_derived(&self, func: &FunctionDecl) -> bool {
        super::self_analysis::function_modifies_self_or_derived_local(
            func,
            Some(&self.signature_registry),
            self.current_struct_name.as_deref(),
        )
    }

    /// Resolve the Rust receiver for a method whose analyzer ownership is `Owned`.
    ///
    /// The analyzer sets Owned for several reasons: match-on-self, consuming field
    /// iteration, builder pattern, returning Self type, bare self consumption,
    /// consuming method calls on self, non-Copy field moves, etc.
    /// Codegen refines Owned into the appropriate Rust receiver:
    /// - `mut self` — builder pattern (modifies + returns Self)
    /// - `&mut self` — mutates fields only (e.g., set field to None, increment)
    /// - `self` — all other Owned cases (the analyzer already validated consumption)
    pub(super) fn owned_self_receiver(&self, func: &FunctionDecl) -> &'static str {
        let body_modifies = self.function_modifies_self_or_derived(func);
        let returns_impl_struct = self.method_returns_impl_struct(func);

        if body_modifies && returns_impl_struct {
            "mut self"
        } else if body_modifies {
            "&mut self"
        } else {
            "self"
        }
    }

    /// Check if the struct currently being implemented is a Copy type.
    /// For Copy types, `self` (by value) is preferred over `&self` since
    /// the copy is trivially cheap and avoids unnecessary indirection.
    pub(super) fn current_struct_is_copy(&self) -> bool {
        if let Some(ref name) = self.current_struct_name {
            self.is_type_copy(&Type::Custom(name.clone()))
        } else {
            false
        }
    }

    pub(super) fn function_returns_self_type(&self, func: &FunctionDecl) -> bool {
        // Check if the function returns Self (for builder pattern detection)
        use crate::parser::{Expression, Statement, Type};

        // First check if return type is a custom type (struct type)
        let returns_custom_type = matches!(&func.return_type, Some(Type::Custom(_)));

        if !returns_custom_type {
            return false;
        }

        // Now check if the function body actually returns `self`
        // Check the last statement in the body
        if let Some(last_stmt) = func.body.last() {
            match last_stmt {
                Statement::Return {
                    value: Some(expr), ..
                } => {
                    // Explicit return self
                    matches!(expr, Expression::Identifier { name, .. } if name == "self")
                }
                Statement::Expression { expr, .. } => {
                    // Implicit return self (last expression)
                    matches!(expr, Expression::Identifier { name, .. } if name == "self")
                }
                _ => false,
            }
        } else {
            false
        }
    }

    pub(super) fn function_modifies_self(&self, func: &FunctionDecl) -> bool {
        super::self_analysis::function_modifies_self(
            func,
            Some(&self.signature_registry),
            self.current_struct_name.as_deref(),
        )
    }

    /// Record when codegen upgrades a self receiver beyond what the analyzer inferred.
    /// For example, when analyzer says Borrowed but body analysis detects mutation,
    /// codegen generates &mut self. This must be recorded so metadata reflects the
    /// actual generated code for cross-file builds.
    pub(super) fn record_self_receiver_upgrade(
        &mut self,
        func_name: &str,
        analyzer_ownership: Option<OwnershipMode>,
        actual_receiver: &str,
    ) {
        let actual_mode = match actual_receiver {
            "&mut self" | "mut self" => OwnershipMode::MutBorrowed,
            "&self" => OwnershipMode::Borrowed,
            "self" => OwnershipMode::Owned,
            _ => return,
        };

        let analyzer_mode = analyzer_ownership.unwrap_or(OwnershipMode::Borrowed);
        if actual_mode == OwnershipMode::MutBorrowed && analyzer_mode == OwnershipMode::Borrowed {
            if let Some(struct_name) = &self.current_struct_name {
                let qualified = format!("{}::{}", struct_name, func_name);
                self.self_receiver_upgrades
                    .insert(qualified, OwnershipMode::MutBorrowed);
            }
        }
    }

    /// E0053 FIX: Get effective self ownership - trait's when in trait impl, else analyzed's.
    /// Impl methods MUST match trait signature exactly.
    ///
    /// The trait codegen defaults to `&mut self` for most methods (unless analysis
    /// explicitly found `Borrowed`). The impl must match what the TRAIT generated.
    /// When we can't determine the trait's choice, default to `&mut self` (most permissive).
    pub(super) fn get_effective_self_ownership(
        &self,
        func_name: &str,
        analyzed: &AnalyzedFunction<'ast>,
    ) -> Option<OwnershipMode> {
        if self.in_trait_impl {
            if let Some(trait_name) = &self.current_trait_impl_name {
                let methods = self.analyzed_trait_methods.get(trait_name).or_else(|| {
                    trait_name
                        .rfind("::")
                        .map(|i| &trait_name[i + 2..])
                        .and_then(|key| self.analyzed_trait_methods.get(key))
                });
                if let Some(trait_method) = methods.and_then(|m| m.get(func_name)) {
                    let ownership = trait_method.inferred_ownership.get("self").copied();
                    match ownership {
                        Some(OwnershipMode::Borrowed) | Some(OwnershipMode::MutBorrowed) => {
                            return ownership;
                        }
                        Some(OwnershipMode::Owned) => {
                            // Explicit or inferred consuming `self` on the trait (e.g. `fn consume(self) -> T`).
                            return Some(OwnershipMode::Owned);
                        }
                        None => {
                            // Abstract trait method (no body): use the impl's own
                            // analyzed ownership so that consuming impls
                            // (e.g. `self.value` for non-Copy) get owned `self`.
                            let impl_ownership = analyzed.inferred_ownership.get("self").copied();
                            return impl_ownership.or(Some(OwnershipMode::Borrowed));
                        }
                    }
                }
                // Trait exists but this specific method wasn't in the analysis map.
                // Trait codegen defaults to &self for unanalyzed methods, so match that.
                if methods.is_some() {
                    return Some(OwnershipMode::Borrowed);
                }
                // Cross-file trait impl: trait not in registry at all (single-file compilation).
                // Choose self ownership based on known trait conventions:
                // - Operator traits (Add, Sub, etc.) require consuming self
                // - Derive traits (Display, Debug, etc.) require &self
                // - Custom traits: check signature registry first, then body analysis
                let base_trait = trait_name
                    .rfind("::")
                    .map(|i| &trait_name[i + 2..])
                    .unwrap_or(trait_name);
                return Some(
                    if crate::type_classification::is_owned_self_trait(base_trait) {
                        OwnershipMode::Owned
                    } else if crate::type_classification::is_ref_receiver_trait(base_trait) {
                        OwnershipMode::Borrowed
                    } else {
                        // Check signature registry for the trait method's self ownership.
                        // This handles cross-file trait impls where metadata was loaded.
                        let registry_ownership =
                            self.lookup_trait_method_ownership_in_registry(trait_name, func_name);
                        if let Some(reg_own) = registry_ownership {
                            reg_own
                        } else if let Some(body_ownership) =
                            analyzed.inferred_ownership.get("self").copied()
                        {
                            body_ownership
                        } else {
                            if analyzed.decl.return_type.is_some() {
                                OwnershipMode::Borrowed
                            } else {
                                OwnershipMode::MutBorrowed
                            }
                        }
                    },
                );
            }
        }
        analyzed.inferred_ownership.get("self").copied()
    }

    /// Look up a trait method's self-ownership in the signature registry.
    /// Checks patterns like "TraitName::method", "module::TraitName::method", etc.
    fn lookup_trait_method_ownership_in_registry(
        &self,
        trait_name: &str,
        method_name: &str,
    ) -> Option<OwnershipMode> {
        let base_trait = trait_name
            .rfind("::")
            .map(|i| &trait_name[i + 2..])
            .unwrap_or(trait_name);

        // Try various qualified name patterns
        let patterns = [
            format!("{}::{}", base_trait, method_name),
            format!("{}::{}", trait_name, method_name),
        ];

        for pattern in &patterns {
            if let Some(sig) = self.signature_registry.get_signature(pattern) {
                if sig.has_self_receiver {
                    if let Some(&ownership) = sig.param_ownership.first() {
                        return Some(ownership);
                    }
                }
            }
        }

        // Also try suffix matching for fully qualified paths
        let suffix = format!("{}::{}", base_trait, method_name);
        for (key, sig) in &self.signature_registry.signatures {
            if key.ends_with(&suffix) && sig.has_self_receiver {
                if let Some(&ownership) = sig.param_ownership.first() {
                    return Some(ownership);
                }
            }
        }

        None
    }

    /// WINDJAMMER LIFETIME INFERENCE: Determine if a function needs explicit lifetime annotations.
    ///
    /// Rust's lifetime elision rules handle most cases:
    ///   1. Single input reference → output gets that lifetime
    ///   2. &self/&mut self → output gets self's lifetime
    ///   3. Multiple input references with no self → MUST be explicit
    ///
    /// We only add 'a when case 3 applies AND the return type contains references.
    pub(super) fn function_needs_lifetime_annotations(
        &self,
        func: &FunctionDecl<'ast>,
        analyzed: &AnalyzedFunction<'ast>,
    ) -> bool {
        use crate::codegen::rust::types::type_contains_reference;

        // First check: does the return type contain any references?
        let return_has_ref = match &func.return_type {
            Some(ret_type) => type_contains_reference(ret_type),
            None => false,
        };

        if !return_has_ref {
            return false;
        }

        // Check if there's a self parameter (explicit or inferred)
        let has_self = func.parameters.iter().any(|p| p.name == "self")
            || analyzed.inferred_ownership.contains_key("self");

        if has_self {
            // &self/&mut self methods: Rust elision rule 2 handles this
            return false;
        }

        // Count the number of reference parameters (explicit refs + analyzer-inferred refs)
        let ref_param_count = func
            .parameters
            .iter()
            .enumerate()
            .filter(|(param_idx, param)| {
                if param.name == "self" {
                    return false;
                }

                // Check if the parameter type is already a reference
                let inferred_type = analyzed
                    .inferred_param_types
                    .get(*param_idx)
                    .unwrap_or(&param.type_);

                if matches!(
                    inferred_type,
                    Type::Reference(_) | Type::MutableReference(_)
                ) {
                    return true;
                }

                // Check explicit ownership hints
                if matches!(
                    param.ownership,
                    crate::parser::OwnershipHint::Ref | crate::parser::OwnershipHint::Mut
                ) {
                    return true;
                }

                // Check analyzer-inferred ownership
                if let Some(ownership) = analyzed.inferred_ownership.get(&param.name) {
                    matches!(
                        ownership,
                        crate::analyzer::OwnershipMode::Borrowed
                            | crate::analyzer::OwnershipMode::MutBorrowed
                    )
                } else {
                    false
                }
            })
            .count();

        // Need explicit lifetime when 2+ reference params and reference return
        ref_param_count >= 2
    }

    /// OPTIMIZATION: Determine if a function should be marked #[inline]
    /// Phase 1: Generate Inlinable Code
    ///
    /// Heuristics for inlining:
    /// 1. Module functions (stdlib wrappers) - always inline for zero-cost abstraction
    /// 2. Small functions (< 10 statements) - likely to benefit from inlining
    /// 3. Trivial getters/setters - always inline
    /// 4. Functions with only one return statement - simple enough to inline
    /// 5. Don't inline: main(), test functions, async functions, large functions
    pub(super) fn should_inline_function(
        &self,
        func: &FunctionDecl,
        _analyzed: &AnalyzedFunction,
    ) -> bool {
        // Never inline main
        if func.name == "main" {
            return false;
        }

        // Never inline test functions
        if func.decorators.iter().any(|d| d.name == "test") {
            return false;
        }

        // Don't inline async functions (they're already state machines)
        if func.decorators.iter().any(|d| d.name == "async") {
            return false;
        }

        // ALWAYS inline module functions (stdlib wrappers)
        // These are thin wrappers around Rust stdlib and should have zero overhead
        if self.is_module {
            return true;
        }

        // Count statements in function body
        let statement_count = ast_utilities::count_statements(&func.body);

        // Inline small functions (< 10 statements)
        if statement_count < 10 {
            return true;
        }

        // Inline trivial single-expression functions
        if statement_count == 1 {
            if let Statement::Return { value: Some(_), .. } = &func.body[0] {
                return true;
            }
            if let Statement::Expression { .. } = &func.body[0] {
                return true;
            }
        }

        // Default: don't inline large functions
        false
    }
}
