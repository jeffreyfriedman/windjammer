//! Mixed-integer promotion for `as T` codegen.

use crate::codegen::rust::CodeGenerator;
use crate::parser::{Expression, Literal, Type};
use crate::type_inference::IntType;

impl<'ast> CodeGenerator<'ast> {
    /// When let/assign/range set `assignment_int_target_type`, mixed-int ops unify to that width.
    pub(in crate::codegen::rust) fn promotion_int_type_from_assignment_context(
        &self,
    ) -> Option<IntType> {
        self.assignment_int_target_type
            .as_ref()
            .and_then(Self::parser_type_to_promotion_int_type)
            .filter(|t| *t != IntType::Unknown && *t != IntType::Usize)
    }

    /// P3.309: i32 loop counter vs int literal / i32 field — do not widen RHS to i64.
    pub(in crate::codegen::rust) fn comparison_should_prefer_i32_over_i64(
        &self,
        i32_side: &Expression<'ast>,
        i64_side: &Expression<'ast>,
    ) -> bool {
        if matches!(
            i64_side,
            Expression::Literal {
                value: Literal::Int(_),
                ..
            }
        ) {
            return self.expression_is_codegen_i32(i32_side)
                || self.expression_promotes_to_i32_in_compare(i32_side);
        }
        // WJ `int` identifiers are i64. Preferring i32 splits `month <= 12` / `n > 0`
        // (P3.329 / P3.336). Soft-promoted while counters are already Type::Int32.
        if let Expression::Identifier { name, .. } = i64_side {
            if matches!(self.local_var_types.get(name.as_str()), Some(Type::Int))
                || matches!(
                    self.local_var_types.get(name.as_str()),
                    Some(Type::Custom(n)) if n == "int" || n == "i64"
                )
            {
                return false;
            }
        }
        if self.expression_promotes_to_i32_in_compare(i32_side) {
            if let Expression::Identifier { name, .. } = i64_side {
                if self.local_var_types.get(name.as_str()).is_some_and(|t| {
                    matches!(t, Type::Int32) || matches!(t, Type::Custom(n) if n == "i32")
                }) {
                    return true;
                }
            }
        }
        self.int_type_for_mixed_int_codegen(i64_side) == IntType::I32
            || self
                .infer_expression_type(i64_side)
                .as_ref()
                .is_some_and(|t| {
                    matches!(t, Type::Int32) || matches!(t, Type::Custom(n) if n == "i32")
                })
    }

    /// P3.347: `cy + dy` with i32 for-range `dy` and WJ `int` coord local `cy` must unify to i32
    /// (not `dy as i64`). Comparisons keep [`comparison_should_prefer_i32_over_i64`] (P3.329).
    pub(in crate::codegen::rust) fn mixed_arith_should_prefer_i32_over_i64(
        &self,
        i32_side: &Expression<'ast>,
        i64_side: &Expression<'ast>,
    ) -> bool {
        if !self.expression_is_codegen_i32(i32_side) {
            return false;
        }
        if matches!(
            i64_side,
            Expression::Literal {
                value: Literal::Int(_),
                ..
            }
        ) {
            return true;
        }
        if let Expression::Identifier { name, .. } = i64_side {
            if self.current_function_params.iter().any(|p| {
                p.name == *name && matches!(&p.type_, Type::Int)
            }) {
                return false;
            }
            if self.explicit_wj_int_annotated_locals.contains(name) {
                return false;
            }
            if self.literal_init_wj_int_loop_counters.contains(name) {
                return false;
            }
            return matches!(
                self.local_var_types.get(name.as_str()),
                Some(Type::Int) | Some(Type::Int32)
            ) || matches!(
                self.local_var_types.get(name.as_str()),
                Some(Type::Custom(n)) if n == "i32"
            );
        }
        self.int_type_for_mixed_int_codegen(i64_side) == IntType::I32
    }

    pub(in crate::codegen::rust) fn expression_is_codegen_i32(&self, expr: &Expression<'ast>) -> bool {
        if let Expression::Identifier { name, .. } = expr {
            if self.codegen_i32_binding_names.contains(name) {
                return true;
            }
            if self.local_var_types.get(name.as_str()).is_some_and(|t| {
                matches!(t, Type::Int32) || matches!(t, Type::Custom(n) if n == "i32")
            }) {
                return true;
            }
        }
        self.expression_promotes_to_i32_in_compare(expr)
    }

