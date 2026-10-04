//! Infers `match` / `if let` pattern binding types.

use crate::codegen::rust::CodeGenerator;
use crate::parser::{EnumPatternBinding, Expression, MatchArm, Pattern, Statement, Type};
use std::collections::HashMap;

impl<'ast> CodeGenerator<'ast> {
    pub(in crate::codegen::rust) fn match_scrutinee_yields_ref_enum_bindings(
        &self,
        scrutinee: &Expression,
    ) -> bool {
        match scrutinee {
            Expression::Unary {
                op: crate::parser::UnaryOp::Ref | crate::parser::UnaryOp::MutRef,
                ..
            } => true,
            Expression::Index { object, .. } => {
                let Some(obj_ty) = self.infer_expression_type(object) else {
                    return false;
                };
                let Some(elem) = Self::peeled_collection_element_type(&obj_ty) else {
                    return false;
                };
                !self.is_type_copy(elem)
            }
            _ => false,
        }
    }

    pub(in crate::codegen::rust) fn enum_pattern_registry_key(
        &self,
        variant_name: &str,
        enum_container: &Type,
    ) -> Option<String> {
        if variant_name.contains("::") {
            Some(variant_name.to_string())
        } else {
            let en = match enum_container {
                Type::Custom(n) => n.as_str(),
                Type::Parameterized(n, _) => n.as_str(),
                _ => return None,
            };
            Some(format!("{}::{}", en, variant_name))
        }
    }

    /// Infer the types of variables bound in match arm patterns.
    /// When matching `Some(x)` on `opt: Option<Stack>`, returns [("x", Type::Custom("Stack"))].
    /// When matching `Variant { a, b }` on `&vec[i]` with non-Copy elements, fields bind as `&FieldTy`.
    pub(in crate::codegen::rust) fn infer_match_bound_types(
        &self,
        scrutinee: &Expression,
        pattern: &Pattern,
    ) -> Vec<(String, Type)> {
        let yields_refs = self.match_scrutinee_yields_ref_enum_bindings(scrutinee);
        self.infer_match_bound_types_with_ref_mode(scrutinee, pattern, yields_refs)
    }

    /// After owned borrow-break (`let __v = expr.clone(); match __v`), bindings are owned `T`.
    pub(in crate::codegen::rust) fn infer_match_bound_types_owned(
        &self,
        scrutinee: &Expression,
        pattern: &Pattern,
    ) -> Vec<(String, Type)> {
        self.infer_match_bound_types_with_ref_mode(scrutinee, pattern, false)
    }

