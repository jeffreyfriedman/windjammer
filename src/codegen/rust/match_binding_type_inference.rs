//! Infers `match` / `if let` pattern binding types.

use crate::codegen::rust::CodeGenerator;
use crate::parser::{EnumPatternBinding, Expression, Pattern, Type};
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

    fn enum_pattern_registry_key(
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
        let scrutinee_type = match self.infer_expression_type(scrutinee) {
            Some(t) => t,
            None => return Vec::new(),
        };

        let inner_type = match &scrutinee_type {
            Type::Reference(inner) | Type::MutableReference(inner) => inner.as_ref().clone(),
            _ => scrutinee_type.clone(),
        };

        let mut out = Vec::new();

        match pattern {
            Pattern::EnumVariant(variant, EnumPatternBinding::Single(var_name))
                if variant == "Some" || variant.ends_with("::Some") =>
            {
                if let Type::Option(inner_t) = &inner_type {
                    out.push((var_name.clone(), inner_t.as_ref().clone()));
                }
            }
            Pattern::EnumVariant(variant_name, EnumPatternBinding::Struct(fields, _)) => {
                let Some(key) = self.enum_pattern_registry_key(variant_name, &inner_type) else {
                    return out;
                };
                let Some(named) = self.enum_variant_struct_fields.get(&key) else {
                    return out;
                };
                let yields_refs = self.match_scrutinee_yields_ref_enum_bindings(scrutinee);
                let map: HashMap<String, Type> = named.iter().cloned().collect();
                for (fname, pat) in fields.iter() {
                    if let Pattern::Identifier(binding_name) = pat {
                        if let Some(ft) = map.get(fname) {
                            if yields_refs {
                                out.push((
                                    binding_name.clone(),
                                    Type::Reference(Box::new(ft.clone())),
                                ));
                            } else {
                                out.push((binding_name.clone(), ft.clone()));
                            }
                        }
                    }
                }
            }
            // Single-field tuple variants use EnumPatternBinding::Single (e.g. Cost::Gold(amount)).
            // Tuple(..) is only used when the inner pattern is not a plain identifier.
            Pattern::EnumVariant(variant_name, EnumPatternBinding::Single(var_name)) => {
                let Some(key) = self.enum_pattern_registry_key(variant_name, &inner_type) else {
                    return out;
                };
                let Some(types) = self.enum_variant_types.get(&key) else {
                    return out;
                };
                if types.len() == 1 {
                    let yields_refs = self.match_scrutinee_yields_ref_enum_bindings(scrutinee);
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
                let Some(types) = self.enum_variant_types.get(&key) else {
                    return out;
                };

                let yields_refs = self.match_scrutinee_yields_ref_enum_bindings(scrutinee);

                for (pat, ty) in pats.iter().zip(types.iter()) {
                    if let Pattern::Identifier(name) = pat {
                        if yields_refs {
                            // Match scrutinee is borrowed, bindings are refs
                            out.push((name.clone(), Type::Reference(Box::new(ty.clone()))));
                        } else {
                            // Match scrutinee is owned, bindings are owned
                            out.push((name.clone(), ty.clone()));
                        }
                    }
                }
            }
            _ => {}
        }

        out
    }
}
