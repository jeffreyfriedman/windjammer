//! Formal parameter list emission (excluding implicit `self` receiver).

use std::collections::HashSet;

use crate::analyzer::*;
use crate::parser::*;

use super::CodeGenerator;

impl<'ast> CodeGenerator<'ast> {
    pub(in crate::codegen::rust) fn compute_unused_formal_parameter_names(
        &self,
        func: &FunctionDecl<'ast>,
    ) -> HashSet<String> {
        // TDD FIX: Pre-compute which parameters are actually used in the function body.
        // Unused parameters get prefixed with `_` to suppress "unused variable" warnings.
        // THE WINDJAMMER WAY: The compiler handles this automatically — developers don't
        // need to manually prefix unused parameters with `_`.
        let body_refs: Vec<&Statement> = func.body.to_vec();
        func.parameters
            .iter()
            .filter(|p| p.name != "self")
            .filter(|p| !Self::variable_used_in_statements(&body_refs, &p.name))
            .map(|p| p.name.clone())
            .collect()
    }

    pub(in crate::codegen::rust) fn refresh_unused_let_bindings_for_function_body(
        &mut self,
        body: &[&'ast Statement<'ast>],
    ) {
        // TDD FIX: Pre-compute unused let bindings and for-loop variables.
        // Like unused params, these get prefixed with `_` in the generated Rust.
        self.unused_let_bindings.clear();
        Self::find_unused_bindings(body, &mut self.unused_let_bindings);
    }

    pub(in crate::codegen::rust) fn collect_additional_formal_parameter_strings(
        &mut self,
        analyzed: &AnalyzedFunction<'ast>,
        func: &FunctionDecl<'ast>,
        needs_lifetime: bool,
        unused_params: &HashSet<String>,
    ) -> Vec<String> {
        let body_modifies = if self.in_trait_impl {
            self.function_modifies_self(&analyzed.decl)
        } else {
            self.function_modifies_self_or_derived(&analyzed.decl)
        };
        func.parameters
            .iter()
            .enumerate()
            .map(|(param_idx, param)| {
                // SMART STRING INFERENCE: Use the inferred type from analyzer (string → &str vs String)
                let inferred_type = analyzed
                    .inferred_param_types
                    .get(param_idx)
                    .unwrap_or(&param.type_);

                // E0053: Trait impl formal parameters must match the trait item. Plain `string` in
                // source is owned `String` — do not emit `&str` from str_ref inference on the impl.
                let is_owned_string_decl = matches!(&param.type_, Type::String)
                    || matches!(&param.type_, Type::Custom(name) if name == "string");
                let formal_type: &Type = if self.in_trait_impl
                    && param.name != "self"
                    && is_owned_string_decl
                {
                    &param.type_
                } else {
                    inferred_type
                };
                let trait_impl_owned_string =
                    self.in_trait_impl && param.name != "self" && is_owned_string_decl;

                // PHASE 9 OPTIMIZATION: Check if this parameter should use Cow<'_, T>
                if self.cow_optimizations.contains(&param.name) {
                    let base_type = self.type_to_rust(formal_type);
                    // For String types, use Cow<'_, str>
                    let cow_type = if base_type == "String" {
                        "Cow<'_, str>".to_string()
                    } else {
                        format!("Cow<'_, {}>", base_type)
                    };
                    return format!("{}: {}", param.name, cow_type);
                }

                if param.name == "self"
                    && !self.in_trait_impl
                    && self.function_calls_self_with_recorded_receiver(
                        &analyzed.decl,
                        OwnershipMode::MutBorrowed,
                    )
                {
                    self.inferred_mut_borrowed_params.insert("self".to_string());
                    self.inferred_borrowed_params.remove("self");
                    self.record_self_receiver_upgrade(
                        &func.name,
                        analyzed.inferred_ownership.get("self").copied(),
                        "&mut self",
                    );
                    return "&mut self".to_string();
                }

                // Handle explicit ownership hints (self, &self, &mut self)
                let type_str = match &param.ownership {
                    OwnershipHint::Owned => {
                        if param.name == "self" {
                            let body_modifies = body_modifies;
                            let consumes_self = super::self_analysis::function_consumes_self(&analyzed.decl)
                                || super::self_analysis::function_return_moves_self_fields(&analyzed.decl);
                            let eff_ownership =
                                self.get_effective_self_ownership(&func.name, analyzed);
                            let self_str = if let Some(ownership_mode) = eff_ownership {
                                match ownership_mode {
                                    OwnershipMode::Borrowed | OwnershipMode::MutBorrowed
                                        if !self.in_trait_impl
                                            && (self.method_returns_impl_struct(func) || consumes_self) =>
                                    {
                                        if body_modifies { "mut self" } else { "self" }
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
                                        if self.in_trait_impl {
                                            if body_modifies {
                                                "mut self"
                                            } else {
                                                "self"
                                            }
                                        } else {
                                            self.owned_self_receiver(&analyzed.decl)
                                        }
                                    }
                                }
                            } else {
                                if self.in_trait_impl {
                                    "self"
                                } else {
                                    self.owned_self_receiver(&analyzed.decl)
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
                            self.record_self_receiver_upgrade(
                                &func.name,
                                eff_ownership,
                                self_str,
                            );
                            return self_str.to_string();
                        }
                        // Owned parameters are always mutable in Windjammer
                        return format!("mut {}: {}", param.name, self.type_to_rust(formal_type));
                    }
                    OwnershipHint::Ref => {
                        if param.name == "self" {
                            let body_modifies = body_modifies;
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
                            formal_type,
                            Type::Reference(_) | Type::MutableReference(_)
                        ) {
                            self.type_to_rust(formal_type)
                        } else {
                            // TDD FIX: Copy types pass by value even with Ref hint
                            if self.is_type_copy(formal_type) {
                                self.type_to_rust(formal_type)
                            } else {
                                // TDD FIX: Borrowed → &T (including &String for strings)
                                // Correctness > idioms: &String works with Vec<String> methods
                                format!("&{}", self.type_to_rust(formal_type))
                            }
                        }
                    }
                    OwnershipHint::Mut => {
                        if param.name == "self" {
                            let body_modifies = body_modifies;
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
                                        if self.in_trait_impl {
                                            "self".to_string()
                                        } else {
                                            self.owned_self_receiver(&analyzed.decl).to_string()
                                        }
                                    }
                                };
                            }
                            return "&mut self".to_string();
                        }
                        // Don't add &mut if the type is already a MutableReference
                        if matches!(formal_type, Type::MutableReference(_)) {
                            self.type_to_rust(formal_type)
                        } else {
                            format!("&mut {}", self.type_to_rust(formal_type))
                        }
                    }
                    OwnershipHint::Inferred => {
                        if param.name == "self" {
                            let body_modifies = body_modifies;
                            let returns_self = self.method_returns_impl_struct(&analyzed.decl);
                            let consumes_self = super::self_analysis::function_consumes_self(&analyzed.decl)
                                || super::self_analysis::function_return_moves_self_fields(&analyzed.decl);
                            let self_str = if let Some(ownership_mode) =
                                self.get_effective_self_ownership(&func.name, analyzed)
                            {
                                match ownership_mode {
                                    OwnershipMode::Borrowed | OwnershipMode::MutBorrowed
                                        if !self.in_trait_impl && (returns_self || consumes_self) =>
                                    {
                                        if body_modifies { "mut self" } else { "self" }
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
                                        if self.in_trait_impl {
                                            if body_modifies {
                                                "mut self"
                                            } else {
                                                "self"
                                            }
                                        } else {
                                            self.owned_self_receiver(&analyzed.decl)
                                        }
                                    }
                                }
                            } else if body_modifies && returns_self {
                                "mut self"
                            } else if consumes_self {
                                "self"
                            } else if body_modifies {
                                "&mut self"
                            } else if returns_self {
                                "self"
                            } else {
                                "&self"
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
                            let eff = self.get_effective_self_ownership(&func.name, analyzed);
                            self.record_self_receiver_upgrade(
                                &func.name,
                                eff,
                                self_str,
                            );
                            return self_str.to_string();
                        }

                        // Check if type already has ownership baked in (like &str from string inference)
                        if matches!(
                            formal_type,
                            Type::Reference(_) | Type::MutableReference(_)
                        ) {
                            // Already has & or &mut - just convert
                            self.type_to_rust(formal_type)
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
                            let ownership_mode = if trait_impl_owned_string {
                                &OwnershipMode::Owned
                            } else if self.in_trait_impl {
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
                                                trait_fn.inferred_ownership.get(&param.name)
                                            })
                                        })
                                    });
                                trait_param_ownership.unwrap_or(&OwnershipMode::Owned)
                            } else {
                                ownership_mode
                            };

