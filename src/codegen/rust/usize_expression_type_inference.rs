//! `usize` detection and inferred-type reference-structure checks.

use crate::codegen::rust::CodeGenerator;
use crate::parser::{BinaryOp, Expression, Literal, Type};

impl<'ast> CodeGenerator<'ast> {
    /// Check if an expression's inferred type wraps a reference
    /// (e.g. `Option<&T>`, `Result<&T, E>`).
    pub(in crate::codegen::rust) fn expression_type_contains_reference(&self, expr: &Expression) -> bool {
        self.infer_expression_type(expr)
            .as_ref()
            .is_some_and(Self::type_contains_reference_static)
    }

    pub(in crate::codegen::rust) fn type_contains_reference_static(ty: &Type) -> bool {
        match ty {
            Type::Reference(_) | Type::MutableReference(_) => true,
            Type::Option(inner) => Self::type_contains_reference_static(inner),
            Type::Result(ok, _) => Self::type_contains_reference_static(ok),
            _ => false,
        }
    }

    pub(in crate::codegen::rust) fn type_contains_mut_reference_static(ty: &Type) -> bool {
        match ty {
            Type::MutableReference(_) => true,
            Type::Option(inner) => Self::type_contains_mut_reference_static(inner),
            Type::Result(ok, _) => Self::type_contains_mut_reference_static(ok),
            _ => false,
        }
    }