    fn expression_promotes_to_i32_in_compare(&self, expr: &Expression<'ast>) -> bool {
        self.int_type_for_mixed_int_codegen(expr) == IntType::I32
            || self
                .infer_expression_type(expr)
                .as_ref()
                .is_some_and(|t| {
                    matches!(t, Type::Int32) || matches!(t, Type::Custom(n) if n == "i32")
                })
    }

    /// P3.323 / P3.326: untyped `let mut x = 0` may emit `_i32` / `_usize` while
    /// `local_var_types` still holds WJ `Int` from a non-int return hint (e.g. `-> string`).
    /// Sync the binding to the emitted width so later assigns do not `as i64` a usize RHS.
    pub(in crate::codegen::rust) fn reconcile_ambiguous_int_local_after_let(
        &mut self,
        name: &str,
        value: &Expression<'ast>,
        emitted_rhs: &str,
    ) {
        let ambiguous_int = match self.local_var_types.get(name) {
            Some(Type::Int) | Some(Type::Int32) => true,
            Some(Type::Custom(n)) => matches!(n.as_str(), "int" | "i64" | "i32"),
            _ => false,
        };
        if !ambiguous_int {
            return;
        }
        // P3.326: numeric inference may emit `0_usize` for index locals while the let
        // still recorded WJ `Int` (String/bool return width hint). Treat as usize.
        if self.explicit_wj_int_annotated_locals.contains(name) {
            return;
        }
        // P3.329: Call/MethodCall WJ `int` results must not become usize via later
        // substring formals (`let plus_pos = find_tz_sign(...)`).
        if matches!(value, Expression::Call { .. } | Expression::MethodCall { .. }) {
            let signed_ret = self.infer_expression_type(value).as_ref().is_some_and(|t| {
                matches!(t, Type::Int | Type::Int32)
                    || matches!(
                        t,
                        Type::Custom(n) if matches!(n.as_str(), "int" | "i64" | "i32")
                    )
            });
            if signed_ret {
                if emitted_rhs.ends_with("_i32")
                    || self.int_type_for_mixed_int_codegen(value) == IntType::I32
                {
                    self.local_var_types.insert(name.to_string(), Type::Int32);
                }
                return;
            }
        }
        if emitted_rhs.ends_with("_usize")
            || self.int_type_for_mixed_int_codegen(value) == IntType::Usize
        {
            self.local_var_types
                .insert(name.to_string(), Type::Custom("usize".into()));
            self.usize_variables.insert(name.to_string());
            return;
        }
        if emitted_rhs.ends_with("_i32")
            || self.int_type_for_mixed_int_codegen(value) == IntType::I32
        {
            self.local_var_types.insert(name.to_string(), Type::Int32);
            return;
        }
        // P3.348: numeric inference may emit `0_u32` for u32 loop peers while let still
        // recorded WJ `Int` from bool/void return width — sync so `while i < count` stays u32.
        if emitted_rhs.ends_with("_u32")
            || self.int_type_for_mixed_int_codegen(value) == IntType::U32
        {
            self.local_var_types.insert(name.to_string(), Type::Uint);
        }
    }


    /// P3.345: `let seg = if segments < 4 { 4 } else { segments }` in void methods must
    /// register as i32 so `while i < seg` promotes the literal-init counter.
    pub(in crate::codegen::rust) fn if_else_binding_should_be_i32(
        &self,
        then_branch: &Expression<'ast>,
        else_branch: &Expression<'ast>,
    ) -> bool {
        self.if_branch_is_i32_width(then_branch) && self.if_branch_is_i32_width(else_branch)
    }

    fn if_branch_is_i32_width(&self, expr: &Expression<'ast>) -> bool {
        match expr {
            Expression::Literal {
                value: Literal::Int(n),
                ..
            } => *n >= 0,
            Expression::Identifier { name, .. } => self.identifier_is_i32_formal_or_binding(name),
            Expression::Cast { type_, .. } => {
                matches!(type_, Type::Int32) || matches!(type_, Type::Custom(n) if n == "i32")
            }
            _ => self.int_type_for_mixed_int_codegen(expr) == IntType::I32,
        }
    }

    fn identifier_is_i32_formal_or_binding(&self, name: &str) -> bool {
        if self.codegen_i32_binding_names.contains(name) {
            return true;
        }
        if self.local_var_types.get(name).is_some_and(|t| {
            matches!(t, Type::Int32) || matches!(t, Type::Custom(n) if n == "i32")
        }) {
            return true;
        }
        self.current_function_params.iter().any(|p| {
            p.name == name
                && (matches!(p.type_, Type::Int32)
                    || matches!(&p.type_, Type::Custom(n) if n == "i32"))
        })
    }