    fn infer_match_bound_types_with_ref_mode(
        &self,
        scrutinee: &Expression,
        pattern: &Pattern,
        yields_refs: bool,
    ) -> Vec<(String, Type)> {
        let scrutinee_type = match self.infer_expression_type(scrutinee) {
            Some(t) => t,
            None => {
                // Cross-module `use crate::handle::Data` may not have converged
                // the scrutinee type; `Data::Arrow(handle)` still keys the
                // global enum-variant registry.
                return self.infer_match_bound_types_from_pattern_key(pattern, yields_refs);
            }
        };

        let inner_type = match &scrutinee_type {
            Type::Reference(inner) | Type::MutableReference(inner) => inner.as_ref().clone(),
            _ => scrutinee_type.clone(),
        };

        let mut out = Vec::new();

        // `Ok(mut app)` / `Some(mut x)` parse as Tuple([MutBinding]) — not Single —
        // because MutBinding is not a plain Identifier. Still bind Option/Result
        // payloads so method receivers resolve (P3.621 / MutexGuard → demoted &str).
        if let Pattern::EnumVariant(variant, binding) = pattern {
            let single_name = match binding {
                EnumPatternBinding::Single(name) => Some(name.as_str()),
                EnumPatternBinding::Tuple(pats) if pats.len() == 1 => {
                    Self::pattern_simple_binding_name(&pats[0])
                }
                _ => None,
            };
            if let Some(var_name) = single_name {
                let payload = if variant == "Some" || variant.ends_with("::Some") {
                    match &inner_type {
                        Type::Option(inner_t) => Some(inner_t.as_ref().clone()),
                        _ => None,
                    }
                } else if variant == "Ok" || variant.ends_with("::Ok") {
                    match &inner_type {
                        Type::Result(ok, _) => Some(ok.as_ref().clone()),
                        _ => None,
                    }
                } else if variant == "Err" || variant.ends_with("::Err") {
                    match &inner_type {
                        Type::Result(_, err) => Some(err.as_ref().clone()),
                        _ => None,
                    }
                } else {
                    None
                };
                if let Some(ty) = payload {
                    if yields_refs {
                        out.push((var_name.to_string(), Type::Reference(Box::new(ty))));
                    } else {
                        out.push((var_name.to_string(), ty));
                    }
                    return out;
                }
            }
        }

        match pattern {
            Pattern::EnumVariant(variant_name, EnumPatternBinding::Struct(fields, _)) => {
                let Some(key) = self.enum_pattern_registry_key(variant_name, &inner_type) else {
                    return out;
                };
                let Some(named) = self.enum_variant_struct_fields.get(&key) else {
                    return out;
                };
                let map: HashMap<String, Type> = named.iter().cloned().collect();
                for (fname, pat) in fields.iter() {
                    if let Some(binding_name) = Self::pattern_simple_binding_name(pat) {
                        if let Some(ft) = map.get(fname) {
                            if yields_refs {
                                out.push((
                                    binding_name.to_string(),
                                    Type::Reference(Box::new(ft.clone())),
                                ));
                            } else {
                                out.push((binding_name.to_string(), ft.clone()));
                            }
                        }
                    }
                }
            }
            // Single-field tuple variants use EnumPatternBinding::Single (e.g. Cost::Gold(amount)).
            // Tuple(..) is only used when the inner pattern is not a plain identifier.
            Pattern::EnumVariant(variant_name, EnumPatternBinding::Single(var_name)) => {
                let types = self
                    .enum_pattern_registry_key(variant_name, &inner_type)
                    .and_then(|key| self.lookup_enum_variant_payload_types(&key).cloned())
                    .or_else(|| {
                        self.lookup_enum_variant_payload_types(variant_name)
                            .cloned()
                    });
                let Some(types) = types else {
                    return out;
                };
                if types.len() == 1 {
                    let ty = &types[0];
                    if yields_refs {
                        out.push((var_name.clone(), Type::Reference(Box::new(ty.clone()))));
                    } else {
                        out.push((var_name.clone(), ty.clone()));
                    }
                }
            }
            // TDD FIX for E0308: Track both ref and owned enum tuple bindings
            // Check if match yields ref bindings or owned bindings
            Pattern::EnumVariant(variant_name, EnumPatternBinding::Tuple(pats)) => {
                let Some(key) = self.enum_pattern_registry_key(variant_name, &inner_type) else {
                    return out;
                };
                let Some(types) = self.lookup_enum_variant_payload_types(&key) else {
                    return out;
                };

                for (pat, ty) in pats.iter().zip(types.iter()) {
                    if let Some(name) = Self::pattern_simple_binding_name(pat) {
                        if yields_refs {
                            // Match scrutinee is borrowed, bindings are refs
                            out.push((name.to_string(), Type::Reference(Box::new(ty.clone()))));
                        } else {
                            // Match scrutinee is owned, bindings are owned
                            out.push((name.to_string(), ty.clone()));
                        }
                    }
                }
            }
            _ => {}
        }

        out
    }

    /// Identifier / `mut x` / `ref x` / `ref mut x` binding names in enum payloads.
    fn pattern_simple_binding_name<'p>(pat: &'p Pattern<'_>) -> Option<&'p str> {
        match pat {
            Pattern::Identifier(name)
            | Pattern::MutBinding(name)
            | Pattern::Ref(name)
            | Pattern::RefMut(name) => Some(name.as_str()),
            _ => None,
        }
    }

    /// `Data::Arrow(handle)` → payload types from the global enum registry,
    /// including when the container is stored as `crate::handle::Data`.
    fn infer_match_bound_types_from_pattern_key(
        &self,
        pattern: &Pattern,
        yields_refs: bool,
    ) -> Vec<(String, Type)> {
        let (variant_name, var_name) = match pattern {
            Pattern::EnumVariant(variant_name, EnumPatternBinding::Single(var_name)) => {
                (variant_name.as_str(), var_name.as_str())
            }
            Pattern::EnumVariant(variant_name, EnumPatternBinding::Tuple(pats))
                if pats.len() == 1 =>
            {
                match Self::pattern_simple_binding_name(&pats[0]) {
                    Some(name) => (variant_name.as_str(), name),
                    None => return Vec::new(),
                }
            }
            _ => return Vec::new(),
        };
        let Some(types) = self.lookup_enum_variant_payload_types(variant_name) else {
            return Vec::new();
        };
        if types.len() != 1 {
            return Vec::new();
        }
        let ty = types[0].clone();
        if yields_refs {
            vec![(var_name.to_string(), Type::Reference(Box::new(ty)))]
        } else {
            vec![(var_name.to_string(), ty)]
        }
    }

