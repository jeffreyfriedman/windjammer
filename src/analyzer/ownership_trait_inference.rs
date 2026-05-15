//! Trait receiver ownership: merging impls, default trait method analysis, and hint conversion.

use crate::parser::*;

use super::{AnalyzedFunction, Analyzer, OwnershipHint, OwnershipMode, SignatureRegistry};

impl<'ast> Analyzer<'ast> {
    /// Analyze a trait method with default implementation
    /// Trait methods must use &self or &mut self (not owned self)
    /// to work with unsized types
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
                    // Trait-only crates keep `analyze_trait_method` defaults (abstract → &mut self).
                    //
                    // Associated functions (`fn create() -> Self`) have no `self` entry — skip.

                    if !trait_method_analysis
                        .inferred_ownership
                        .contains_key("self")
                    {
                        updated_methods.insert(method_name, trait_method_analysis);
                        continue;
                    }

                    if matches!(
                        trait_method_analysis.inferred_ownership.get("self"),
                        Some(OwnershipMode::Owned)
                    ) {
                        // Consuming `self` on the trait is authoritative; do not refine from impls.
                        updated_methods.insert(method_name, trait_method_analysis);
                        continue;
                    }

                    let trait_lookup_key = trait_name
                        .rfind("::")
                        .map(|i| &trait_name[i + 2..])
                        .unwrap_or(trait_name.as_str());
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
                                let empty_registry = SignatureRegistry::new();
                                let impl_analysis = self.analyze_function_in_impl(
                                    func,
                                    impl_block,
                                    program,
                                    &empty_registry,
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
                        trait_method_analysis
                            .inferred_ownership
                            .insert("self".to_string(), merged_self);
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

        for param in &func.parameters {
            if param.name == "self" {
                match &param.ownership {
                    OwnershipHint::Ref => {
                        // User explicitly wrote &self - preserve it
                        analyzed
                            .inferred_ownership
                            .insert("self".to_string(), OwnershipMode::Borrowed);
                    }
                    OwnershipHint::Mut => {
                        // User explicitly wrote &mut self - preserve it
                        analyzed
                            .inferred_ownership
                            .insert("self".to_string(), OwnershipMode::MutBorrowed);
                    }
                    OwnershipHint::Owned => {
                        // Explicit consuming `self` in the trait (e.g. fn consume(self) -> T)
                        analyzed
                            .inferred_ownership
                            .insert("self".to_string(), OwnershipMode::Owned);
                    }
                    OwnershipHint::Inferred => {
                        // Omitted receiver: infer from trait body. Abstract: void → &mut self; with return → &self.
                        let modifies_self =
                            self.function_modifies_self_fields_with_registry(func, Some(registry));
                        let self_ownership = if modifies_self {
                            OwnershipMode::MutBorrowed
                        } else if func.body.is_empty() {
                            if func.return_type.is_some() {
                                OwnershipMode::Borrowed
                            } else {
                                OwnershipMode::MutBorrowed
                            }
                        } else {
                            OwnershipMode::Borrowed
                        };
                        analyzed
                            .inferred_ownership
                            .insert("self".to_string(), self_ownership);
                    }
                }
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

        // E0053 FIX: Trait methods without explicit self (e.g. fn initialize()) need self for impl matching.
        // Windjammer trait methods often omit self - add default so infer_trait_signatures_from_impls can upgrade.
        //
        // Associated functions (constructors / `fn make() -> MyTrait`): no receiver in Rust.
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
            let default_receiver = if func.return_type.is_some() {
                OwnershipMode::Borrowed
            } else {
                OwnershipMode::MutBorrowed
            };
            analyzed
                .inferred_ownership
                .insert("self".to_string(), default_receiver);
        }

        Ok(analyzed)
    }

    /// Merge receiver ownership from impls: strongest wins.
    /// `Owned` (consuming) > `MutBorrowed` (&mut self) > `Borrowed` (&self).
    fn merge_borrow_trait_receivers(a: OwnershipMode, b: OwnershipMode) -> OwnershipMode {
        use OwnershipMode::*;
        match (a, b) {
            (Owned, _) | (_, Owned) => Owned,
            (MutBorrowed, _) | (_, MutBorrowed) => MutBorrowed,
            _ => Borrowed,
        }
    }

    /// Look up the analyzed receiver (`self`) ownership for a trait method (trait is the contract).
    pub(super) fn trait_method_receiver_ownership(
        &self,
        trait_name: &str,
        trait_key: &str,
        method_name: &str,
    ) -> Option<OwnershipMode> {
        for key in [trait_key, trait_name] {
            if let Some(methods) = self.analyzed_trait_methods.get(key) {
                if let Some(trait_fn) = methods.get(method_name) {
                    return trait_fn.inferred_ownership.get("self").copied();
                }
            }
        }
        None
    }
}