    fn expression_has_u32_width_in_tree(&self, expr: &Expression<'ast>) -> bool {
        match expr {
            Expression::Identifier { name, .. } => {
                self.local_var_types.get(name.as_str()).is_some_and(|t| {
                    matches!(t, Type::Uint) || matches!(t, Type::Custom(n) if n == "u32")
                }) || self.int_type_for_mixed_int_codegen(expr) == IntType::U32
            }
            Expression::Binary { left, right, .. } => {
                self.expression_has_u32_width_in_tree(left)
                    || self.expression_has_u32_width_in_tree(right)
            }
            Expression::Unary { operand, .. } => self.expression_has_u32_width_in_tree(operand),
            Expression::Cast { expr, type_, .. } => {
                matches!(type_, Type::Uint)
                    || matches!(type_, Type::Custom(n) if n == "u32")
                    || self.expression_has_u32_width_in_tree(expr)
            }
            Expression::MethodCall { .. } | Expression::Call { .. } => {
                self.int_type_for_mixed_int_codegen(expr) == IntType::U32
                    || self
                        .infer_expression_type(expr)
                        .as_ref()
                        .is_some_and(|t| {
                            matches!(t, Type::Uint)
                                || matches!(t, Type::Custom(n) if n == "u32")
                        })
            }
            _ => false,
        }
    }

    fn expression_has_i32_width_in_tree(&self, expr: &Expression<'ast>) -> bool {
        match expr {
            Expression::Identifier { name, .. } => {
                self.codegen_i32_binding_names.contains(name)
                    || self.local_var_types.get(name.as_str()).is_some_and(|t| {
                        matches!(t, Type::Int32)
                            || matches!(t, Type::Custom(n) if n == "i32")
                    })
                    || self.int_type_for_mixed_int_codegen(expr) == IntType::I32
            }
            Expression::Binary { left, right, .. } => {
                self.expression_has_i32_width_in_tree(left)
                    || self.expression_has_i32_width_in_tree(right)
            }
            Expression::Unary { operand, .. } => self.expression_has_i32_width_in_tree(operand),
            Expression::Cast { expr, type_, .. } => {
                matches!(type_, Type::Int32)
                    || matches!(type_, Type::Custom(n) if n == "i32")
                    || self.expression_has_i32_width_in_tree(expr)
            }
            _ => false,
        }
    }

    /// P3.323: untyped `let mut i = 0` + `while i < 4` — treat counter as i32, not WJ `int`/i64.
    pub(in crate::codegen::rust) fn promote_ambiguous_int_loop_counter_in_while_condition(
        &mut self,
        condition: &Expression<'ast>,
    ) {
        use crate::parser::BinaryOp;
        let Expression::Binary { left, right, op, .. } = condition else {
            return;
        };
        if !matches!(
            op,
            BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge
        ) {
            return;
        }
        let ident_name =
            |expr: &Expression<'ast>, lit: &Expression<'ast>| -> Option<String> {
                match (expr, lit) {
                    (
                        Expression::Identifier { name, .. },
                        Expression::Literal {
                            value: Literal::Int(n),
                            ..
                        },
                    ) if (0..=4096).contains(n) => Some(name.to_string()),
                    (
                        Expression::Literal {
                            value: Literal::Int(n),
                            ..
                        },
                        Expression::Identifier { name, .. },
                    ) if (0..=4096).contains(n) => Some(name.to_string()),
                    _ => None,
                }
            };
        if let Some(name) = ident_name(left, right).or_else(|| ident_name(right, left)) {
            if matches!(self.local_var_types.get(name.as_str()), Some(Type::Int))
                && self.literal_init_wj_int_loop_counters.contains(&name)
            {
                // P3.329: int-/struct-int returns already chose WJ `int` (i64) at the let.
                if let Some(ret_ty) = &self.current_function_return_type {
                    let peeled = Self::peel_option_result_payload(ret_ty);
                    match peeled {
                        Type::Int | Type::Int32 | Type::Uint => return,
                        Type::Custom(n)
                            if matches!(
                                n.as_str(),
                                "int" | "i64" | "i32" | "u32" | "u64" | "usize"
                            ) =>
                        {
                            return;
                        }
                        Type::Custom(n)
                            if self.struct_fields_include_wj_int(n)
                                && !self.struct_fields_include_vec_or_array(n) =>
                        {
                            return;
                        }
                        _ => {}
                    }
                }
                self.local_var_types.insert(name.clone(), Type::Int32);
                self.codegen_i32_binding_names.insert(name.clone());
                return;
            }
        }

        if let Expression::Identifier { name, .. } = left {
            if self.expression_has_i32_width_in_tree(right) {
                let is_counter = self.literal_init_wj_int_loop_counters.contains(name)
                    || self.codegen_i32_binding_names.contains(name)
                    || matches!(
                        self.local_var_types.get(name.as_str()),
                        Some(Type::Int) | Some(Type::Int32)
                    );
                if is_counter {
                    self.local_var_types.insert(name.clone(), Type::Int32);
                    self.codegen_i32_binding_names.insert(name.clone());
                    self.usize_variables.remove(name);
                }
            } else if self.expression_has_u32_width_in_tree(right) {
                let is_counter = self.literal_init_wj_int_loop_counters.contains(name)
                    || matches!(
                        self.local_var_types.get(name.as_str()),
                        Some(Type::Int) | Some(Type::Uint)
                    );
                if is_counter {
                    self.local_var_types.insert(name.clone(), Type::Uint);
                    self.usize_variables.remove(name);
                }
            }
        }
    }


