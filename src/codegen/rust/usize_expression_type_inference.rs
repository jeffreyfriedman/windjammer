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
            } => {
                // `strings.len(s)` / `json.len(v)` parse as Call(FieldAccess) with args —
                // registry baseline returns Rust `usize` even when WJ stubs say `-> int`.
                if self.method_call_rust_emits_usize(expr) {
                    return true;
                }
                if arguments.is_empty() {
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
                }
                // Free calls — inferred/registry return type.
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
                // Binding width beats `.len()`-compare promotion into `usize_variables`
                // (`let mut i = 0` under `-> int` stays i64; cast `strings::len` instead — P3.299).
                if self.local_var_types.get(name.as_str()).is_some_and(|t| {
                    matches!(t, Type::Int | Type::Int32)
                        || matches!(
                            t,
                            Type::Custom(n)
                                if matches!(n.as_str(), "int" | "i64" | "i32" | "u32" | "u64")
                        )
                }) {
                    return false;
                }
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
                // Numeric tuple index: only usize when that element is usize (not `(u64, bool).0`).
                if field.chars().all(|c| c.is_ascii_digit()) {
                    return self.infer_expression_type_is_usize(expr);
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
        if let Expression::Identifier { name, .. } = expr {
            if self.codegen_i32_binding_names.contains(name)
                || self.local_var_types.get(name.as_str()).is_some_and(|t| {
                    matches!(t, Type::Int32) || matches!(t, Type::Custom(n) if n == "i32")
                })
            {
                return false;
            }
            if self.local_var_types.get(name.as_str()).is_some_and(|t| {
                matches!(t, Type::Int)
                    || matches!(
                        t,
                        Type::Custom(n)
                            if matches!(n.as_str(), "int" | "i64" | "u32" | "u64")
                    )
            }) {
                return true;
            }
            if self.usize_variables.contains(name) {
                return false;
            }
        }
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
            if Self::unsigned_int_width_for_len_cast(&t).is_some() {
                return false;
            }
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
            if matches!(it, IntType::I32 | IntType::U32 | IntType::Usize) {
                return false;
            }
            return matches!(it, IntType::I8 | IntType::I16 | IntType::I64 | IntType::Isize);
        }
        false
    }

    fn type_is_signed_int_for_len_usize_comparison(t: &Type) -> bool {
        match t {
            Type::Int32 => false,
            Type::Int => true,
            Type::Custom(name) => {
                if name == "i32" {
                    return false;
                }
                crate::type_classification::is_integer_type(name) && name.starts_with('i')
            }
            Type::Reference(inner) | Type::MutableReference(inner) => {
                Self::type_is_signed_int_for_len_usize_comparison(inner)
            }
            _ => false,
        }
    }

    /// When comparing `u64` (etc.) against `.len()`, cast the **len** side to that width — not `i64`.
    pub(in crate::codegen::rust) fn unsigned_int_width_for_len_cast(ty: &Type) -> Option<&'static str> {
        match ty {
            Type::Uint => Some("u32"),
            Type::Custom(name) => match name.as_str() {
                "u8" => Some("u8"),
                "u16" => Some("u16"),
                "u32" => Some("u32"),
                "u64" => Some("u64"),
                "u128" => Some("u128"),
                _ => None,
            },
            Type::Reference(inner) | Type::MutableReference(inner) => {
                Self::unsigned_int_width_for_len_cast(inner.as_ref())
            }
            _ => None,
        }
    }

    /// i32 counters vs `.len()`: cast len to `i32`, not `i64` (P3.338).
    pub(in crate::codegen::rust) fn narrow_signed_width_for_len_compare(ty: &Type) -> Option<&'static str> {
        match ty {
            Type::Int32 => Some("i32"),
            Type::Custom(name) if name == "i32" => Some("i32"),
            Type::Reference(inner) | Type::MutableReference(inner) => {
                Self::narrow_signed_width_for_len_compare(inner.as_ref())
            }
            _ => None,
        }
    }

    pub(in crate::codegen::rust) fn expression_narrow_signed_width_for_len_compare(
        &self,
        expr: &Expression,
    ) -> Option<&'static str> {
        if let Expression::Identifier { name, .. } = expr {
            if self.codegen_i32_binding_names.contains(name) {
                return Some("i32");
            }
            if let Some(t) = self.local_var_types.get(name) {
                if let Some(w) = Self::narrow_signed_width_for_len_compare(t) {
                    return Some(w);
                }
            }
        }
        self.infer_expression_type(expr)
            .as_ref()
            .and_then(Self::narrow_signed_width_for_len_compare)
            .or_else(|| {
                if self.int_type_for_mixed_int_codegen(expr) == crate::type_inference::IntType::I32 {
                    Some("i32")
                } else {
                    None
                }
            })
    }

    pub(in crate::codegen::rust) fn expression_unsigned_width_for_len_cast(
        &self,
        expr: &Expression,
    ) -> Option<&'static str> {
        self.infer_expression_type(expr)
            .as_ref()
            .and_then(Self::unsigned_int_width_for_len_cast)
            .or_else(|| {
                if let Expression::Identifier { name, .. } = expr {
                    self.local_var_types
                        .get(name)
                        .and_then(Self::unsigned_int_width_for_len_cast)
                } else {
                    None
                }
            })
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
            // `json.len(v)` / `strings.len(s)` parse as Call(FieldAccess), often with args.
            Expression::Call { function, .. } => {
                if let Expression::FieldAccess { object, field, .. } = &**function {
                    (field.as_str(), &**object)
                } else {
                    return false;
                }
            }
            _ => return false,
        };
        // `strings.len(s)` — registry `strings::len` → usize (module is not a receiver type).
        // Prefer the runtime/stdlib scanner baseline over WJ stubs that declare `-> int`
        // (`json.len` → `usize` in windjammer_runtime).
        if let Expression::Identifier { name, .. } = object {
            let key = format!("{name}::{method}");
            let stdlib = crate::analyzer::SignatureRegistry::stdlib();
            if let Some(baseline) = stdlib
                .get_signature(&key)
                .or_else(|| stdlib.get_fallback_signature(&key))
            {
                if baseline
                    .return_type
                    .as_ref()
                    .is_some_and(crate::codegen::rust::type_casting::type_is_usize)
                {
                    return true;
                }
            }
            if let Some(ret) = self.runtime_std_module_fn_return_type(name, method) {
                if matches!(ret, Type::Custom(ref n) if n == "usize") {
                    return true;
                }
            }
        }
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
    pub(in crate::codegen::rust) fn identifier_emits_as_usize(&self, name: &str) -> bool {
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
        // P3.369: binary index already cast the i64 base to usize — do not wrap again.
        if idx_str.contains(" as usize)") {
            return;
        }
        // P3.329: float coercion from call-site context must not stick on `[expr]` indices.
        if idx_str.contains(" as f32") {
            *idx_str = idx_str.replace(" as f32", " as usize");
            return;
        }
        if idx_str.contains(" as f64") {
            *idx_str = idx_str.replace(" as f64", " as usize");
            return;
        }
        // P3.335: concrete `i32` locals always cast for slice/Vec index (before usize inference
        // early-returns that skip cast when numeric inference disagrees with the binding).
        if let Expression::Identifier { name, .. } = index {
            if self.local_int_rust_type_name_excluding_ambiguous_int(name) == Some("i32")
                && !idx_str.contains(" as usize")
            {
                *idx_str = format!("({} as usize)", idx_str);
                return;
            }
        }
        if let Expression::Identifier { name, .. } = index {
            if self.local_var_types.get(name.as_str()).is_some_and(|ty| {
                matches!(ty, Type::Int32)
                    || matches!(ty, Type::Custom(n) if n == "i32")
            }) && !idx_str.contains(" as usize")
            {
                *idx_str = format!("({} as usize)", idx_str);
                return;
            }
        }
        if self.int_type_for_mixed_int_codegen(index) == crate::type_inference::IntType::I32
            && !idx_str.contains(" as usize")
        {
            *idx_str = format!("({} as usize)", idx_str);
            return;
        }
        // Non-negative integer literals infer as usize in index context — no cast needed.
        if let Expression::Literal {
            value: Literal::Int(n),
            ..
        } = index
        {
            if *n >= 0 {
                // Bounds may be pre-emitted with `_i64` before this cast runs (e.g. substring).
                if idx_str.ends_with("_i64")
                    || idx_str.ends_with("_i32")
                    || idx_str.ends_with("_int")
                {
                    *idx_str = n.to_string();
                }
                return;
            }
        }
        // Already a usize index form — never double-cast (`0_usize as usize`).
        if idx_str.contains(" as usize") || idx_str.ends_with("_usize") {
            return;
        }
        if self.infer_expression_type_is_usize(index) {
            if let Expression::Identifier { name, .. } = index {
                let is_i32_scan = self.local_var_types.get(name.as_str()).is_some_and(|ty| {
                    matches!(ty, Type::Int32)
                        || matches!(ty, Type::Custom(n) if n == "i32")
                }) || self.int_type_for_mixed_int_codegen(index) == crate::type_inference::IntType::I32;
                if is_i32_scan && !idx_str.contains(" as usize") {
                    *idx_str = format!("({} as usize)", idx_str);
                    return;
                }
                if !is_i32_scan {
                    return;
                }
            } else {
                return;
            }
        }
        if let Expression::Identifier { name, .. } = index {
            // Loop-promoted `int` counters (`while i < vec.len()`) stay i64 in Rust — index still
            // needs `as usize` even when comparison analysis marked them in `usize_variables`.
            if self.identifier_emits_as_usize(name)
                && self.local_int_rust_type_name_excluding_ambiguous_int(name) != Some("i32")
            {
                return;
            }
        } else if self.expression_produces_usize(index) {
            return;
        }
        if let Some(ty) = self.infer_expression_type(index) {
            let needs_usize_cast = matches!(ty, Type::Int | Type::Int32)
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
            if let Expression::Identifier { name, .. } = index {
                if self.local_int_rust_type_name_excluding_ambiguous_int(name).as_deref()
                    == Some("i32")
                {
                    *idx_str = format!("{} as usize", idx_str);
                    return;
                }
            }
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

    /// Rust integer width an expression would emit before unified-int coercion.
    pub(in crate::codegen::rust) fn natural_int_emission_type(
        &self,
        expr: &Expression<'ast>,
    ) -> Option<crate::type_inference::IntType> {
        use crate::type_inference::IntType;
        if self.expression_produces_usize(expr) || self.infer_expression_type_is_usize(expr) {
            return Some(IntType::Usize);
        }
        if let Some(t) = self.infer_expression_type(expr) {
            if let Some(it) = Self::parser_type_to_promotion_int_type(&t) {
                return Some(it);
            }
        }
        None
    }

    /// Narrow/widen a returned integer to the function's declared int width (`-> i32`).
    ///
    /// Default WJ `int` counters emit as `i64`; an explicit `i32` return needs `as i32`
    /// (same ergonomics as `usize` → `int` casts elsewhere).
    pub(in crate::codegen::rust) fn maybe_cast_to_function_return_int_width(
        &self,
        expr_str: &mut String,
        expr: &Expression<'ast>,
    ) {
        use crate::type_inference::int_implicit_casts::{get_cast_suffix, is_safe_implicit_cast};
        use crate::type_inference::IntType;

        let Some(ret) = &self.current_function_return_type else {
            return;
        };
        // Peel Result/Option so `-> Result<i32, E>` still narrows returned ints.
        let to = match Self::peel_option_result_payload(ret) {
            Type::Int32 => IntType::I32,
            Type::Custom(name) if name == "i32" => IntType::I32,
            Type::Int => IntType::I64,
            Type::Custom(name) if name == "i64" || name == "int" => IntType::I64,
            _ => return,
        };
        if expr_str.contains(" as i32") || expr_str.contains(" as i64") {
            return;
        }
        let mut from = self.natural_int_emission_type(expr).unwrap_or_else(|| {
            // Unannotated `let mut count = 0` defaults to WJ `int` → Rust `i64`.
            match expr {
                Expression::Identifier { .. }
                | Expression::Literal {
                    value: Literal::Int(_),
                    ..
                }
                | Expression::Binary { .. } => IntType::I64,
                _ => IntType::Unknown,
            }
        });
        // Numeric inference may unify a counter to `i32` from `-> i32` while codegen still
        // emits default WJ `int` bindings as `i64` (`count += 1_i64`). Prefer emission width.
        if to == IntType::I32 && from == IntType::I32 {
            if let Expression::Identifier { name, .. } = expr {
                let emits_default_i64 = match self.local_var_types.get(name.as_str()) {
                    Some(Type::Int32) => false,
                    Some(Type::Custom(n)) if n == "i32" => false,
                    Some(Type::Int) => true,
                    Some(Type::Custom(n)) if n == "int" || n == "i64" => true,
                    None => true,
                    Some(_) => false,
                };
                if emits_default_i64 {
                    from = IntType::I64;
                }
            } else if matches!(
                expr,
                Expression::Literal {
                    value: Literal::Int(_),
                    ..
                } | Expression::Binary { .. }
            ) {
                from = IntType::I64;
            }
        }
        if from == IntType::Unknown || from == to {
            return;
        }
        if !is_safe_implicit_cast(from, to) {
            return;
        }
        let suffix = get_cast_suffix(to);
        let needs_parens = matches!(expr, Expression::Binary { .. });
        if needs_parens {
            *expr_str = format!("({}) as {}", expr_str, suffix);
        } else {
            *expr_str = format!("{} as {}", expr_str, suffix);
        }
    }

    /// Cast an if/else branch tail to the solver-unified integer type (e.g. `int` + `len` → `usize`).
    pub(in crate::codegen::rust) fn maybe_cast_branch_tail_to_unified_int(
        &self,
        expr_str: &mut String,
        expr: &Expression<'ast>,
    ) {
        if !self.in_expression_context {
            return;
        }
        let Some(ni) = &self.numeric_inference else {
            return;
        };
        let unified = ni.get_int_type(expr);
        if unified == crate::type_inference::IntType::Unknown {
            return;
        }
        let natural = self
            .natural_int_emission_type(expr)
            .unwrap_or(unified);
        if natural == unified {
            return;
        }
        use crate::type_inference::int_implicit_casts::{get_cast_suffix, is_safe_implicit_cast};
        if !is_safe_implicit_cast(natural, unified) {
            return;
        }
        if expr_str.contains(" as ") {
            return;
        }
        let suffix = get_cast_suffix(unified);
        let needs_parens = matches!(expr, Expression::Binary { .. });
        if needs_parens {
            *expr_str = format!("({}) as {}", expr_str, suffix);
        } else {
            *expr_str = format!("{} as {}", expr_str, suffix);
        }
    }
}

#[cfg(test)]
mod i32_return_width_tests {
    use crate::analyzer::Analyzer;
    use crate::codegen::rust::CodeGenerator;
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use crate::CompilationTarget;

    #[test]
    fn trailing_int_counter_into_i32_return_casts() {
        let source = r#"
pub fn count_data_lines(lines: Vec<string>) -> i32 {
    let mut count = 0
    for line in lines {
        count = count + 1
    }
    count
}
"#;
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize_with_locations();
        let parser = Box::leak(Box::new(Parser::new(tokens)));
        let program = parser.parse().expect("parse");
        let mut analyzer = Analyzer::new();
        let (analyzed, registry, _) = analyzer.analyze_program(&program).expect("analyze");
        let mut codegen = CodeGenerator::new(registry, CompilationTarget::Rust);
        let generated = codegen.generate_program(&program, &analyzed);
        assert!(
            generated.contains("count as i32"),
            "i32 return must cast i64 counter. Got:\n{generated}"
        );
    }
}
