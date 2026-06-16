//! Trait receiver ownership: merging impls, default trait method analysis, and hint conversion.

use crate::parser::*;

use super::{AnalyzedFunction, Analyzer, OwnershipHint, OwnershipMode, SignatureRegistry};

impl<'ast> Analyzer<'ast> {
    /// Infer `&self` vs `&mut self` for trait methods from body analysis only.
    /// No name-based heuristics — mutation, mutating self calls, and impl merging decide.
    pub(super) fn infer_trait_self_receiver_from_body(
        &self,
        func: &FunctionDecl<'ast>,
        registry: &SignatureRegistry,
    ) -> OwnershipMode {
        if self.function_modifies_self_fields_with_registry(func, Some(registry)) {
            return OwnershipMode::MutBorrowed;
        }
        let mut visited = std::collections::HashSet::new();
        if self.function_calls_mutating_self_methods_with_registry(
            func,
            Some(registry),
            &mut visited,
        ) {
            return OwnershipMode::MutBorrowed;
        }
        OwnershipMode::Borrowed
    }

    /// Trait receiver when `self` is omitted or inferred (not explicit `&self` / `&mut self`).
    /// Abstract methods (empty body): void → `&mut self`, value-returning → `&self`.
    /// Methods with bodies: analyze usage; consuming `-> Self` builders use owned `self`.
    fn infer_trait_self_receiver(
        &self,
        func: &FunctionDecl<'ast>,
        registry: &SignatureRegistry,
        trait_method_returns_self: bool,
    ) -> OwnershipMode {
        if trait_method_returns_self && self.function_returns_self(func) {
            return OwnershipMode::Owned;
        }
        if func.body.is_empty() {
            // Abstract trait methods: safe object-safe default is &self.
            // Impl merging upgrades to &mut self when any impl body mutates.
            return OwnershipMode::Borrowed;
        }
        self.infer_trait_self_receiver_from_body(func, registry)
    }

    /// Upgrade stored trait method receiver when an impl body needs a stronger borrow.
    pub(super) fn upgrade_trait_method_self_receiver(
        &mut self,
        trait_name: &str,
        trait_key: &str,
        method_name: &str,
        receiver: OwnershipMode,
    ) {
        for key in [trait_name, trait_key] {
            if let Some(methods) = self.analyzed_trait_methods.get_mut(key) {
                if let Some(method) = methods.get_mut(method_name) {
                    method
                        .inferred_ownership
                        .insert("self".to_string(), receiver);
                }
            }
        }
    }

    /// Helper: Convert OwnershipHint to OwnershipMode
    pub(super) fn convert_ownership_hint_to_mode(
        &self,
        hint: &OwnershipHint,
        param_name: &str,
    ) -> OwnershipMode {
        match hint {
            OwnershipHint::Owned => OwnershipMode::Owned,
            OwnershipHint::Ref => OwnershipMode::Borrowed,
            OwnershipHint::Mut => OwnershipMode::MutBorrowed,
            OwnershipHint::Inferred => {
                // For inferred parameters, default to borrowed for self, owned otherwise
                if param_name == "self" {
                    OwnershipMode::Borrowed
                } else {
                    OwnershipMode::Owned
                }
            }
        }
    }