    /// P3.348: u32 loop counter vs u32 bound — do not widen either side to i64.
    pub(in crate::codegen::rust) fn comparison_should_prefer_u32_over_i64(
        &self,
        u32_side: &Expression<'ast>,
        i64_side: &Expression<'ast>,
    ) -> bool {
        let u32_counter = match u32_side {
            Expression::Identifier { name, .. } => {
                self.literal_init_wj_int_loop_counters.contains(name)
                    || self.local_var_types.get(name.as_str()).is_some_and(|t| {
                        matches!(t, Type::Uint)
                            || matches!(t, Type::Custom(n) if n == "u32")
                    })
                    || self.int_type_for_mixed_int_codegen(u32_side) == IntType::U32
            }
            _ => self.expression_promotes_to_u32_in_compare(u32_side),
        };
        if !u32_counter {
            return false;
        }
        if matches!(
            i64_side,
            Expression::Literal {
                value: Literal::Int(n),
                ..
            } if *n >= 0
        ) {
            return true;
        }
        self.expression_promotes_to_u32_in_compare(i64_side)
            || self.int_type_for_mixed_int_codegen(i64_side) == IntType::U32
            || self.infer_expression_type(i64_side).as_ref().is_some_and(|t| {
                matches!(t, Type::Uint) || matches!(t, Type::Custom(n) if n == "u32")
            })
    }

    /// P3.324: u32 locals / fields vs untyped int literals — do not widen to u64.
    pub(in crate::codegen::rust) fn comparison_should_prefer_u32_over_u64(
        &self,
        u32_side: &Expression<'ast>,
        u64_side: &Expression<'ast>,
    ) -> bool {
        if matches!(
            u64_side,
            Expression::Literal {
                value: Literal::Int(n),
                ..
            } if *n >= 0
        ) {
            return true;
        }
        if self.expression_promotes_to_u32_in_compare(u32_side) {
            if let Expression::Identifier { name, .. } = u64_side {
                if matches!(self.local_var_types.get(name.as_str()), Some(Type::Int))
                    || matches!(
                        self.local_var_types.get(name.as_str()),
                        Some(Type::Custom(n)) if n == "uint"
                    )
                {
                    return true;
                }
            }
        }
        self.int_type_for_mixed_int_codegen(u64_side) == IntType::U32
            || self
                .infer_expression_type(u64_side)
                .as_ref()
                .is_some_and(|t| {
                    matches!(t, Type::Uint)
                        || matches!(t, Type::Custom(n) if n == "u32")
                })
    }

    fn expression_promotes_to_u32_in_compare(&self, expr: &Expression<'ast>) -> bool {
        self.int_type_for_mixed_int_codegen(expr) == IntType::U32
            || self
                .infer_expression_type(expr)
                .as_ref()
                .is_some_and(|t| {
                    matches!(t, Type::Uint)
                        || matches!(t, Type::Custom(n) if n == "u32")
                })
    }

