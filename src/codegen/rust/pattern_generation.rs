//! Pattern code generation
//!
//! Handles code generation for patterns in let bindings, match arms, and for loops including:
//! - Wildcard patterns
//! - Identifier patterns
//! - Mutable bindings
//! - Reference patterns
//! - Tuple patterns
//! - Struct patterns
//! - Enum patterns
//! - Or patterns

use crate::parser::*;

use super::CodeGenerator;

impl<'ast> CodeGenerator<'ast> {
    pub(in crate::codegen::rust) fn pattern_to_rust(&self, pattern: &Pattern) -> String {
        use crate::parser::EnumPatternBinding;
        match pattern {
            Pattern::Wildcard => "_".to_string(),
            Pattern::Identifier(name) => name.clone(),
            Pattern::MutBinding(name) => format!("mut {}", name),
            Pattern::Reference(inner) => format!("&{}", self.pattern_to_rust(inner)),
            Pattern::Ref(name) => format!("ref {}", name),
            Pattern::RefMut(name) => format!("ref mut {}", name),
            Pattern::Tuple(patterns) => {
                let rust_patterns: Vec<String> =
                    patterns.iter().map(|p| self.pattern_to_rust(p)).collect();
                format!("({})", rust_patterns.join(", "))
            }
            Pattern::EnumVariant(variant, binding) => match binding {
                EnumPatternBinding::Single(name) => format!("{}({})", variant, name),
                EnumPatternBinding::Wildcard => format!("{}(_)", variant),
                EnumPatternBinding::None => variant.clone(),
                EnumPatternBinding::Tuple(patterns) => {
                    let rust_patterns: Vec<String> =
                        patterns.iter().map(|p| self.pattern_to_rust(p)).collect();
                    format!("{}({})", variant, rust_patterns.join(", "))
                }
                EnumPatternBinding::Struct(fields, has_wildcard) => {
                    if fields.is_empty() {
                        format!("{} {{ .. }}", variant)
                    } else {
                        let field_strs: Vec<String> = fields
                            .iter()
                            .map(|(name, pat)| format!("{}: {}", name, self.pattern_to_rust(pat)))
                            .collect();
                        if *has_wildcard {
                            format!("{} {{ {}, .. }}", variant, field_strs.join(", "))
                        } else {
                            format!("{} {{ {} }}", variant, field_strs.join(", "))
                        }
                    }
                }
            },
            Pattern::Literal(lit) => self.generate_literal(lit),
            Pattern::Or(patterns) => {
                let rust_patterns: Vec<String> =
                    patterns.iter().map(|p| self.pattern_to_rust(p)).collect();
                rust_patterns.join(" | ")
            }
        }
    }

    pub(crate) fn generate_pattern(&self, pattern: &Pattern) -> String {
        use crate::parser::EnumPatternBinding;
        match pattern {
            Pattern::Wildcard => "_".to_string(),
            Pattern::Identifier(name) => name.clone(),
            Pattern::MutBinding(name) => format!("mut {}", name),
            Pattern::Reference(inner) => format!("&{}", self.generate_pattern(inner)),
            Pattern::Ref(name) => format!("ref {}", name),
            Pattern::RefMut(name) => format!("ref mut {}", name),
            Pattern::EnumVariant(name, binding) => match binding {
                EnumPatternBinding::Single(b) => format!("{}({})", name, b),
                EnumPatternBinding::Wildcard => format!("{}(_)", name),
                EnumPatternBinding::None => name.clone(),
                EnumPatternBinding::Tuple(patterns) => {
                    let rust_patterns: Vec<String> =
                        patterns.iter().map(|p| self.generate_pattern(p)).collect();
                    format!("{}({})", name, rust_patterns.join(", "))
                }
                EnumPatternBinding::Struct(fields, has_wildcard) => {
                    if fields.is_empty() {
                        format!("{} {{ .. }}", name)
                    } else {
                        let field_strs: Vec<String> = fields
                            .iter()
                            .map(|(n, pat)| {
                                if let Pattern::Identifier(binding) = pat {
                                    if binding == n {
                                        return n.clone();
                                    }
                                }
                                format!("{}: {}", n, self.generate_pattern(pat))
                            })
                            .collect();
                        if *has_wildcard {
                            format!("{} {{ {}, .. }}", name, field_strs.join(", "))
                        } else {
                            format!("{} {{ {} }}", name, field_strs.join(", "))
                        }
                    }
                }
            },
            Pattern::Literal(lit) => self.generate_literal(lit),
            Pattern::Tuple(patterns) => {
                let pattern_strs: Vec<String> =
                    patterns.iter().map(|p| self.generate_pattern(p)).collect();
                format!("({})", pattern_strs.join(", "))
            }
            Pattern::Or(patterns) => {
                let pattern_strs: Vec<String> =
                    patterns.iter().map(|p| self.generate_pattern(p)).collect();
                pattern_strs.join(" | ")
            }
        }
    }