    /// THE WINDJAMMER WAY: Infer trait method signatures from ALL implementations
    /// If any impl needs &mut self, the trait gets &mut self
    /// The compiler does the work, not the user!
    pub fn infer_trait_signatures_from_impls(
        &mut self,
        program: &Program<'ast>,
        registry: &SignatureRegistry,
    ) -> Result<(), String> {
        use std::collections::HashMap;

        // Step 1: Collect all trait implementations WITH their impl blocks
        // THE WINDJAMMER WAY: We need the impl block for proper ownership analysis
        // Map: trait_name -> Vec<(ImplBlock, functions)>
        let mut trait_impls: HashMap<String, Vec<crate::parser::ast::ImplBlock>> = HashMap::new();

        for item in &program.items {
            if let Item::Impl {
                block: impl_block, ..
            } = item
            {
                if let Some(trait_name) = &impl_block.trait_name {
                    trait_impls
                        .entry(trait_name.clone())
                        .or_default()
                        .push(impl_block.clone());
                }
            }
        }

        // Step 2: For each trait, analyze ALL implementations and determine most permissive signature
        for (trait_name, impl_blocks) in trait_impls {
            let trait_methods_opt = self
                .analyzed_trait_methods
                .get(&trait_name)
                .cloned()
                .or_else(|| {
                    trait_name
                        .rfind("::")
                        .map(|i| trait_name[i + 2..].to_string())
                        .and_then(|short| self.analyzed_trait_methods.get(&short).cloned())
                });

            if let Some(trait_methods) = trait_methods_opt {
                let mut updated_methods = HashMap::new();

                for (method_name, mut trait_method_analysis) in trait_methods {
                    // Trait receiver is the contract. When any impl exists in this program, derive
                    // `self` **only** from those impls (merge with max-permissive: &mut beats &).
                    // Trait-only crates: receiver from body analysis (&self default).
                    // Impl blocks in this program merge upward via infer_trait_signatures_from_impls.

                    if !trait_method_analysis
                        .inferred_ownership
                        .contains_key("self")
                    {
                        updated_methods.insert(method_name, trait_method_analysis);
                        continue;
                    }

                    let trait_lookup_key = trait_name
                        .rfind("::")
                        .map(|i| &trait_name[i + 2..])
                        .unwrap_or(trait_name.as_str());

                    let trait_method_returns_self = self
                        .trait_definitions
                        .get(trait_lookup_key)
                        .and_then(|decl| decl.methods.iter().find(|m| m.name == method_name))
                        .is_some_and(|m| {
                            matches!(
                                &m.return_type,
                                Some(Type::Custom(name)) if name == "Self"
                            )
                        });

                    if matches!(
                        trait_method_analysis.inferred_ownership.get("self"),
                        Some(OwnershipMode::Owned)
                    ) && trait_method_returns_self
                    {
                        // Consuming `self` only when return mentions Self (builders/consumers).
                        updated_methods.insert(method_name, trait_method_analysis);
                        continue;
                    }
                    let explicit_mut_self_contract = self
                        .trait_definitions
                        .get(trait_lookup_key)
                        .and_then(|decl| decl.methods.iter().find(|m| m.name == method_name))
                        .and_then(|m| m.parameters.iter().find(|p| p.name == "self"))
                        .is_some_and(|p| matches!(p.ownership, OwnershipHint::Mut));
                    if explicit_mut_self_contract {
                        // Rust std / user wrote `&mut self` on the trait — never refine from impls.
                        // Otherwise `infer_trait_signatures_from_impls` can replace `&mut self` with `&self`
                        // (e.g. `Drop::drop` vs Windjammer `fn drop(self)`), causing E0186/E0053.
                        updated_methods.insert(method_name, trait_method_analysis);
                        continue;
                    }

                    let mut merged_from_impls: Option<OwnershipMode> = None;

                    for impl_block in &impl_blocks {
                        for func in &impl_block.functions {
                            if func.name == method_name {
                                let impl_analysis = self.analyze_function_in_impl(
                                    func, impl_block, program, registry,
                                )?;

                                if let Some(&impl_self_ownership) =
                                    impl_analysis.inferred_ownership.get("self")
                                {
                                    merged_from_impls = Some(match merged_from_impls {
                                        None => impl_self_ownership,
                                        Some(acc) => Self::merge_borrow_trait_receivers(
                                            acc,
                                            impl_self_ownership,
                                        ),
                                    });
                                }
                            }
                        }
                    }

                    if let Some(merged_self) = merged_from_impls {
                        let final_self = trait_method_analysis
                            .inferred_ownership
                            .get("self")
                            .copied()
                            .map(|existing| {
                                Self::merge_borrow_trait_receivers(existing, merged_self)
                            })
                            .unwrap_or(merged_self);
                        trait_method_analysis
                            .inferred_ownership
                            .insert("self".to_string(), final_self);
                    }

                    updated_methods.insert(method_name, trait_method_analysis);
                }

                // Store under the impl's trait name and, when qualified, also under the final segment
                // so `analyzed_trait_methods.get("GameLoop")` and `.get("crate::GameLoop")` stay in sync.
                self.analyzed_trait_methods
                    .insert(trait_name.clone(), updated_methods.clone());
                if let Some(pos) = trait_name.rfind("::") {
                    let short = trait_name[pos + 2..].to_string();
                    if short != trait_name {
                        self.analyzed_trait_methods.insert(short, updated_methods);
                    }
                }
            }
        }

        Ok(())
    }

