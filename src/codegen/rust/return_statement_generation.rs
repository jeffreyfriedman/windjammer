//! Return statement generation
//!
//! Handles code generation for return statements including:
//! - Optional return values
//! - Auto-conversion of string literals to String
//! - Borrowed iterator variable dereferencing
//! - Type coercion (usize → i64, Option reference cloning)
//! - Vec indexing clone insertion
//! - Reference unpacking for Copy types

use crate::parser::*;

use super::CodeGenerator;

impl<'ast> CodeGenerator<'ast> {
    /// Generate code for a return statement
    pub(in crate::codegen::rust) fn generate_return_statement(
        &mut self,
        expr: &Option<&'ast Expression<'ast>>,
    ) -> String {
        let mut output = self.indent();
        output.push_str("return");
        if let Some(e) = expr {
            output.push(' ');
            let mut return_str = self.generate_expression(e);

            // TDD FIX: Borrowed iterator vars need deref when returned as Copy types
            // For `for (_, val) in &vec` where val: &i32, `return val` needs `return *val`
            if let Expression::Identifier { name, .. } = e {
                if self.borrowed_iterator_vars.contains(name) {
                    let return_type_is_copy = self
                        .current_function_return_type
                        .as_ref()
                        .is_some_and(|t| self.is_type_copy(t));
                    if return_type_is_copy && !return_str.starts_with('*') {
                        return_str = format!("*{}", return_str);
                    }
                }
            }

            self.apply_owned_string_tail_coercion(&mut return_str, e, false);

            {
                let target = match &self.current_function_return_type {
                    Some(Type::Int) => Some("int"),
                    Some(Type::Custom(name)) if name == "i64" || name == "int" => Some("int"),
                    _ => None,
                };
                self.maybe_cast_usize_to_int_target(&mut return_str, e, target);
            }

            let returns_option_owned = self.returns_option_owned_type();
            if returns_option_owned
                && self.expression_type_contains_reference(e)
                && !return_str.ends_with(".cloned()")
                && !return_str.ends_with(".clone()")
            {
                if self
                    .infer_expression_type(e)
                    .as_ref()
                    .is_some_and(Self::type_contains_mut_reference_static)
                {
                    return_str = format!("{}.map(|v| v.clone())", return_str);
                } else {
                    return_str = format!("{}.cloned()", return_str);
                }
            }

            // DOGFOODING FIX: Vec indexing returns &T for non-Copy, but return expects T
            // e.g. return self.slots[idx] where slots: Vec<SaveSlot> → need .clone()
            // Use parentheses: (&vec[idx]).clone() - . has higher precedence than &
            // Never apply to &mut … — functions returning &mut T must pass the reference through
            // (e.g. return &mut self.items[i], not (&mut self.items[i]).clone()).
            let mut needs_index_return_clone = false;
            if matches!(e, Expression::Index { .. })
                && !return_str.ends_with(".clone()")
                && !return_str.starts_with("&mut")
            {
                let expects_owned = !matches!(
                    &self.current_function_return_type,
                    Some(Type::Reference(_)) | Some(Type::MutableReference(_))
                );
                if expects_owned {
                    let is_copy = self
                        .infer_expression_type(e)
                        .as_ref()
                        .is_some_and(|t| self.is_type_copy(t));
                    needs_index_return_clone = !is_copy;
                }
            }
            if needs_index_return_clone {
                if return_str.starts_with('&') && !return_str.starts_with("&mut") {
                    return_str = format!("({}).clone()", return_str);
                } else {
                    return_str = format!("{}.clone()", return_str);
                }
            } else if return_str.starts_with("&")
                && !return_str.starts_with("&mut")
                && !return_str.ends_with(".clone()")
            {
                let expects_owned = !matches!(
                    &self.current_function_return_type,
                    Some(Type::Reference(_)) | Some(Type::MutableReference(_))
                );
                if expects_owned {
                    let inner_type = self.infer_expression_type(e).map(|t| match &t {
                        Type::Reference(inner) => inner.as_ref().clone(),
                        _ => t,
                    });
                    if let Some(inner) = inner_type {
                        if self.is_type_copy(&inner) && !return_str.starts_with('*') {
                            return_str = format!("*{}", return_str);
                        } else if !self.is_type_copy(&inner) {
                            return_str = format!("({}).clone()", return_str);
                        }
                    }
                }
            }

            self.coerce_return_ref_to_owned_copy(&mut return_str, e);

            // `return self.field` on borrowed `self` when the return type is owned (not `&T`).
            // After index/`&` fixes above so we emit `( &expr ).clone()` not `&expr.clone()`.
            if super::self_analysis::is_self_field_chain(e)
                && (self.inferred_mut_borrowed_params.contains("self")
                    || self.inferred_borrowed_params.contains("self"))
            {
                let returns_ref = matches!(
                    &self.current_function_return_type,
                    Some(Type::Reference(_)) | Some(Type::MutableReference(_))
                );
                if !returns_ref && !return_str.ends_with(".clone()") {
                    if return_str.starts_with('&') && !return_str.starts_with("&mut") {
                        return_str = format!("({}).clone()", return_str);
                    } else {
                        return_str = format!("{}.clone()", return_str);
                    }
                }
            }

            // `let (a, b) = &vec[i]` in Rust: Copy fields like `i32` are still `&i32` bindings.
            // When we record `Type::Reference(i32)` in local_var_types, `return b` must become `*b`.
            if let Expression::Identifier { .. } = e {
                let expects_owned_ref = !matches!(
                    &self.current_function_return_type,
                    Some(Type::Reference(_)) | Some(Type::MutableReference(_))
                );
                if expects_owned_ref {
                    if let Some(Type::Reference(inner)) = self.infer_expression_type(e) {
                        if self.is_type_copy(inner.as_ref()) && !return_str.starts_with('*') {
                            return_str = format!("*{}", return_str);
                        }
                    }
                }
            }

            output.push_str(&return_str);
        }
        output.push_str(";\n");
        output
    }
}