    pub(super) fn extract_pattern_bindings(
        &self,
        pattern: &Pattern,
        bindings: &mut std::collections::HashSet<String>,
    ) {
        use crate::parser::EnumPatternBinding;
        match pattern {
            Pattern::Identifier(name) | Pattern::MutBinding(name) => {
                bindings.insert(name.clone());
            }
            Pattern::Reference(inner) => {
                self.extract_pattern_bindings(inner, bindings);
            }
            Pattern::Ref(name) | Pattern::RefMut(name) => {
                bindings.insert(name.clone());
            }
            Pattern::EnumVariant(_name, binding) => match binding {
                EnumPatternBinding::Single(var_name) => {
                    bindings.insert(var_name.clone());
                }
                EnumPatternBinding::Tuple(patterns) => {
                    for pat in patterns {
                        self.extract_pattern_bindings(pat, bindings);
                    }
                }
                EnumPatternBinding::Struct(fields, _) => {
                    for (_field_name, pat) in fields {
                        self.extract_pattern_bindings(pat, bindings);
                    }
                }
                _ => {}
            },
            Pattern::Tuple(patterns) => {
                for pat in patterns {
                    self.extract_pattern_bindings(pat, bindings);
                }
            }
            _ => {}
        }
    }

    pub(super) fn upgrade_pattern_mut_bindings<'s>(
        &self,
        pattern: &Pattern<'s>,
        body_stmts: &[&Statement<'s>],
        scrutinee_is_ref: bool,
        body_expr: Option<&Expression<'s>>,
        scrutinee: Option<&Expression<'s>>,
    ) -> Pattern<'s> {
        use crate::parser::EnumPatternBinding;
        let binding_type_for = |name: &str| -> Option<Type> {
            scrutinee.and_then(|scr| {
                self.infer_match_bound_types_owned(scr, pattern)
                    .into_iter()
                    .find(|(n, _)| n == name)
                    .map(|(_, t)| t)
                    .or_else(|| {
                        self.infer_match_bound_types(scr, pattern)
                            .into_iter()
                            .find(|(n, _)| n == name)
                            .map(|(_, t)| t)
                    })
            })
        };
        let binding_needs_mut = |name: &str| -> bool {
            let field_mut = body_stmts
                .iter()
                .any(|stmt| self.statement_mutates_variable_field(stmt, name));
            if field_mut {
                return true;
            }
            if let (Some(body), Some(ty)) = (body_expr, binding_type_for(name)) {
                if self.binding_receives_mutating_call_with_sig_check(body, name, &ty) {
                    return true;
                }
            }
            body_stmts.iter().any(|stmt| {
                self.statement_nonreadonly_method_call_on_var(stmt, name)
            })
        };
        match pattern {
            Pattern::Identifier(name) => {
                if binding_needs_mut(name) {
                    if scrutinee_is_ref {
                        Pattern::RefMut(name.clone())
                    } else {
                        Pattern::MutBinding(name.clone())
                    }
                } else {
                    pattern.clone()
                }
            }
            Pattern::EnumVariant(variant, binding) => {
                let new_binding = match binding {
                    EnumPatternBinding::Single(name) => {
                        if binding_needs_mut(name) {
                            if scrutinee_is_ref {
                                EnumPatternBinding::Tuple(vec![Pattern::RefMut(name.clone())])
                            } else {
                                EnumPatternBinding::Tuple(vec![Pattern::MutBinding(name.clone())])
                            }
                        } else {
                            binding.clone()
                        }
                    }
                    EnumPatternBinding::Tuple(patterns) => {
                        let new_patterns: Vec<Pattern<'s>> = patterns
                            .iter()
                            .map(|p| {
                                self.upgrade_pattern_mut_bindings(
                                    p,
                                    body_stmts,
                                    scrutinee_is_ref,
                                    body_expr,
                                    scrutinee,
                                )
                            })
                            .collect();
                        EnumPatternBinding::Tuple(new_patterns)
                    }
                    other => other.clone(),
                };
                Pattern::EnumVariant(variant.clone(), new_binding)
            }
            Pattern::Tuple(patterns) => {
                let new_patterns: Vec<Pattern<'s>> = patterns
                    .iter()
                    .map(|p| {
                        self.upgrade_pattern_mut_bindings(
                            p,
                            body_stmts,
                            scrutinee_is_ref,
                            body_expr,
                            scrutinee,
                        )
                    })
                    .collect();
                Pattern::Tuple(new_patterns)
            }
            _ => pattern.clone(),
        }
    }

    pub(super) fn match_expression_binds_refs(&self, expr: &Expression) -> bool {
        match expr {
            Expression::Identifier { name, .. } => {
                if let Some(ty) = self.local_var_types.get(name) {
                    return matches!(ty, Type::Reference(_) | Type::MutableReference(_));
                }
                if name == "self" && self.inferred_borrowed_params.contains("self") {
                    return true;
                }
                false
            }
            Expression::FieldAccess { object, .. } => {
                if let Expression::Identifier { name: obj_name, .. } = &**object {
                    if obj_name == "self" {
                        return self.current_function_params.iter().any(|p| {
                            p.name == "self"
                                && matches!(
                                    p.ownership,
                                    crate::parser::OwnershipHint::Ref
                                        | crate::parser::OwnershipHint::Mut
                                )
                        });
                    }
                }
                false
            }
            Expression::Index { object, .. } => {
                // Vec/array indexing can return references
                if let Some(ty) = self.infer_expression_type(object) {
                    matches!(ty, Type::Vec(_) | Type::Array(_, _))
                } else {
                    false
                }
            }
            Expression::MethodCall { method, .. } => {
                if let Some(ty) = self.infer_expression_type(expr) {
                    if matches!(ty, Type::Reference(_) | Type::MutableReference(_)) {
                        return true;
                    }
                    // HashMap.get()/BTreeMap.get() returns Option<&V>.
                    // Custom methods like Map::get -> Option<i32> return owned values.
                    if method == "get" {
                        if let Type::Option(inner) = ty {
                            return matches!(
                                inner.as_ref(),
                                Type::Reference(_) | Type::MutableReference(_)
                            );
                        }
                    }
                }
                false
            }
            Expression::Call { function, .. } => {
                if let Some(ty) = self.infer_expression_type(expr) {
                    if matches!(ty, Type::Reference(_) | Type::MutableReference(_)) {
                        return true;
                    }
                    if let Expression::FieldAccess { field, .. } = function {
                        if field == "get" {
                            if let Type::Option(inner) = ty {
                                return matches!(
                                    inner.as_ref(),
                                    Type::Reference(_) | Type::MutableReference(_)
                                );
                            }
                        }
                    }
                }
                false
            }
            _ => false,
        }
    }
}
