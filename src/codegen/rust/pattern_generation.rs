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
                            .map(|(name, pat)| match pat {
                                Pattern::Identifier(binding) if binding == name => name.clone(),
                                Pattern::MutBinding(binding) if binding == name => {
                                    format!("mut {}", name)
                                }
                                _ => format!("{}: {}", name, self.pattern_to_rust(pat)),
                            })
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
        self.generate_pattern_with_scrutinee(pattern, None)
    }

    /// Qualify bare unit/variant patterns against the match scrutinee enum type
    /// (`Home` → `Route::Home`) so rustc does not treat them as bindings (E0170).
    pub(crate) fn generate_pattern_with_scrutinee(
        &self,
        pattern: &Pattern,
        scrutinee_ty: Option<&Type>,
    ) -> String {
        use crate::parser::EnumPatternBinding;
        match pattern {
            Pattern::Wildcard => "_".to_string(),
            Pattern::Identifier(name) => {
                self.qualify_enum_variant_path(name, scrutinee_ty)
            }
            Pattern::MutBinding(name) => format!("mut {}", name),
            Pattern::Reference(inner) => {
                format!("&{}", self.generate_pattern_with_scrutinee(inner, scrutinee_ty))
            }
            Pattern::Ref(name) => format!("ref {}", name),
            Pattern::RefMut(name) => format!("ref mut {}", name),
            Pattern::EnumVariant(name, binding) => {
                let qualified = self.qualify_enum_variant_path(name, scrutinee_ty);
                match binding {
                    EnumPatternBinding::Single(b) => format!("{}({})", qualified, b),
                    EnumPatternBinding::Wildcard => format!("{}(_)", qualified),
                    EnumPatternBinding::None => qualified,
                    EnumPatternBinding::Tuple(patterns) => {
                        let rust_patterns: Vec<String> = patterns
                            .iter()
                            .map(|p| self.generate_pattern_with_scrutinee(p, None))
                            .collect();
                        format!("{}({})", qualified, rust_patterns.join(", "))
                    }
                    EnumPatternBinding::Struct(fields, has_wildcard) => {
                        if fields.is_empty() {
                            format!("{} {{ .. }}", qualified)
                        } else {
                            let field_strs: Vec<String> = fields
                                .iter()
                                .map(|(n, pat)| match pat {
                                    Pattern::Identifier(binding) if binding == n => n.clone(),
                                    Pattern::MutBinding(binding) if binding == n => {
                                        format!("mut {}", n)
                                    }
                                    _ => format!(
                                        "{}: {}",
                                        n,
                                        self.generate_pattern_with_scrutinee(pat, None)
                                    ),
                                })
                                .collect();
                            if *has_wildcard {
                                format!("{} {{ {}, .. }}", qualified, field_strs.join(", "))
                            } else {
                                format!("{} {{ {} }}", qualified, field_strs.join(", "))
                            }
                        }
                    }
                }
            }
            Pattern::Literal(lit) => self.generate_literal(lit),
            Pattern::Tuple(patterns) => {
                let pattern_strs: Vec<String> = patterns
                    .iter()
                    .map(|p| self.generate_pattern_with_scrutinee(p, None))
                    .collect();
                format!("({})", pattern_strs.join(", "))
            }
            Pattern::Or(patterns) => {
                let pattern_strs: Vec<String> = patterns
                    .iter()
                    .map(|p| self.generate_pattern_with_scrutinee(p, scrutinee_ty))
                    .collect();
                pattern_strs.join(" | ")
            }
        }
    }

    /// When `variant` is an unqualified enum variant matching `scrutinee_ty`, emit `Enum::Variant`.
    fn qualify_enum_variant_path(&self, variant: &str, scrutinee_ty: Option<&Type>) -> String {
        if variant.contains("::") {
            return variant.to_string();
        }
        // Only PascalCase identifiers are candidate unit/enum variants (not bindings).
        // SCREAMING_SNAKE consts (`TAG_NULL`) are not enum variants — leave unqualified.
        let mut chars = variant.chars();
        let Some(first) = chars.next() else {
            return variant.to_string();
        };
        if !first.is_ascii_uppercase() {
            return variant.to_string();
        }
        if variant.contains('_') && variant.chars().all(|c| c.is_ascii_uppercase() || c == '_') {
            return variant.to_string();
        }
        let Some(ty) = scrutinee_ty else {
            return variant.to_string();
        };
        let inner = match ty {
            Type::Reference(inner) | Type::MutableReference(inner) => inner.as_ref(),
            other => other,
        };
        // Option/Result std paths stay unqualified (`Some`, `Ok`, `Err`, `None`).
        if matches!(inner, Type::Option(_) | Type::Result(_, _))
            || matches!(
                inner,
                Type::Custom(n)
                    if n == "Option"
                        || n == "Result"
                        || n.ends_with("::Option")
                        || n.ends_with("::Result")
            )
        {
            return variant.to_string();
        }
        let Some(key) = self.enum_pattern_registry_key(variant, inner) else {
            return variant.to_string();
        };
        // Only rewrite when this is a known user enum variant — never invent
        // `u8::TAG_NULL` / primitive associated paths for const patterns.
        if self.enum_variant_types.contains_key(&key)
            || self.enum_variant_struct_fields.contains_key(&key)
        {
            return key;
        }
        let enum_name = match inner {
            Type::Custom(n) => n.as_str(),
            Type::Parameterized(n, _) => n.as_str(),
            _ => return variant.to_string(),
        };
        let prefix = format!("{enum_name}::");
        let known_enum = self
            .enum_variant_types
            .keys()
            .any(|k| k.starts_with(&prefix))
            || self
                .enum_variant_struct_fields
                .keys()
                .any(|k| k.starts_with(&prefix));
        if known_enum {
            return key;
        }
        variant.to_string()
    }

    pub(super) fn extract_pattern_bindings(
        &self,
        pattern: &Pattern,
        bindings: &mut std::collections::HashSet<String>,
    ) {
        use crate::parser::EnumPatternBinding;
        match pattern {
            Pattern::Identifier(name) | Pattern::MutBinding(name) => {
                // Unit enum variants (`None`, `Empty`, …) parse as Identifier; they are
                // not variable bindings. Treating them as bindings makes match arms emit
                // `None.clone()` when the scrutinee is behind a reference (E0614-ish noise /
                // nonsense for Option::None).
                if name
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_uppercase())
                {
                    // not a binding
                } else {
                    bindings.insert(name.clone());
                }
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
            body_stmts
                .iter()
                .any(|stmt| self.statement_nonreadonly_method_call_on_var(stmt, name))
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
            Expression::MethodCall { method, object, .. } => {
                // .get() on HashMap/BTreeMap always returns Option<&V> in Rust.
                // Check object type via both type inference and struct field lookup.
                if method == "get" {
                    if let Some(obj_ty) = self.infer_expression_type(object) {
                        if Self::is_hashmap_like_type(&obj_ty) {
                            return true;
                        }
                    }
                    // Fallback: check struct field types when object is self.field
                    if let Expression::FieldAccess {
                        object: fa_obj,
                        field: fa_field,
                        ..
                    } = object
                    {
                        if let Expression::Identifier { name, .. } = fa_obj {
                            if name == "self" {
                                if let Some(ft) = self.get_struct_field_type(fa_field) {
                                    if Self::is_hashmap_like_type(&ft) {
                                        return true;
                                    }
                                }
                            }
                        }
                    }
                }
                if let Some(ty) = self.infer_expression_type(expr) {
                    if matches!(ty, Type::Reference(_) | Type::MutableReference(_)) {
                        return true;
                    }
                    if method == "get" {
                        if let Type::Option(inner) = &ty {
                            if matches!(
                                inner.as_ref(),
                                Type::Reference(_) | Type::MutableReference(_)
                            ) {
                                return true;
                            }
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
                    if let Expression::FieldAccess {
                        field,
                        object: fa_obj,
                        ..
                    } = function
                    {
                        if field == "get" {
                            if let Type::Option(inner) = &ty {
                                if matches!(
                                    inner.as_ref(),
                                    Type::Reference(_) | Type::MutableReference(_)
                                ) {
                                    return true;
                                }
                            }
                            if let Some(obj_ty) = self.infer_expression_type(fa_obj) {
                                if Self::is_hashmap_like_type(&obj_ty) {
                                    return true;
                                }
                            }
                        }
                    }
                }
                false
            }
            _ => false,
        }
    }

    fn get_struct_field_type(&self, field_name: &str) -> Option<Type> {
        let struct_name = self.current_struct_name.as_ref()?;
        let field_types = self.struct_field_types.get(struct_name)?;
        field_types.get(field_name).cloned()
    }

    fn is_hashmap_like_type(ty: &Type) -> bool {
        match ty {
            Type::Custom(name) => {
                // Bare "Map" without type params could be a user struct, not a HashMap alias
                matches!(name.as_str(), "HashMap" | "BTreeMap" | "IndexMap")
            }
            Type::Parameterized(name, _) => {
                matches!(name.as_str(), "HashMap" | "BTreeMap" | "Map" | "IndexMap")
            }
            _ => false,
        }
    }
}