    /// Operand type for driving int literal suffixes in binary ops (`idx + 1` → `1_usize`).
    pub(in crate::codegen::rust) fn peer_type_for_int_literal_operand(
        &self,
        expr: &Expression<'ast>,
    ) -> Option<Type> {
        if let Expression::Identifier { name, .. } = expr {
            // usize index/len counters beat return-inferred Int32 (P3.311/P3.314).
            if self.usize_variables.contains(name) {
                return Some(Type::Custom("usize".into()));
            }
            if self.codegen_i32_binding_names.contains(name) {
                return Some(Type::Int32);
            }
            if let Some(w) = self.local_int_rust_type_name_excluding_ambiguous_int(name) {
                return Self::parser_type_from_rust_int_name(w);
            }
        }
        if let Expression::Binary { left, right, op, .. } = expr {
            use crate::parser::BinaryOp;
            if matches!(
                op,
                BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod
            ) {
                return self
                    .peer_type_for_int_literal_operand(left)
                    .or_else(|| self.peer_type_for_int_literal_operand(right));
            }
        }
        if self.infer_expression_type_is_usize(expr) {
            return Some(Type::Custom("usize".into()));
        }
        // P3.322: ambiguous WJ `int` locals may emit as i32 while `local_var_types` stays `Int`.
        if self.int_type_for_mixed_int_codegen(expr) == crate::type_inference::IntType::I32 {
            return Some(Type::Int32);
        }
        self.infer_expression_type(expr).filter(|t| {
            Self::assignment_target_needs_int_codegen_context(t)
                && Self::int_type_from_assignment_target(t).is_some()
        })
    }

    pub(in crate::codegen::rust) fn expr_is_usize_loop_counter(
        &self,
        expr: &Expression<'ast>,
    ) -> bool {
        matches!(expr, Expression::Identifier { name, .. } if self.usize_variables.contains(name))
    }

    /// Map parser [`Type`] to [`crate::type_inference::IntType`] for mixed-integer `as T` codegen.
    pub(in crate::codegen::rust) fn parser_type_to_promotion_int_type(
        ty: &Type,
    ) -> Option<crate::type_inference::IntType> {
        use crate::type_inference::IntType;
        match ty {
            // Windjammer `int` is i64 in Rust codegen — never promote as i32 (P3.267).
            Type::Int => Some(IntType::I64),
            Type::Int32 => Some(IntType::I32),
            Type::Uint => Some(IntType::U32),
            Type::Custom(name) => match name.as_str() {
                "i8" => Some(IntType::I8),
                "i16" => Some(IntType::I16),
                "i32" => Some(IntType::I32),
                "i64" => Some(IntType::I64),
                "isize" => Some(IntType::Isize),
                "u8" => Some(IntType::U8),
                "u16" => Some(IntType::U16),
                "u32" => Some(IntType::U32),
                "u64" => Some(IntType::U64),
                "usize" => Some(IntType::Usize),
                _ => None,
            },
            Type::Reference(inner) | Type::MutableReference(inner) => {
                Self::parser_type_to_promotion_int_type(inner.as_ref())
            }
            _ => None,
        }
    }

    /// Integer kind for binary mixed-type casts: use annotated types for params/fields when
    /// inference disagrees (e.g. `a: u32` must not be treated as `i32`).
    pub(in crate::codegen::rust) fn int_type_for_mixed_int_codegen(
        &self,
        expr: &Expression<'ast>,
    ) -> crate::type_inference::IntType {
        let eng = if let Some(ni) = &self.numeric_inference {
            ni.get_int_type(expr)
        } else {
            return crate::type_inference::IntType::Unknown;
        };
        match expr {
            Expression::Identifier { name, .. } => {
                if self.usize_variables.contains(name) {
                    return crate::type_inference::IntType::Usize;
                }
                if let Some(t) = self.local_var_types.get(name.as_str()) {
                    if let Some(a) = Self::parser_type_to_promotion_int_type(t) {
                        if a == IntType::I64 {
                            if let Some(w) = self.local_int_rust_type_name_excluding_ambiguous_int(name)
                            {
                                if w == "i32" {
                                    return IntType::I32;
                                }
                            }
                            if eng == IntType::I32 {
                                return IntType::I32;
                            }
                        }
                        return a;
                    }
                }
                if let Some(a) = self
                    .infer_expression_type(expr)
                    .as_ref()
                    .and_then(Self::parser_type_to_promotion_int_type)
                {
                    return a;
                }
                eng
            }
            Expression::FieldAccess { .. } => {
                // For field accesses, prefer codegen type inference over int inference
                // engine. If the codegen can determine the field type (via struct_field_types),
                // use it. If not (e.g., ambiguous struct names across modules), return
                // Unknown to prevent incorrect casts. The int inference engine may resolve
                // field types through a different struct with the same name, producing
                // wrong results.
                if self.expression_produces_usize(expr) {
                    return crate::type_inference::IntType::Usize;
                }
                if let Some(a) = self
                    .infer_expression_type(expr)
                    .as_ref()
                    .and_then(Self::parser_type_to_promotion_int_type)
                {
                    return a;
                }
                crate::type_inference::IntType::Unknown
            }
            _ => {
                if self.expression_produces_usize(expr) {
                    return crate::type_inference::IntType::Usize;
                }
                eng
            }
        }
    }
}
