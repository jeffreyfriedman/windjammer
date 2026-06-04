//! Implicit `self` / `&self` / `&mut self` receiver emission for regular functions.

use crate::analyzer::*;
use crate::codegen::rust::self_analysis;
use crate::parser::*;

use super::CodeGenerator;

impl<'ast> CodeGenerator<'ast> {
    /// Extend `params` with implicit receiver when inferred or required by trait/field access.
    pub(in crate::codegen::rust) fn extend_implicit_self_parameters(
        &mut self,
        analyzed: &AnalyzedFunction<'ast>,
        func: &FunctionDecl<'ast>,
        params: &mut Vec<String>,
    ) {
        // Add implicit &self or &mut self for impl block methods that access fields
        // THE WINDJAMMER WAY: Constructors (associated functions) should NOT get self added!
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
                let body_modifies = self.function_modifies_self_or_derived(&analyzed.decl);
                let consumes_self = super::self_analysis::function_consumes_self(&analyzed.decl);
                let self_param = match ownership {
                    OwnershipMode::Borrowed if !self.in_trait_impl && consumes_self => {
                        if body_modifies { "mut self" } else { "self" }
                    }
                    OwnershipMode::Borrowed => {
                        if body_modifies {
                            "&mut self"
                        } else if !self.in_trait_impl && self.current_struct_is_copy() {
                            "self"
                        } else {
                            "&self"
                        }
                    }
                    OwnershipMode::MutBorrowed if !self.in_trait_impl && consumes_self => {
                        if body_modifies { "mut self" } else { "self" }
                    }
                    OwnershipMode::MutBorrowed => "&mut self",
                    OwnershipMode::Owned => {
                        if self.in_trait_impl {
                            "self"
                        } else {
                            self.owned_self_receiver(&analyzed.decl)
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
            let local_bindings = self_analysis::collect_local_bindings(&func.body);
            let ctx = self_analysis::AnalysisContext::with_locals(
                &func.parameters,
                &self.current_struct_fields,
                &local_bindings,
            );
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
                if !self.in_trait_impl && self.current_struct_is_copy() {
                    params.push("self".to_string());
                } else {
                    params.push("&self".to_string());
                    self.inferred_borrowed_params.insert("self".to_string());
                }
            }
        }
    }
}
