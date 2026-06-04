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

            // WINDJAMMER PHILOSOPHY: Auto-convert string literals in return statements
            // when the function returns String
            let returns_string = match &self.current_function_return_type {
                Some(Type::String) => true,
                Some(Type::Custom(name)) if name == "String" => true,
                _ => false,
            };

            if returns_string {
                // String literal needs owned String — use .into(), not .to_string() (no Rust leakage)
                if matches!(
                    e,
                    Expression::Literal {
                        value: Literal::String(_),
                        ..
                    }
                ) && !crate::codegen::rust::literals::is_already_owned_string(&return_str)
                {
                    return_str =
                        crate::codegen::rust::literals::string_literal_to_owned_rust(&return_str);
                }
                // param.clone() where param: &str → param.to_string()
                // &str.clone() returns &str, but we need String
                else if let Expression::MethodCall { method, object, .. } = e {
                    if method == "clone" {
                        if let Expression::Identifier { name, .. } = &**object {
                            // Check if this identifier is a borrowed string parameter
                            let is_string_type = self.current_function_params.iter().any(|p| {
                                p.name == *name
                                    && (matches!(p.type_, Type::String)
                                        || matches!(p.type_, Type::Custom(ref n) if n == "string"))
                            });
                            let is_borrowed_str_param =
                                self.inferred_borrowed_params.contains(name) && is_string_type;

                            if is_borrowed_str_param {
                                return_str = return_str.replace(".clone()", ".to_string()");
                            }
                        }
                    }
                }
                // self.field needs .clone() when self is borrowed
                // BUT: Skip .clone() for Copy types (f32, i32, bool, etc.)
                else if let Expression::FieldAccess { object, .. } = e {
                    if let Expression::Identifier { name: obj_name, .. } = &**object {
                        if obj_name == "self" && !return_str.ends_with(".clone()") {
                            let self_is_borrowed = self.current_function_params.iter().any(|p| {
                                p.name == "self"
                                    && matches!(p.ownership, crate::parser::OwnershipHint::Ref)
                            });
                            if self_is_borrowed {
                                let is_copy = self
                                    .infer_expression_type(e)
                                    .as_ref()
                                    .is_some_and(|t| self.is_type_copy(t));
                                if !is_copy {
                                    return_str = format!("{}.clone()", return_str);
                                }
                            }
                        }
                    }
                }
            }

            // FIXED: Auto-cast usize to i64 when function returns int
            // WINDJAMMER PHILOSOPHY: Compiler handles type conversions automatically
            let returns_int = match &self.current_function_return_type {
                Some(Type::Int) => true,
                Some(Type::Custom(name)) if name == "i64" || name == "int" => true,
                _ => false,
            };

            if returns_int && self.expression_produces_usize(e) {
                // .len() returns usize, but function expects i64 - auto-cast!
                return_str = format!("{} as i64", return_str);
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
                return_str = format!("{}.clone()", return_str);
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
                        if !self.is_type_copy(&inner) {
                            return_str = format!("({}).clone()", return_str);
                        }
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