    pub(super) fn analyze_trait_method(
        &mut self,
        func: &FunctionDecl<'ast>,
        registry: &SignatureRegistry,
        trait_name: Option<&str>,
    ) -> Result<AnalyzedFunction<'ast>, String> {
        // Analyze the function normally first
        let mut analyzed = self.analyze_function(func, registry)?;

        // WINDJAMMER PHILOSOPHY: Ownership is a mechanical detail the compiler handles
        // - User writes trait methods without thinking about ownership
        // - Compiler infers optimal ownership from usage
        // - For explicit `&self` or `&mut self`, preserve them (user explicitly requested)
        // - For `self` (inferred), analyze body/implementations and optimize

        let trait_method_returns_self = trait_name.is_some_and(|t| {
            self.trait_definitions
                .get(t)
                .and_then(|decl| decl.methods.iter().find(|m| m.name == func.name))
                .is_some_and(
                    |m| matches!(&m.return_type, Some(Type::Custom(name)) if name == "Self"),
                )
        });

        for param in &func.parameters {
            if param.name == "self" {
                let self_ownership = match &param.ownership {
                    OwnershipHint::Ref => OwnershipMode::Borrowed,
                    OwnershipHint::Mut => OwnershipMode::MutBorrowed,
                    OwnershipHint::Owned => OwnershipMode::Owned,
                    OwnershipHint::Inferred => {
                        if trait_name.is_some() {
                            self.infer_trait_self_receiver(
                                func,
                                registry,
                                trait_method_returns_self,
                            )
                        } else {
                            self.infer_trait_self_receiver_from_body(func, registry)
                        }
                    }
                };
                analyzed
                    .inferred_ownership
                    .insert("self".to_string(), self_ownership);
            } else {
                // Non-self parameters: preserve explicit, infer otherwise
                let ownership = match &param.ownership {
                    OwnershipHint::Ref => OwnershipMode::Borrowed,
                    OwnershipHint::Mut => OwnershipMode::MutBorrowed,
                    OwnershipHint::Owned | OwnershipHint::Inferred => OwnershipMode::Owned,
                };
                analyzed
                    .inferred_ownership
                    .insert(param.name.clone(), ownership);
            }
        }

        // E0053 FIX: Instance methods without `self` in the signature still need a receiver
        // entry so impl matching and codegen can attach &self or &mut self.
        //
        // WINDJAMMER WAY: Omit `self` in source for every instance method (init, update, render).
        // Receiver strength comes from body analysis and impl merging — never from void-return
        // defaults or method names.
        //
        // Associated functions (constructors / `fn make() -> Self`): no receiver in Rust.
        // Detect by: common factory names, `-> Self`, or `-> TraitName` for the trait being defined.
        let has_explicit_self = func.parameters.iter().any(|p| p.name == "self");
        let is_named_constructor = crate::type_classification::is_constructor_name(&func.name);
        let returns_associated_type = matches!(
            &func.return_type,
            Some(Type::Custom(name))
                if name == "Self" || trait_name.is_some_and(|t| t == name.as_str())
        );
        let is_associated_fn =
            !has_explicit_self && (is_named_constructor || returns_associated_type);
        if !analyzed.inferred_ownership.contains_key("self") && !is_associated_fn {
            let receiver = if trait_name.is_some() {
                self.infer_trait_self_receiver(func, registry, trait_method_returns_self)
            } else {
                self.infer_trait_self_receiver_from_body(func, registry)
            };
            analyzed
                .inferred_ownership
                .insert("self".to_string(), receiver);
        }

        Ok(analyzed)
    }

    /// Merge receiver ownership from impls: strongest wins.
    /// `MutBorrowed` beats `Borrowed`; `Owned` only when **both** sides need consumption.
    /// A single impl that consumes `self` for a void trait method (e.g. calling an owned helper)
    /// must not force the trait to `self` — cap at `&mut self` unless the trait returns `Self`.
    pub(crate) fn merge_borrow_trait_receivers(
        a: OwnershipMode,
        b: OwnershipMode,
    ) -> OwnershipMode {
        use OwnershipMode::*;
        match (a, b) {
            (Owned, Owned) => Owned,
            (MutBorrowed, _) | (_, MutBorrowed) => MutBorrowed,
            (Owned, Borrowed) | (Borrowed, Owned) => MutBorrowed,
            _ => Borrowed,
        }
    }
}
