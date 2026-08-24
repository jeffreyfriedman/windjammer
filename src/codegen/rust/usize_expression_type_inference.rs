//! `usize` detection and inferred-type reference-structure checks.

use crate::codegen::rust::CodeGenerator;
use crate::parser::{BinaryOp, Expression, Literal, Type};

impl<'ast> CodeGenerator<'ast> {
    /// Check if an expression's inferred type wraps a reference
    /// (e.g. `Option<&T>`, `Result<&T, E>`).
    pub(in crate::codegen::rust) fn expression_type_contains_reference(
        &self,
        expr: &Expression,
    ) -> bool {
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
            Expression::MethodCall { object, method, .. } => {
                let obj_ty = self.infer_expression_type(object);
                let recv = obj_ty.as_ref().and_then(Self::type_to_name);
                if crate::codegen::rust::stdlib_method_traits::method_returns_usize_qualified(
                    method,
                    recv.as_deref(),
                    &self.signature_registry,
                ) {
                    return true;
                }
                // `&str` / unknown receivers: consensus or String::{method} usize API.
                if self.method_call_rust_emits_usize(expr) {
                    return true;
                }
                if self.infer_expression_type_is_usize(expr) {
                    return true;
                }
                false
            }
            Expression::Call {
                function,
                arguments,
                ..
            } if arguments.is_empty() => {
                if self.method_call_rust_emits_usize(expr) {
                    return true;
                }
                if let Expression::FieldAccess { object, field, .. } = function {
                    if crate::codegen::rust::stdlib_method_traits::method_returns_usize_qualified(
                        field,
                        self.infer_expression_type(object)
                            .as_ref()
                            .and_then(Self::type_to_name)
                            .as_deref(),
                        &self.signature_registry,
                    ) {
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
                if field == "0" {
                    if let Expression::Identifier { name, .. } = &**object {
                        if self.usize_variables.contains(name)
                            || self
                                .local_var_types
                                .get(name.as_str())
                                .is_some_and(|t| matches!(t, Type::Tuple(_)))
                        {
                            return true;
                        }
                    }
                }
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
    pub(in crate::codegen::rust) fn infer_expression_type_is_usize(
        &self,
        expr: &Expression,
    ) -> bool {
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
    pub(in crate::codegen::rust) fn comparison_other_side_needs_len_as_i64(
        &self,
        expr: &Expression,
    ) -> bool {
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
        if self.numeric_inference.is_some() {
            use crate::type_inference::IntType;
            let it = self.int_type_for_mixed_int_codegen(expr);
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

    /// Cast a `usize`-producing expression to the target int type when needed.
    ///
    /// Handles: `expr` → `(expr) as i64`, `(expr) as i32`, or no-op when target
    /// is already `usize` or unknown.
    pub(in crate::codegen::rust) fn maybe_cast_usize_to_int_target(
        &self,
        expr_str: &mut String,
        expr: &Expression<'ast>,
        target_type: Option<&str>,
    ) {
        if let Some(t) = target_type {
            if matches!(t, "int" | "i64" | "i32") {
                if let Expression::Call {
                    function,
                    arguments,
                    ..
                } = expr
                {
                    if arguments.len() == 1 {
                        let is_some = matches!(
                            &**function,
                            Expression::Identifier { name, .. }
                                if name == "Some" || name.ends_with("::Some")
                        );
                        if is_some {
                            let (_, inner) = &arguments[0];
                            let inner_is_usize = self.expression_produces_usize(inner)
                                || self.infer_expression_type_is_usize(inner);
                            if inner_is_usize {
                                let cast_suffix = if t == "i32" { " as i32" } else { " as i64" };
                                if expr_str.starts_with("Some(") && expr_str.ends_with(')') {
                                    let inner_part =
                                        expr_str[5..expr_str.len().saturating_sub(1)].trim();
                                    let base = inner_part
                                        .strip_suffix(".clone()")
                                        .unwrap_or(inner_part)
                                        .trim();
                                    *expr_str = format!("Some({base}{cast_suffix})");
                                    return;
                                }
                                // E0282 turbofish: `Some::<i64>(i)` when `i: usize` (e.g. find index).
                                if expr_str.starts_with("Some::<") {
                                    if let Some(open_paren) = expr_str.rfind('(') {
                                        if expr_str.ends_with(')') {
                                            let inner_part =
                                                expr_str[open_paren + 1..expr_str.len() - 1].trim();
                                            let base = inner_part
                                                .strip_suffix(".clone()")
                                                .unwrap_or(inner_part)
                                                .trim();
                                            let prefix = &expr_str[..=open_paren];
                                            *expr_str = format!("{prefix}{base}{cast_suffix})");
                                            return;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        let cast_suffix = match target_type {
            Some("usize") => return,
            Some("i32") => " as i32",
            Some("int") | Some("i64") => " as i64",
            _ => return,
        };

        if self.expression_produces_usize(expr) || self.method_call_rust_emits_usize(expr) {
            if !expr_str.contains(" as i64") && !expr_str.contains(" as i32") {
                *expr_str = format!("{expr_str}{cast_suffix}");
            }
        }
    }

    /// Rust lowers text/collection size queries to `usize`. Prefer registry
    /// `usize` returns; fall back to consensus when the receiver type is unknown
    /// or only known as Rust `str`.
    fn method_call_rust_emits_usize(&self, expr: &Expression) -> bool {
        let (method, object) = match expr {
            Expression::MethodCall { method, object, .. } => (method.as_str(), &**object),
            Expression::Call {
                function,
                arguments,
                ..
            } if arguments.is_empty() => {
                if let Expression::FieldAccess { object, field, .. } = &**function {
                    (field.as_str(), &**object)
                } else {
                    return false;
                }
            }
            _ => return false,
        };
        let obj_ty = self.infer_expression_type(object);
        let recv = obj_ty.as_ref().and_then(Self::type_to_name);
        let registry = &self.signature_registry;
        let returns_usize = |receiver: Option<&str>| {
            crate::codegen::rust::stdlib_method_traits::method_returns_usize_qualified(
                method, receiver, registry,
            )
        };
        if returns_usize(recv.as_deref()) {
            return true;
        }
        // Text receivers share `String::{len,…}` usize APIs.
        if obj_ty
            .as_ref()
            .is_some_and(|t| crate::codegen::rust::types::is_windjammer_text_type(t))
            && returns_usize(Some("String"))
        {
            return true;
        }
        if obj_ty
            .as_ref()
            .is_some_and(|t| crate::type_classification::type_is_vec_container(t))
            && returns_usize(Some("Vec"))
        {
            return true;
        }
        if let Some(obj_ty) = obj_ty.as_ref() {
            let receiver = Self::type_to_name(obj_ty);
            if crate::codegen::rust::stdlib_method_traits::method_returns_usize_qualified(
                method,
                receiver.as_deref(),
                &self.signature_registry,
            ) {
                return true;
            }
        }
        // Unknown receiver: registry consensus only — never invent usize from
        // hardcoded String/Vec method-name fallbacks.
        returns_usize(None)
    }

    /// Cast an index expression to `usize` if needed for Rust array/Vec indexing.
    ///
    /// Handles: int→usize cast, i64/int cast rewrite, usize variable skip,
    /// non-negative literal skip, binary expression parenthesization.
    fn identifier_emits_as_usize(&self, name: &str) -> bool {
        self.current_function_params.iter().any(|p| {
            p.name == name && matches!(&p.type_, Type::Custom(s) if s == "usize")
        }) || self
            .local_var_types
            .get(name)
            .is_some_and(|t| matches!(t, Type::Custom(s) if s == "usize"))
    }

    pub(in crate::codegen::rust) fn maybe_cast_index_to_usize(
        &self,
        idx_str: &mut String,
        index: &Expression<'ast>,
    ) {
        // Non-negative integer literals infer as usize in index context — no cast needed.
        if let Expression::Literal {
            value: Literal::Int(n),
            ..
        } = index
        {
            if *n >= 0 {
                return;
            }
        }
        // Already a usize index form — never double-cast (`0_usize as usize`).
        if idx_str.contains(" as usize") || idx_str.ends_with("_usize") {
            return;
        }
        if self.infer_expression_type_is_usize(index) {
            return;
        }
        if let Expression::Identifier { name, .. } = index {
            // Loop-promoted `int` counters (`while i < vec.len()`) stay i64 in Rust — index still
            // needs `as usize` even when comparison analysis marked them in `usize_variables`.
            if self.identifier_emits_as_usize(name) {
                return;
            }
        } else if self.expression_produces_usize(index) {
            return;
        }
        if let Some(ty) = self.infer_expression_type(index) {
            let needs_usize_cast = matches!(ty, Type::Int)
                || matches!(ty, Type::Custom(name) if name == "int" || name == "i64" || name == "i32");
            if needs_usize_cast {
                let needs_parens = matches!(index, Expression::Binary { .. });
                if needs_parens {
                    *idx_str = format!("({}) as usize", idx_str);
                } else {
                    *idx_str = format!("{} as usize", idx_str);
                }
                return;
            }
        }
        if idx_str.ends_with("as i64)") || idx_str.ends_with("as int)") {
            let base = idx_str
                .trim_end_matches("as i64)")
                .trim_end_matches("as int)")
                .trim()
                .trim_start_matches('(')
                .trim();
            *idx_str = format!("{} as usize", base);
        } else if idx_str.ends_with("as i64") || idx_str.ends_with("as int") {
            let base = idx_str
                .trim_end_matches("as i64")
                .trim_end_matches("as int")
                .trim();
            *idx_str = format!("{} as usize", base);
        } else if !idx_str.contains(" as ") {
            if self.infer_expression_type_is_usize(index) {
                return;
            }
            if let Expression::Identifier { name, .. } = index {
                if self.identifier_emits_as_usize(name) {
                    return;
                }
            }
            let needs_cast = match index {
                Expression::Identifier { name, .. } => !self.identifier_emits_as_usize(name),
                Expression::Literal {
                    value: Literal::Int(n),
                    ..
                } => {
                    if *n >= 0 {
                        let suffixes = ["_usize", "_i32", "_i64", "_u32", "_u64"];
                        for s in &suffixes {
                            if idx_str.ends_with(s) {
                                idx_str.truncate(idx_str.len() - s.len());
                                break;
                            }
                        }
                    }
                    *n < 0
                }
                _ => true,
            };
            if needs_cast {
                let needs_parens = matches!(index, Expression::Binary { .. });
                if needs_parens {
                    *idx_str = format!("({}) as usize", idx_str);
                } else {
                    *idx_str = format!("{} as usize", idx_str);
                }
            }
        }
    }
}