                            match ownership_mode {
                                OwnershipMode::Owned => self.type_to_rust(formal_type),
                                OwnershipMode::Borrowed | OwnershipMode::MutBorrowed
                                    if self.is_type_copy(formal_type) =>
                                {
                                    self.type_to_rust(formal_type)
                                }
                                OwnershipMode::Borrowed => {
                                    let is_string = matches!(formal_type, Type::String)
                                        || matches!(formal_type, Type::Custom(ref name) if name == "string");
                                    // Impl read-only `string` → `&str` (HashSet::contains(name), not &name).
                                    if is_string && !trait_impl_owned_string {
                                        "&str".to_string()
                                    } else {
                                        format!("&{}", self.type_to_rust(formal_type))
                                    }
                                }
                                OwnershipMode::MutBorrowed => {
                                    format!("&mut {}", self.type_to_rust(formal_type))
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

                // Copy owned formals pass by value — clear stale borrow metadata from body analysis.
                if self.is_type_copy(formal_type) && !type_str.starts_with('&') {
                    self.inferred_borrowed_params.remove(&param.name);
                    self.inferred_mut_borrowed_params.remove(&param.name);
                }

                // TDD FIX: Auto-infer `mut` for owned parameters
                // THE WINDJAMMER WAY: Users don't track mutability - the compiler does.
                // If a parameter has mutating method calls or field mutations,
                // the binding needs `mut` even if not explicitly written.
                let auto_needs_mut = param.name != "self"
                    && !param.is_mutable
                    && matches!(type_str.as_str(), s if !s.starts_with("&"))
                    && !self.is_type_copy(formal_type)
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
            .collect()
    }
}