    /// Check if an expression already produces `&str`, making a redundant
    /// `.as_str()` call unnecessary. Uses type inference plus borrowed-param tracking.
    pub(in crate::codegen::rust) fn expression_produces_str_ref(&self, expr: &Expression) -> bool {
        if let Some(ty) = self.infer_expression_type(expr) {
            if matches!(
                ty,
                Type::Reference(ref inner) if matches!(inner.as_ref(), Type::String)
            ) {
                return true;
            }
        }
        if let Expression::Identifier { name, .. } = expr {
            if self.inferred_borrowed_params.contains(name.as_str()) {
                if let Some(param) = self
                    .current_function_params
                    .iter()
                    .find(|p| p.name == *name)
                {
                    if matches!(&param.type_, Type::String) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Check if an expression produces usize (e.g., .len(), array indexing)
    /// Used for auto-casting between i32 and usize in comparisons
    pub(crate) fn expression_produces_usize(&self, expr: &Expression) -> bool {
        match expr {
            // .len() returns usize
            Expression::MethodCall { method, .. } => {
                if method == "len" || method == "count" || method == "capacity" {
                    return true;
                }
                // Fallback: check via type inference
                self.infer_expression_type_is_usize(expr)
            }
            // Postfix `obj.len()` parses as Call(FieldAccess(obj, "len"), []), not MethodCall.
            Expression::Call {
                function,
                arguments,
                ..
            } if arguments.is_empty() => {
                if let Expression::FieldAccess { field, .. } = function {
                    if field == "len" || field == "count" || field == "capacity" {
                        return true;
                    }
                }
                self.infer_expression_type_is_usize(expr)
            }
            // Binary ops with usize operands: i + 1, len() - 1, etc.
            // TDD FIX (Bug #4): If BOTH sides are usize (or one side is usize and other is int literal),
            // then the result is usize. The old logic used OR which was wrong.
            Expression::Binary {
                op,
                left,
                right,
                location: _,
            } => {
                match op {
                    // Arithmetic operations preserve usize if both operands are usize-compatible
                    BinaryOp::Add
                    | BinaryOp::Sub
                    | BinaryOp::Mul
                    | BinaryOp::Div
                    | BinaryOp::Mod => {
                        let left_is_usize = self.expression_produces_usize(left);
                        let right_is_usize = self.expression_produces_usize(right);

                        // Int literals adapt to the other operand's type
                        let right_is_literal = matches!(**right, Expression::Literal { .. });
                        let left_is_literal = matches!(**left, Expression::Literal { .. });

                        // Result is usize if:
                        // - Both are usize, OR
                        // - One is usize and the other is an int literal
                        (left_is_usize && (right_is_usize || right_is_literal))
                            || (right_is_usize && left_is_literal)
                    }
                    // Comparison/logical operations don't produce usize
                    _ => false,
                }
            }
            // Casts to usize: (x as usize)
            Expression::Cast { type_, .. } => {
                matches!(type_, Type::Custom(name) if name == "usize")
            }
            // Variables assigned from .len() or typed as usize
            Expression::Identifier { name, .. } => {
                if self.usize_variables.contains(name) {
                    return true;
                }

                // Check if this is a struct field with usize type (in impl block)
                if self.in_impl_block && self.current_struct_fields.contains(name) {
                    // Look up the struct to see if this field is usize
                    // Strip generic parameters: "Pool<T>" → "Pool"
                    if let Some(struct_name) = &self.current_struct_name {
                        let base_name = struct_name.split('<').next().unwrap_or(struct_name);
                        if let Some(usize_fields) = self.usize_struct_fields.get(base_name) {
                            if usize_fields.contains(name) {
                                return true;
                            }
                        }
                    }
                }

                // Fallback: check parameters and local variable types via type inference
                self.infer_expression_type_is_usize(expr)
            }
            // Field access: self.field_name or obj.field_name (including nested)
            Expression::FieldAccess { object, field, .. } => {
                // Check if accessing a usize field on self (fast path)
                if let Expression::Identifier { name: obj_name, .. } = &**object {
                    if obj_name == "self" && self.in_impl_block {
                        // Look up struct to see if this field is usize
                        if let Some(struct_name) = &self.current_struct_name {
                            // Strip generic parameters: "Pool<T>" → "Pool"
                            let base_name = struct_name.split('<').next().unwrap_or(struct_name);
                            if let Some(usize_fields) = self.usize_struct_fields.get(base_name) {
                                if usize_fields.contains(field) {
                                    return true;
                                }
                            }
                        }
                    }
                }
                // Fallback: use type inference for obj.field, self.config.field, etc.
                self.infer_expression_type_is_usize(expr)
            }
            _ => false,
        }
    }

    /// Check if an expression's inferred type is usize.
    /// Uses infer_expression_type() for comprehensive type resolution including
    /// parameters, local variables, nested field access, and method return types.
    pub(in crate::codegen::rust) fn infer_expression_type_is_usize(&self, expr: &Expression) -> bool {
        if let Some(t) = self.infer_expression_type(expr) {
            return matches!(t, Type::Custom(ref name) if name == "usize");
        }
        false
    }

    /// `true` when comparing against `.len()` should cast the **usize/len** side to `i64`
    /// (Windjammer `int` / signed Rust integers on the other operand).
    ///
    /// When the other operand is already `usize` (or an untyped int literal, which Rust
    /// matches to `usize` next to `.len()`), returns `false`.
    pub(in crate::codegen::rust) fn comparison_other_side_needs_len_as_i64(&self, expr: &Expression) -> bool {
        if self.infer_expression_type_is_usize(expr) {
            return false;
        }
        if self.expression_produces_usize(expr) {
            return false;
        }
        // Untyped integer: Rust infers `usize` next to `.len()` — never force `len() as i64`.
        if matches!(
            expr,
            Expression::Literal {
                value: Literal::Int(_),
                ..
            }
        ) {
            return false;
        }
        if let Some(t) = self.infer_expression_type(expr) {
            if Self::type_is_signed_int_for_len_usize_comparison(&t) {
                return true;
            }
        }
        if let Some(inference) = &self.int_inference {
            use crate::type_inference::IntType;
            let it = self.int_type_for_mixed_int_codegen(expr, inference);
            if it == IntType::Usize {
                return false;
            }
            return matches!(
                it,
                IntType::I8 | IntType::I16 | IntType::I32 | IntType::I64 | IntType::Isize
            );
        }
        false
    }

    fn type_is_signed_int_for_len_usize_comparison(t: &Type) -> bool {
        match t {
            Type::Int => true,
            Type::Custom(name) => {
                crate::type_classification::is_integer_type(name) && name.starts_with('i')
            }
            Type::Reference(inner) | Type::MutableReference(inner) => {
                Self::type_is_signed_int_for_len_usize_comparison(inner)
            }
            _ => false,
        }
    }
}