    fn lookup_enum_variant_payload_types(&self, variant_name: &str) -> Option<&Vec<Type>> {
        if let Some(types) = self.enum_variant_types.get(variant_name) {
            return Some(types);
        }
        let parts: Vec<&str> = variant_name.rsplit("::").take(2).collect();
        if parts.len() == 2 {
            let key = format!("{}::{}", parts[1], parts[0]);
            if let Some(types) = self.enum_variant_types.get(&key) {
                return Some(types);
            }
        }
        // `crate::handle::Data::Arrow` vs registry `Data::Arrow` (and the reverse).
        let suffix = format!("::{variant_name}");
        self.enum_variant_types.iter().find_map(|(k, types)| {
            if k == variant_name || k.ends_with(&suffix) || variant_name.ends_with(&format!("::{k}"))
            {
                Some(types)
            } else {
                None
            }
        })
    }

    /// After `.copied()` on `Option<&T>` (Copy `T`), match bindings are owned `T`.
    pub(in crate::codegen::rust) fn infer_match_bound_types_from_copied_option(
        &self,
        scrutinee: &Expression,
        pattern: &Pattern,
    ) -> Vec<(String, Type)> {
        let Some(ty) = self.infer_expression_type(scrutinee) else {
            return Vec::new();
        };
        let Type::Option(inner) = ty else {
            return self.infer_match_bound_types(scrutinee, pattern);
        };
        let owned_inner = match inner.as_ref() {
            Type::Reference(r) | Type::MutableReference(r) => r.as_ref().clone(),
            other => other.clone(),
        };
        match pattern {
            Pattern::EnumVariant(variant, EnumPatternBinding::Single(var_name))
                if variant == "Some" || variant.ends_with("::Some") =>
            {
                vec![(var_name.clone(), owned_inner)]
            }
            _ => self.infer_match_bound_types_owned(scrutinee, pattern),
        }
    }

    /// Type of `let x = match scrutinee { … }` (parsed as `Block { Match }`).
    /// Diverging arms (`return` / `break`) are skipped so `Some(n) => n` yields `n`'s type.
    pub(in crate::codegen::rust) fn infer_match_expression_type(
        &self,
        scrutinee: &Expression,
        arms: &[MatchArm],
    ) -> Option<Type> {
        for arm in arms {
            if Self::match_arm_body_diverges(arm.body) {
                continue;
            }
            if let Some(ty) = self.infer_match_arm_body_type(scrutinee, arm) {
                return Some(ty);
            }
        }
        None
    }

    fn match_arm_body_diverges(body: &Expression) -> bool {
        match body {
            Expression::Block { statements, .. } => {
                !statements.is_empty()
                    && statements.iter().all(|s| {
                        matches!(
                            s,
                            Statement::Return { .. }
                                | Statement::Break { .. }
                                | Statement::Continue { .. }
                        )
                    })
            }
            _ => false,
        }
    }

    fn infer_match_arm_body_type(&self, scrutinee: &Expression, arm: &MatchArm) -> Option<Type> {
        let bindings = self.infer_match_bound_types(scrutinee, &arm.pattern);
        match arm.body {
            Expression::Identifier { name, .. } => bindings
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, t)| t.clone())
                .or_else(|| self.infer_expression_type(arm.body)),
            Expression::Block { statements, .. } => {
                let last = statements.last()?;
                match last {
                    Statement::Expression { expr, .. } => {
                        if let Expression::Identifier { name, .. } = expr {
                            if let Some((_, t)) = bindings.iter().find(|(n, _)| n == name) {
                                return Some(t.clone());
                            }
                        }
                        self.infer_expression_type(expr)
                    }
                    Statement::Return { .. }
                    | Statement::Break { .. }
                    | Statement::Continue { .. } => None,
                    _ => None,
                }
            }
            _ => self.infer_expression_type(arm.body),
        }
    }
}
