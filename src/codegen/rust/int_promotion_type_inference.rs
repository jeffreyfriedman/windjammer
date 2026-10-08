//! Mixed-integer promotion for `as T` codegen.

use crate::codegen::rust::CodeGenerator;
use crate::parser::{Expression, Literal, Type};
use crate::type_inference::{promote_types, IntType};

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

    /// P3.497: while-slot i32 must not win over an i64 timestamp / WJ `int` identifier.
    /// P3.309 i32 counters vs int *literals* still keep forced i32.
    pub(in crate::codegen::rust) fn comparison_should_keep_i64_over_forced_i32(
        &self,
        left: &Expression<'ast>,
        right: &Expression<'ast>,
        left_ty: IntType,
        right_ty: IntType,
    ) -> bool {
        let (i64_side, _i32_side) = if left_ty == IntType::I64 && right_ty == IntType::I32 {
            (left, right)
        } else if right_ty == IntType::I64 && left_ty == IntType::I32 {
            (right, left)
        } else if left_ty == IntType::I64 && right_ty == IntType::I64 {
            return true;
        } else {
            return false;
        };
        if matches!(
            i64_side,
            Expression::Literal {
                value: Literal::Int(_),
                ..
            }
        ) {
            return false;
        }
        self.expression_is_stable_i64_compare_peer(i64_side)
    }

    fn expression_is_stable_i64_compare_peer(&self, expr: &Expression<'ast>) -> bool {
        if self.int_type_for_mixed_int_codegen(expr) == IntType::I64 {
            return true;
        }
        self.infer_expression_type(expr).is_some_and(|t| {
            matches!(t, Type::Int) || matches!(t, Type::Custom(n) if n == "int" || n == "i64")
        })
    }

    /// P3.369: Typed i64 entity ids / WJ `int` vs i32-coord literal (`old_parent >= 0`).
    pub(in crate::codegen::rust) fn comparison_should_prefer_i64_over_i32(
        &self,
        i64_side: &Expression<'ast>,
        literal_side: &Expression<'ast>,
    ) -> bool {
        if !matches!(
            literal_side,
            Expression::Literal {
                value: Literal::Int(_),
                ..
            }
        ) {
            return false;
        }
        if self.int_type_for_mixed_int_codegen(i64_side) == crate::type_inference::IntType::I64 {
            return true;
        }
        if self.infer_expression_type(i64_side).is_some_and(|t| {
            matches!(t, Type::Int) || matches!(t, Type::Custom(n) if n == "int" || n == "i64")
        }) {
            return true;
        }
        if let Expression::Identifier { name, .. } = i64_side {
            if self.local_var_types.get(name.as_str()).is_some_and(|t| {
                matches!(t, Type::Int) || matches!(t, Type::Custom(n) if n == "int" || n == "i64")
            }) {
                return true;
            }
            if self.current_function_params.iter().any(|p| {
                p.name == *name
                    && (matches!(&p.type_, Type::Int)
                        || matches!(&p.type_, Type::Custom(n) if n == "int" || n == "i64"))
            }) {
                return true;
            }
        }
        false
    }

    /// Library/file `const NAME: int` — type-driven, not a const-name list.
    fn identifier_is_wj_int_module_const(&self, name: &str) -> bool {
        self.module_const_type_for_binding(name).is_some_and(|t| {
            matches!(t, Type::Int)
                || matches!(t, Type::Custom(n) if matches!(n.as_str(), "int" | "i64"))
        })
    }

    /// P3.353: WJ `int` const/locals in void/i32-coord builders (not `int` params).
    pub(in crate::codegen::rust) fn wj_int_coord_builder_operand(
        &self,
        expr: &Expression<'ast>,
    ) -> bool {
        if !self.function_prefers_i32_coord_locals() {
            return false;
        }
        let Expression::Identifier { name, .. } = expr else {
            return false;
        };
        if self.explicit_wj_int_annotated_locals.contains(name) {
            return false;
        }
        if self.identifier_is_wj_int_module_const(name) {
            return true;
        }
        if self.current_function_params.iter().any(|p| {
            p.name == *name
                && (matches!(&p.type_, Type::Int)
                    || matches!(
                        &p.type_,
                        Type::Custom(n) if matches!(n.as_str(), "int" | "i64")
                    ))
        }) {
            return false;
        }
        if self.codegen_i32_binding_names.contains(name) {
            return true;
        }
        self.arithmetic_prefers_i32_ambiguous_int_local(expr)
    }

    /// P3.353: WJ `int` locals in voxel/set_if coord builders (not formal `int` params).
    pub(in crate::codegen::rust) fn arithmetic_prefers_i32_ambiguous_int_local(
        &self,
        expr: &Expression<'ast>,
    ) -> bool {
        if !self.function_prefers_i32_coord_locals() {
            return false;
        }
        let Expression::Identifier { name, .. } = expr else {
            return false;
        };
        if self.explicit_wj_int_annotated_locals.contains(name) {
            return false;
        }
        if self.literal_init_wj_int_loop_counters.contains(name) {
            return false;
        }
        if self.current_function_params.iter().any(|p| {
            p.name == *name
                && (matches!(&p.type_, Type::Int)
                    || matches!(
                        &p.type_,
                        Type::Custom(n) if matches!(n.as_str(), "int" | "i64")
                    ))
        }) {
            return false;
        }
        if self.codegen_i32_binding_names.contains(name) {
            return true;
        }
        matches!(self.local_var_types.get(name.as_str()), Some(Type::Int32))
            || matches!(
                self.local_var_types.get(name.as_str()),
                Some(Type::Custom(n)) if n == "i32"
            )
    }

    /// P3.347: `cy + dy` with i32 for-range `dy` and WJ `int` coord local `cy` must unify to i32
    /// (not `dy as i64`). Comparisons keep [`comparison_should_prefer_i32_over_i64`] (P3.329).
    pub(in crate::codegen::rust) fn mixed_arith_should_prefer_i32_over_i64(
        &self,
        i32_side: &Expression<'ast>,
        i64_side: &Expression<'ast>,
    ) -> bool {
        let i32_side_ok = self.expression_is_codegen_i32(i32_side)
            || self.arithmetic_prefers_i32_ambiguous_int_local(i32_side)
            || self.wj_int_coord_builder_operand(i32_side);
        if !i32_side_ok {
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
            if self
                .current_function_params
                .iter()
                .any(|p| p.name == *name && matches!(&p.type_, Type::Int))
            {
                return false;
            }
            if self.explicit_wj_int_annotated_locals.contains(name) {
                return false;
            }
            if self.literal_init_wj_int_loop_counters.contains(name) {
                return false;
            }
            if self.identifier_is_wj_int_module_const(name) {
                return true;
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

    pub(in crate::codegen::rust) fn expression_is_codegen_i32(
        &self,
        expr: &Expression<'ast>,
    ) -> bool {
        if let Expression::Identifier { name, .. } = expr {
            if self.current_function_params.iter().any(|p| {
                p.name == *name
                    && (matches!(&p.type_, Type::Int32)
                        || matches!(&p.type_, Type::Custom(n) if n == "i32"))
            }) {
                return true;
            }
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
            || self.infer_expression_type(expr).as_ref().is_some_and(|t| {
                matches!(t, Type::Int32) || matches!(t, Type::Custom(n) if n == "i32")
            })
    }

    /// P3.353: after `let cx = (VIEWER_GRID as i32) / 2_i32`, keep binding width i32 for downstream ops.
    pub(in crate::codegen::rust) fn sync_i32_coord_binding_after_let(
        &mut self,
        name: &str,
        value_str: &str,
    ) {
        if !self.function_prefers_i32_coord_locals() {
            return;
        }
        if self.explicit_wj_int_annotated_locals.contains(name) {
            return;
        }
        // P3_ATOMIC_I64_SYNC_I32_NOMINAL_GUARD: do not overwrite AtomicI64/struct locals when nested lit emitted _i32.
        if let Some(ty) = self.local_var_types.get(name) {
            let is_int_width = matches!(ty, Type::Int | Type::Int32 | Type::Uint)
                || matches!(
                    ty,
                    Type::Custom(n) if matches!(
                        n.as_str(),
                        "int" | "i64" | "i32" | "u32" | "uint"
                    )
                );
            if !is_int_width {
                return;
            }
        }
        let i32_emitted = value_str.contains("_i32")
            || value_str.contains(" as i32")
            || value_str.contains("as i32)");
        if !i32_emitted {
            return;
        }
        self.local_var_types.insert(name.to_string(), Type::Int32);
        self.codegen_i32_binding_names.insert(name.to_string());
        self.usize_variables.remove(name);
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
        // P3.454: rustc sees the emitted suffix. Mixed-int inference must not
        // paint `let mut colon_at = -1_i64` as usize (then `colon_at = j` skips
        // the i64 cast). Negative sentinels stay WJ `int`.
        // P3.568: `let start = i + (marker_len as i64)` emits i64 width without a
        // trailing `_i64` suffix — do not let mixed-int Usize inference repaint the
        // binding as usize (then `let mut j = start` / `j < n` stay broken).
        if crate::codegen::rust::type_casting::expression_is_negative_int_init(value)
            || emitted_rhs.ends_with("_i64")
            || emitted_rhs.contains(" as i64")
        {
            self.local_var_types.insert(name.to_string(), Type::Int);
            self.codegen_i32_binding_names.remove(name);
            self.usize_variables.remove(name);
            return;
        }
        // P3.568: WJ `int` + `strings.len`/`usize` arithmetic keeps i64 even when
        // `int_type_for_mixed_int_codegen` reports Usize for the AST mix.
        if let Expression::Binary {
            op, left, right, ..
        } = value
        {
            if matches!(
                op,
                crate::parser::BinaryOp::Add
                    | crate::parser::BinaryOp::Sub
                    | crate::parser::BinaryOp::Mul
                    | crate::parser::BinaryOp::Div
                    | crate::parser::BinaryOp::Mod
            ) && (self.comparison_other_side_needs_len_as_i64(left)
                || self.comparison_other_side_needs_len_as_i64(right))
            {
                self.local_var_types.insert(name.to_string(), Type::Int);
                self.codegen_i32_binding_names.remove(name);
                self.usize_variables.remove(name);
                return;
            }
        }
        // P3.329: Call/MethodCall WJ `int` results must not become usize via later
        // substring formals (`let plus_pos = find_tz_sign(...)`).
        if matches!(
            value,
            Expression::Call { .. } | Expression::MethodCall { .. }
        ) {
            let ret_ty = self.infer_expression_type(value);
            let signed_ret = ret_ty.as_ref().is_some_and(|t| {
                matches!(t, Type::Int | Type::Int32)
                    || matches!(
                        t,
                        Type::Custom(n) if matches!(n.as_str(), "int" | "i64" | "i32")
                    )
            });
            if signed_ret {
                // P3.371: WJ `int` / i64 callee results stay i64 — do not narrow bindings
                // to i32 because numeric inference tagged the call after `< 0` compares.
                let wj_int_ret = ret_ty.as_ref().is_some_and(|t| {
                    matches!(t, Type::Int)
                        || matches!(t, Type::Custom(n) if n == "int" || n == "i64")
                });
                if wj_int_ret {
                    if emitted_rhs.ends_with("_i32") {
                        self.local_var_types.insert(name.to_string(), Type::Int32);
                    }
                    return;
                }
                if emitted_rhs.ends_with("_i32")
                    || self.int_type_for_mixed_int_codegen(value) == IntType::I32
                {
                    self.local_var_types.insert(name.to_string(), Type::Int32);
                }
                return;
            }
        }
        // WDB-327: do NOT use `contains("_usize")` — identifiers like `best_idx_usize`
        // inside `open_set[best_idx_usize].x` falsely widened i32 field loads to usize
        // (`current_x == goal_x as usize`). Only real usize *literal* suffixes / casts.
        // P3.679: an emitted `_i32` suffix is the width rustc sees (`let mut i: i32 = 0_i32`).
        // Mixed-int Usize inference from a later substring/index use must not repaint
        // that binding, or `while i < 64` emits `64_usize`.
        if emitted_rhs.ends_with("_i32")
            && !Self::emitted_rhs_indicates_usize_literal_width(emitted_rhs)
        {
            self.local_var_types.insert(name.to_string(), Type::Int32);
            self.codegen_i32_binding_names.insert(name.to_string());
            self.usize_variables.remove(name);
            return;
        }
        if Self::emitted_rhs_indicates_usize_literal_width(emitted_rhs)
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

    /// Emitted RHS is usize-width from a literal suffix / cast — not from an identifier
    /// that merely contains the substring `_usize` (WDB-327 / `best_idx_usize`).
    ///
    /// Index payloads are stripped first: `self.depths[node_idx as usize]` loads an
    /// element (WDB-406/395); the index cast must not mark the binding as usize.
    fn emitted_rhs_indicates_usize_literal_width(emitted: &str) -> bool {
        let without_indexes = Self::strip_bracket_regions(emitted);
        if without_indexes.contains(" as usize") {
            return true;
        }
        // `0_usize`, `i + 1_usize`, `(0_usize)` — digit immediately before `_usize`.
        let mut rest = without_indexes.as_str();
        while let Some(idx) = rest.find("_usize") {
            if rest[..idx]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_ascii_digit())
            {
                return true;
            }
            rest = &rest[idx + "_usize".len()..];
        }
        // Bare copy of a usize-named binding: `let x = best_idx_usize`.
        let trimmed = without_indexes
            .trim()
            .trim_matches(|c| c == '(' || c == ')');
        trimmed.ends_with("_usize") && !trimmed.contains('.')
    }

    /// Drop `[...]` regions (incl. nested) so index casts are ignored for value-width.
    fn strip_bracket_regions(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        let mut depth = 0usize;
        for c in s.chars() {
            match c {
                '[' => depth = depth.saturating_add(1),
                ']' => depth = depth.saturating_sub(1),
                _ if depth == 0 => out.push(c),
                _ => {}
            }
        }
        out
    }

    /// WDB-305: untyped `let mut best_count = 0` defaults to return-width i64 while later
    /// `best_count = count` assigns a `u32` (CDLP majority). Prefer the later assign width.
    /// Int width from a later call that passes `name` into a typed formal
    /// (`let status = if … { 404 } else { 400 }` then `error_from_message(status, …)`
    /// where the formal is `u16`).
    pub(in crate::codegen::rust) fn let_binding_int_width_from_later_call_formals(
        &self,
        name: &str,
    ) -> Option<Type> {
        let body: Vec<&crate::parser::Statement> = if !self.full_function_body_snapshot.is_empty() {
            self.full_function_body_snapshot.iter().copied().collect()
        } else {
            self.current_function_body.iter().copied().collect()
        };
        let mut peer = None;
        self.scan_stmts_for_call_arg_int_formal(&body, name, &mut peer);
        peer
    }

    /// WDB-411: `let mut i = 0` later compared to a u32 (`while i < count`) must
    /// peer u32 even when `function_prefers_i32_coord_locals` (f32 return).
    ///
    /// P3.549/P3.329: a small literal while bound (`while month <= 12`) alone would
    /// peer i32, but when `name` is also passed to a WJ `int`/`i64` formal
    /// (`days_in_month(year, month)`), keep int width — never `1_i32` + `12_i32 as i32`.
    /// WDB-328 / P3.403: `let mut i = -1` must stay WJ `int` (i64), not i32 while peers.
    pub(in crate::codegen::rust) fn let_binding_has_negative_int_literal_init(
        &self,
        name: &str,
    ) -> bool {
        let body: Vec<&crate::parser::Statement> = if !self.full_function_body_snapshot.is_empty() {
            self.full_function_body_snapshot.iter().copied().collect()
        } else {
            self.current_function_body.iter().copied().collect()
        };
        Self::find_let_rhs_in_stmts(&body, name)
            .is_some_and(crate::codegen::rust::type_casting::expression_is_negative_int_init)
    }

    pub(in crate::codegen::rust) fn let_binding_int_width_from_later_while_compare(
        &self,
        name: &str,
    ) -> Option<Type> {
        if self.let_binding_has_negative_int_literal_init(name) {
            return Some(Type::Int);
        }
        let body: Vec<&crate::parser::Statement> = if !self.full_function_body_snapshot.is_empty() {
            self.full_function_body_snapshot.iter().copied().collect()
        } else {
            self.current_function_body.iter().copied().collect()
        };
        let mut peer = None;
        self.scan_stmts_for_while_compare_int_peer(&body, name, &mut peer);
        if matches!(peer.as_ref(), Some(Type::Int32))
            || matches!(peer.as_ref(), Some(Type::Custom(n)) if n == "i32")
        {
            let mut formal = None;
            self.scan_stmts_for_call_arg_int_formal(&body, name, &mut formal);
            if matches!(formal.as_ref(), Some(Type::Int))
                || matches!(
                    formal.as_ref(),
                    Some(Type::Custom(n)) if n == "int" || n == "i64"
                )
            {
                return formal;
            }
        }
        peer
    }

    fn scan_stmts_for_while_compare_int_peer(
        &self,
        stmts: &[&crate::parser::Statement<'ast>],
        name: &str,
        peer: &mut Option<Type>,
    ) {
        use crate::parser::Statement;
        for stmt in stmts {
            if peer.is_some() {
                return;
            }
            match stmt {
                Statement::While {
                    condition, body, ..
                } => {
                    self.scan_while_cond_for_compare_int_peer(condition, name, peer);
                    if peer.is_none() {
                        self.scan_stmts_for_while_compare_int_peer(body, name, peer);
                    }
                }
                Statement::For { body, .. } => {
                    self.scan_stmts_for_while_compare_int_peer(body, name, peer);
                }
                Statement::If {
                    then_block,
                    else_block,
                    condition,
                    ..
                } => {
                    self.scan_while_cond_for_compare_int_peer(condition, name, peer);
                    self.scan_stmts_for_while_compare_int_peer(then_block, name, peer);
                    if let Some(eb) = else_block {
                        self.scan_stmts_for_while_compare_int_peer(eb, name, peer);
                    }
                }
                Statement::Match { arms, .. } => {
                    for arm in arms {
                        if let Expression::Block { statements, .. } = arm.body {
                            self.scan_stmts_for_while_compare_int_peer(statements, name, peer);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn scan_while_cond_for_compare_int_peer(
        &self,
        condition: &Expression<'ast>,
        name: &str,
        peer: &mut Option<Type>,
    ) {
        use crate::parser::BinaryOp;
        let Expression::Binary {
            left, right, op, ..
        } = condition
        else {
            return;
        };
        if matches!(op, BinaryOp::And | BinaryOp::Or) {
            self.scan_while_cond_for_compare_int_peer(left, name, peer);
            if peer.is_none() {
                self.scan_while_cond_for_compare_int_peer(right, name, peer);
            }
            return;
        }
        if !matches!(
            op,
            BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge
        ) {
            return;
        }
        let bound = if matches!(left, Expression::Identifier { name: n, .. } if n == name) {
            right
        } else if matches!(right, Expression::Identifier { name: n, .. } if n == name) {
            left
        } else {
            return;
        };
        // P3.323/P3.350: `while i < 512` — small literal bound → i32 counter (not WJ int/i64).
        // WDB-328: `let mut i = -1` / `while i <= 1` keeps i64 peers (`1_i64`, not `1_i32`).
        if matches!(
            bound,
            Expression::Literal {
                value: Literal::Int(n),
                ..
            } if (0..=4096).contains(n)
        ) {
            *peer = Some(if self.let_binding_has_negative_int_literal_init(name) {
                Type::Int
            } else {
                Type::Int32
            });
            return;
        }
        if self.expression_has_i32_width_in_tree(bound) {
            *peer = Some(Type::Int32);
            return;
        }
        if let Some(t) = self.infer_expression_type(bound) {
            if matches!(t, Type::Uint) || matches!(&t, Type::Custom(n) if n == "u32") {
                *peer = Some(Type::Uint);
                return;
            } else if matches!(t, Type::Int32) || matches!(&t, Type::Custom(n) if n == "i32") {
                *peer = Some(Type::Int32);
                return;
            }
        }
        // P3.578: `let half = n / 2` may still be recorded as WJ `int` while emit uses
        // `2_u32` (u32 peer). Resolve Identifier bounds through their let RHS / params.
        if let Expression::Identifier {
            name: bound_name, ..
        } = bound
        {
            let body: Vec<&crate::parser::Statement> =
                if !self.full_function_body_snapshot.is_empty() {
                    self.full_function_body_snapshot.iter().copied().collect()
                } else {
                    self.current_function_body.iter().copied().collect()
                };
            if let Some(rhs) = Self::find_let_rhs_in_stmts(&body, bound_name) {
                if let Some(t) = self.u32_width_from_expr_for_while_peer(rhs) {
                    *peer = Some(t);
                    return;
                }
            }
            if let Some(t) = self.u32_width_from_expr_for_while_peer(bound) {
                *peer = Some(t);
            }
        }
    }

    /// P3.578 / P3.348: walk an expression for a concrete `u32` width (param, local,
    /// `pixel_count() -> u32`, or binary with a u32 operand + untyped int lit).
    fn u32_width_from_expr_for_while_peer(&self, expr: &Expression<'ast>) -> Option<Type> {
        match expr {
            Expression::Identifier { name, .. } => {
                if self.local_var_types.get(name.as_str()).is_some_and(|t| {
                    matches!(t, Type::Uint) || matches!(t, Type::Custom(n) if n == "u32")
                }) {
                    return Some(Type::Uint);
                }
                if self.current_function_params.iter().any(|p| {
                    p.name == *name
                        && (matches!(p.type_, Type::Uint)
                            || matches!(&p.type_, Type::Custom(n) if n == "u32"))
                }) {
                    return Some(Type::Uint);
                }
                let body: Vec<&crate::parser::Statement> =
                    if !self.full_function_body_snapshot.is_empty() {
                        self.full_function_body_snapshot.iter().copied().collect()
                    } else {
                        self.current_function_body.iter().copied().collect()
                    };
                if let Some(rhs) = Self::find_let_rhs_in_stmts(&body, name) {
                    return self.u32_width_from_expr_for_while_peer(rhs);
                }
                None
            }
            Expression::Binary { left, right, .. } => {
                let l = self.u32_width_from_expr_for_while_peer(left);
                let r = self.u32_width_from_expr_for_while_peer(right);
                let untyped_lit = |e: &Expression| {
                    matches!(
                        e,
                        Expression::Literal {
                            value: Literal::Int(_),
                            ..
                        }
                    )
                };
                match (l, r) {
                    (Some(t), _) | (_, Some(t)) => Some(t),
                    (None, None)
                        if (untyped_lit(left) || untyped_lit(right))
                            && (self.infer_expression_type(left).is_some_and(|t| {
                                matches!(t, Type::Uint)
                                    || matches!(t, Type::Custom(n) if n == "u32")
                            }) || self.infer_expression_type(right).is_some_and(|t| {
                                matches!(t, Type::Uint)
                                    || matches!(t, Type::Custom(n) if n == "u32")
                            })) =>
                    {
                        Some(Type::Uint)
                    }
                    _ => None,
                }
            }
            Expression::Call { .. } | Expression::MethodCall { .. } => {
                self.infer_expression_type(expr).and_then(|t| {
                    if matches!(t, Type::Uint) || matches!(&t, Type::Custom(n) if n == "u32") {
                        Some(Type::Uint)
                    } else {
                        None
                    }
                })
            }
            Expression::Cast { type_, .. } => {
                if matches!(type_, Type::Uint) || matches!(type_, Type::Custom(n) if n == "u32") {
                    Some(Type::Uint)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn scan_stmts_for_call_arg_int_formal(
        &self,
        stmts: &[&crate::parser::Statement<'ast>],
        name: &str,
        peer: &mut Option<Type>,
    ) {
        use crate::parser::Statement;
        for stmt in stmts {
            match stmt {
                Statement::Expression { expr, .. }
                | Statement::Return {
                    value: Some(expr), ..
                } => {
                    self.scan_expr_for_call_arg_int_formal(expr, name, peer);
                }
                Statement::Let { value, .. } | Statement::Assignment { value, .. } => {
                    self.scan_expr_for_call_arg_int_formal(value, name, peer);
                }
                Statement::While { body, .. } | Statement::For { body, .. } => {
                    self.scan_stmts_for_call_arg_int_formal(body, name, peer);
                }
                Statement::If {
                    then_block,
                    else_block,
                    condition,
                    ..
                } => {
                    self.scan_expr_for_call_arg_int_formal(condition, name, peer);
                    self.scan_stmts_for_call_arg_int_formal(then_block, name, peer);
                    if let Some(eb) = else_block {
                        self.scan_stmts_for_call_arg_int_formal(eb, name, peer);
                    }
                }
                Statement::Match { arms, value, .. } => {
                    self.scan_expr_for_call_arg_int_formal(value, name, peer);
                    for arm in arms {
                        if let Expression::Block { statements, .. } = arm.body {
                            self.scan_stmts_for_call_arg_int_formal(statements, name, peer);
                        } else {
                            self.scan_expr_for_call_arg_int_formal(arm.body, name, peer);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn scan_expr_for_call_arg_int_formal(
        &self,
        expr: &Expression<'ast>,
        name: &str,
        peer: &mut Option<Type>,
    ) {
        match expr {
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                for (i, (_, arg)) in arguments.iter().enumerate() {
                    if matches!(arg, Expression::Identifier { name: n, .. } if n == name) {
                        if let Some(ty) = self.callee_int_formal_type(function, i, arguments.len())
                        {
                            *peer = Some(ty);
                            return;
                        }
                    }
                    self.scan_expr_for_call_arg_int_formal(arg, name, peer);
                    if peer.is_some() {
                        return;
                    }
                }
                self.scan_expr_for_call_arg_int_formal(function, name, peer);
            }
            Expression::MethodCall {
                object,
                arguments,
                method,
                ..
            } => {
                for (i, (_, arg)) in arguments.iter().enumerate() {
                    if matches!(arg, Expression::Identifier { name: n, .. } if n == name) {
                        if let Some(ty) =
                            self.method_int_formal_type(object, method, i, arguments.len())
                        {
                            *peer = Some(ty);
                            return;
                        }
                    }
                    self.scan_expr_for_call_arg_int_formal(arg, name, peer);
                    if peer.is_some() {
                        return;
                    }
                }
                self.scan_expr_for_call_arg_int_formal(object, name, peer);
            }
            Expression::Block { statements, .. } => {
                self.scan_stmts_for_call_arg_int_formal(statements, name, peer);
            }
            Expression::Binary { left, right, .. } => {
                self.scan_expr_for_call_arg_int_formal(left, name, peer);
                if peer.is_none() {
                    self.scan_expr_for_call_arg_int_formal(right, name, peer);
                }
            }
            Expression::Unary { operand, .. } => {
                self.scan_expr_for_call_arg_int_formal(operand, name, peer);
            }
            _ => {}
        }
    }

    fn callee_int_formal_type(
        &self,
        function: &Expression<'ast>,
        arg_index: usize,
        arg_count: usize,
    ) -> Option<Type> {
        let name = match function {
            Expression::Identifier { name, .. } => name.clone(),
            Expression::FieldAccess { object, field, .. } => {
                if let Expression::Identifier { name, .. } = object {
                    format!("{name}::{field}")
                } else {
                    field.clone()
                }
            }
            _ => return None,
        };
        self.int_formal_from_resolved_name(&name, arg_index, arg_count)
    }

    fn method_int_formal_type(
        &self,
        object: &Expression<'ast>,
        method: &str,
        arg_index: usize,
        arg_count: usize,
    ) -> Option<Type> {
        let recv = self.infer_expression_type(object)?;
        let recv_name = match &recv {
            Type::Custom(n) | Type::Parameterized(n, _) => n.as_str(),
            Type::Reference(inner) | Type::MutableReference(inner) => match inner.as_ref() {
                Type::Custom(n) | Type::Parameterized(n, _) => n.as_str(),
                _ => return None,
            },
            _ => return None,
        };
        let key = format!("{recv_name}::{method}");
        self.int_formal_from_resolved_name(&key, arg_index, arg_count)
            .or_else(|| self.int_formal_from_resolved_name(method, arg_index, arg_count))
    }

    fn int_formal_from_resolved_name(
        &self,
        name: &str,
        arg_index: usize,
        _arg_count: usize,
    ) -> Option<Type> {
        let sig = self.signature_registry.get_signature(name).or_else(|| {
            name.rsplit_once("::")
                .and_then(|(_, leaf)| self.signature_registry.get_signature(leaf))
        })?;
        let pidx = sig.arg_param_index(arg_index);
        let ty = sig
            .formal_param_type(pidx)
            .or_else(|| sig.param_types.get(pidx))?;
        let bare = match ty {
            Type::Reference(inner) | Type::MutableReference(inner) => inner.as_ref(),
            other => other,
        };
        if Self::assignment_target_needs_int_codegen_context(bare) {
            Some(bare.clone())
        } else {
            None
        }
    }

    pub(in crate::codegen::rust) fn mut_int_local_peer_width_from_later_assigns(
        &self,
        name: &str,
    ) -> Option<Type> {
        let body: Vec<&crate::parser::Statement> = if !self.full_function_body_snapshot.is_empty() {
            self.full_function_body_snapshot.iter().copied().collect()
        } else {
            self.current_function_body.iter().copied().collect()
        };
        let mut peer = None;
        self.scan_stmts_for_mut_int_assign_peer(&body, name, &mut peer);
        peer
    }

    fn scan_stmts_for_mut_int_assign_peer(
        &self,
        stmts: &[&crate::parser::Statement<'ast>],
        name: &str,
        peer: &mut Option<Type>,
    ) {
        use crate::parser::Statement;
        for stmt in stmts {
            match stmt {
                Statement::Assignment { target, value, .. } => {
                    if matches!(
                        target,
                        Expression::Identifier { name: n, .. } if n == name
                    ) {
                        // Resolve RHS width against the *full* function body so
                        // `best_count = count` inside `if` still finds
                        // `let count = counts[i]` in the enclosing while (WDB-305).
                        let full: Vec<&crate::parser::Statement> =
                            if !self.full_function_body_snapshot.is_empty() {
                                self.full_function_body_snapshot.iter().copied().collect()
                            } else {
                                self.current_function_body.iter().copied().collect()
                            };
                        if let Some(ty) = self.int_width_type_from_assign_rhs(value, &full) {
                            *peer = Some(ty);
                        }
                    }
                }
                Statement::While { body, .. } | Statement::For { body, .. } => {
                    self.scan_stmts_for_mut_int_assign_peer(body, name, peer);
                }
                Statement::If {
                    then_block,
                    else_block,
                    ..
                } => {
                    self.scan_stmts_for_mut_int_assign_peer(then_block, name, peer);
                    if let Some(eb) = else_block {
                        self.scan_stmts_for_mut_int_assign_peer(eb, name, peer);
                    }
                }
                Statement::Match { arms, .. } => {
                    for arm in arms {
                        if let Expression::Block { statements, .. } = arm.body {
                            self.scan_stmts_for_mut_int_assign_peer(statements, name, peer);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn int_width_type_from_assign_rhs(
        &self,
        value: &Expression<'ast>,
        body: &[&crate::parser::Statement<'ast>],
    ) -> Option<Type> {
        if let Some(ty) = self.concrete_u32_or_i32_width(value) {
            return Some(ty);
        }
        // P3.538: `prev = m.version` where `version: int`/`i64` must peer WJ int so
        // `let mut prev = -1` emits `_i64`, not coordinate `_i32`.
        if let Some(ty) = self
            .infer_expression_type(value)
            .as_ref()
            .and_then(Self::parser_type_as_wj_int_peer)
        {
            return Some(ty);
        }
        if let Expression::Index { object, .. } = value {
            if let Some(ty) = self.vec_index_elem_u32_or_i32(object) {
                return Some(ty);
            }
        }
        // `best_count = count` where `let count = counts[i]` with `counts: Vec<u32>`.
        if let Expression::Identifier { name, .. } = value {
            if let Some(rhs) = Self::find_let_rhs_in_stmts(body, name) {
                return self.int_width_type_from_assign_rhs(rhs, body);
            }
            if let Some(ty) = self.local_var_types.get(name.as_str()) {
                return Self::parser_type_as_u32_or_i32_peer(ty)
                    .or_else(|| Self::parser_type_as_wj_int_peer(ty));
            }
            for p in &self.current_function_params {
                if p.name == *name {
                    return Self::parser_type_as_u32_or_i32_peer(&p.type_)
                        .or_else(|| Self::parser_type_as_wj_int_peer(&p.type_));
                }
            }
        }
        None
    }

    fn vec_index_elem_u32_or_i32(&self, object: &Expression<'ast>) -> Option<Type> {
        let Expression::Identifier { name, .. } = object else {
            return None;
        };
        let ty = self
            .local_var_types
            .get(name.as_str())
            .cloned()
            .or_else(|| {
                self.current_function_params
                    .iter()
                    .find(|p| p.name == *name)
                    .map(|p| p.type_.clone())
            })?;
        let elem = match &ty {
            Type::Vec(inner) => inner.as_ref(),
            Type::Reference(inner) | Type::MutableReference(inner) => match inner.as_ref() {
                Type::Vec(elem) => elem.as_ref(),
                _ => return None,
            },
            _ => return None,
        };
        Self::parser_type_as_u32_or_i32_peer(elem)
    }

    fn concrete_u32_or_i32_width(&self, value: &Expression<'ast>) -> Option<Type> {
        self.infer_expression_type(value)
            .as_ref()
            .and_then(Self::parser_type_as_u32_or_i32_peer)
    }

    fn parser_type_as_u32_or_i32_peer(ty: &Type) -> Option<Type> {
        match ty {
            Type::Uint => Some(Type::Uint),
            Type::Int32 => Some(Type::Int32),
            Type::Custom(n) if n == "u32" => Some(Type::Uint),
            Type::Custom(n) if n == "i32" => Some(Type::Int32),
            _ => None,
        }
    }

    /// WJ `int` / Rust `i64` peer for mut-local later-assign width (P3.538).
    fn parser_type_as_wj_int_peer(ty: &Type) -> Option<Type> {
        match ty {
            Type::Int => Some(Type::Int),
            Type::Custom(n) if n == "int" || n == "i64" => Some(Type::Int),
            _ => None,
        }
    }

    fn find_let_rhs_in_stmts<'b>(
        stmts: &[&'b crate::parser::Statement<'ast>],
        name: &str,
    ) -> Option<&'b Expression<'ast>> {
        use crate::parser::{Pattern, Statement};
        for stmt in stmts {
            match stmt {
                Statement::Let { pattern, value, .. } => {
                    if matches!(
                        pattern,
                        Pattern::Identifier(n) | Pattern::MutBinding(n) if n == name
                    ) {
                        return Some(value);
                    }
                }
                Statement::While { body, .. } | Statement::For { body, .. } => {
                    if let Some(v) = Self::find_let_rhs_in_stmts(body, name) {
                        return Some(v);
                    }
                }
                Statement::If {
                    then_block,
                    else_block,
                    ..
                } => {
                    if let Some(v) = Self::find_let_rhs_in_stmts(then_block, name) {
                        return Some(v);
                    }
                    if let Some(eb) = else_block {
                        if let Some(v) = Self::find_let_rhs_in_stmts(eb, name) {
                            return Some(v);
                        }
                    }
                }
                Statement::Match { arms, .. } => {
                    for arm in arms {
                        if let Expression::Block { statements, .. } = arm.body {
                            if let Some(v) = Self::find_let_rhs_in_stmts(statements, name) {
                                return Some(v);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        None
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

    /// WDB-440: `let mse = if count > 0 { sum / count } else { 0.0 }` must type as
    /// `f32` so struct-lit reuse does not emit `mse.clone()` (Copy auto-copy).
    pub(in crate::codegen::rust) fn if_else_binding_should_be_float(
        &self,
        then_branch: &Expression<'ast>,
        else_branch: &Expression<'ast>,
    ) -> bool {
        self.if_branch_is_float(then_branch) && self.if_branch_is_float(else_branch)
    }

    fn if_branch_is_float(&self, expr: &Expression<'ast>) -> bool {
        match expr {
            Expression::Literal {
                value: Literal::Float(_),
                ..
            } => true,
            Expression::Identifier { name, .. } => {
                self.local_var_types.get(name).is_some_and(|t| {
                    matches!(t, Type::Float)
                        || matches!(t, Type::Custom(n) if n == "f32" || n == "f64")
                }) || self.current_function_params.iter().any(|p| {
                    p.name == *name
                        && (matches!(p.type_, Type::Float)
                            || matches!(&p.type_, Type::Custom(n) if n == "f32" || n == "f64"))
                })
            }
            Expression::Cast { type_, .. } => {
                matches!(type_, Type::Float)
                    || matches!(type_, Type::Custom(n) if n == "f32" || n == "f64")
            }
            Expression::Binary {
                left, right, op, ..
            } if matches!(
                op,
                crate::parser::BinaryOp::Add
                    | crate::parser::BinaryOp::Sub
                    | crate::parser::BinaryOp::Mul
                    | crate::parser::BinaryOp::Div
                    | crate::parser::BinaryOp::Mod
            ) =>
            {
                self.if_branch_is_float(left) || self.if_branch_is_float(right)
            }
            _ => self.infer_expression_type(expr).as_ref().is_some_and(|t| {
                matches!(t, Type::Float) || matches!(t, Type::Custom(n) if n == "f32" || n == "f64")
            }),
        }
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
        if self
            .local_var_types
            .get(name)
            .is_some_and(|t| matches!(t, Type::Int32) || matches!(t, Type::Custom(n) if n == "i32"))
        {
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
                    || self.infer_expression_type(expr).as_ref().is_some_and(|t| {
                        matches!(t, Type::Uint) || matches!(t, Type::Custom(n) if n == "u32")
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
                        matches!(t, Type::Int32) || matches!(t, Type::Custom(n) if n == "i32")
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

    /// Concrete WJ `int`/`i64` formal or binding in `expr` (not bare literals).
    /// Beats `function_prefers_i32_coord_locals` for bitop lets under `-> Vec<u8>`
    /// (`let clock_hi = clock_seq >> 8 & …`, P3.590 / wj-uuid).
    pub(in crate::codegen::rust) fn expression_has_concrete_wj_int_i64_peer(
        &self,
        expr: &Expression<'ast>,
    ) -> bool {
        match expr {
            Expression::Identifier { name, .. } => {
                self.explicit_wj_int_annotated_locals.contains(name)
                    || self.local_var_types.get(name.as_str()).is_some_and(|t| {
                        matches!(t, Type::Int)
                            || matches!(t, Type::Custom(n) if n == "int" || n == "i64")
                    })
                    || self.current_function_params.iter().any(|p| {
                        p.name == name.as_str()
                            && (matches!(p.type_, Type::Int)
                                || matches!(&p.type_, Type::Custom(n) if n == "int" || n == "i64"))
                    })
            }
            Expression::Binary { left, right, .. } => {
                self.expression_has_concrete_wj_int_i64_peer(left)
                    || self.expression_has_concrete_wj_int_i64_peer(right)
            }
            Expression::Unary { operand, .. } => {
                self.expression_has_concrete_wj_int_i64_peer(operand)
            }
            Expression::Cast { expr, type_, .. } => {
                matches!(type_, Type::Int)
                    || matches!(type_, Type::Custom(n) if n == "int" || n == "i64")
                    || self.expression_has_concrete_wj_int_i64_peer(expr)
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
        let Expression::Binary {
            left, right, op, ..
        } = condition
        else {
            return;
        };
        if !matches!(
            op,
            BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge
        ) {
            return;
        }
        let ident_name = |expr: &Expression<'ast>, lit: &Expression<'ast>| -> Option<String> {
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
            if self.let_binding_has_negative_int_literal_init(name.as_str()) {
                return;
            }
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
                // P3.497: `now` from `timestamp_millis` / i64 lets is not an i32 coord counter.
                // P3.549: untyped `let mut i = 0` (literal-init loop counter) stays WJ `Int` in
                // `local_var_types` but must still promote when the while peer is i32-width
                // (`while i < len - 1` after `let len = …len() as i32`) — otherwise emit
                // keeps `0_i64` and casts the i32 bound (`(len - 1) as i64`).
                let i64_init = !self.literal_init_wj_int_loop_counters.contains(name)
                    && (matches!(self.local_var_types.get(name.as_str()), Some(Type::Int))
                        || matches!(
                            self.local_var_types.get(name.as_str()),
                            Some(Type::Custom(n)) if n == "int" || n == "i64"
                        )
                        || self.int_type_for_mixed_int_codegen(left) == IntType::I64);
                if is_counter && !i64_init {
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
                        matches!(t, Type::Uint) || matches!(t, Type::Custom(n) if n == "u32")
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
            || self
                .infer_expression_type(i64_side)
                .as_ref()
                .is_some_and(|t| {
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
                    matches!(t, Type::Uint) || matches!(t, Type::Custom(n) if n == "u32")
                })
    }

    fn expression_promotes_to_u32_in_compare(&self, expr: &Expression<'ast>) -> bool {
        self.int_type_for_mixed_int_codegen(expr) == IntType::U32
            || self.infer_expression_type(expr).as_ref().is_some_and(|t| {
                matches!(t, Type::Uint) || matches!(t, Type::Custom(n) if n == "u32")
            })
    }

    /// Operand type for driving int literal suffixes in binary ops (`idx + 1` → `1_usize`).
    pub(in crate::codegen::rust) fn peer_type_for_int_literal_operand(
        &self,
        expr: &Expression<'ast>,
    ) -> Option<Type> {
        // WDB-315: usize + lit chains (incl. nested) peer usize before coord i32.
        if self.expression_produces_usize(expr) {
            return Some(Type::Custom("usize".into()));
        }
        // Formal `int`/`i64` beats coord-builder i32 (P3.705: `n == 1` beside Vec::push).
        if let Expression::Identifier { name, .. } = expr {
            if self.current_function_params.iter().any(|p| {
                p.name == *name
                    && (matches!(&p.type_, Type::Int)
                        || matches!(
                            &p.type_,
                            Type::Custom(n) if matches!(n.as_str(), "int" | "i64")
                        ))
            }) {
                return Some(Type::Int);
            }
        }
        if self.infer_expression_type(expr).is_some_and(|t| {
            matches!(t, Type::Int) || matches!(t, Type::Custom(n) if n == "int" || n == "i64")
        }) {
            return Some(Type::Int);
        }
        if self.int_type_for_mixed_int_codegen(expr) == crate::type_inference::IntType::I64 {
            return Some(Type::Int);
        }
        if self.wj_int_coord_builder_operand(expr) {
            return Some(Type::Int32);
        }
        if let Expression::Identifier { name, .. } = expr {
            // P3.679: i32 / WJ-int loop counters beat stale `usize_variables` from
            // substring/index formals — else `while i < 64` emits `64_usize` vs `i: i32`.
            if self.codegen_i32_binding_names.contains(name) {
                return Some(Type::Int32);
            }
            if self.local_var_types.get(name.as_str()).is_some_and(|t| {
                matches!(t, Type::Int32) || matches!(t, Type::Custom(n) if n == "i32")
            }) {
                return Some(Type::Int32);
            }
            if self.literal_init_wj_int_loop_counters.contains(name)
                && self.local_var_types.get(name.as_str()).is_some_and(|t| {
                    matches!(t, Type::Int)
                        || matches!(t, Type::Custom(n) if n == "int" || n == "i64")
                })
            {
                return Some(Type::Int);
            }
            // usize index/len counters beat return-inferred Int32 (P3.311/P3.314).
            if self.usize_variables.contains(name) {
                return Some(Type::Custom("usize".into()));
            }
            if let Some(w) = self.local_int_rust_type_name_excluding_ambiguous_int(name) {
                return Self::parser_type_from_rust_int_name(w);
            }
        }
        if let Expression::Binary {
            left, right, op, ..
        } = expr
        {
            use crate::parser::BinaryOp;
            if matches!(
                op,
                BinaryOp::Add
                    | BinaryOp::Sub
                    | BinaryOp::Mul
                    | BinaryOp::Div
                    | BinaryOp::Mod
                    | BinaryOp::BitAnd
                    | BinaryOp::BitOr
                    | BinaryOp::BitXor
                    | BinaryOp::Shl
                    | BinaryOp::Shr
            ) {
                // WDB-315: usize + lit chains peer usize before coord i32.
                if self.expression_produces_usize(expr) {
                    return Some(Type::Custom("usize".into()));
                }
                return self
                    .peer_type_for_int_literal_operand(left)
                    .or_else(|| self.peer_type_for_int_literal_operand(right));
            }
        }
        if self.infer_expression_type_is_usize(expr) {
            return Some(Type::Custom("usize".into()));
        }
        if let Expression::MethodCall { object, method, .. } = expr {
            if matches!(
                method.as_str(),
                "wrapping_mul"
                    | "wrapping_add"
                    | "wrapping_sub"
                    | "wrapping_div"
                    | "wrapping_shl"
                    | "wrapping_shr"
                    | "rotate_left"
                    | "rotate_right"
            ) {
                return self.peer_type_for_int_literal_operand(object);
            }
        }
        if self
            .infer_expression_type(expr)
            .is_some_and(|t| matches!(t, Type::Uint) || matches!(t, Type::Custom(n) if n == "u32"))
        {
            return Some(Type::Uint);
        }
        // P3.322: ambiguous WJ `int` locals may emit as i32 while `local_var_types` stays `Int`.
        if self.int_type_for_mixed_int_codegen(expr) == crate::type_inference::IntType::U32 {
            return Some(Type::Uint);
        }
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
        use crate::type_inference::{promote_types, IntType};
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
                // Declared `int`/`i64` formals stay i64. Coord-builder i32 and stale
                // usize marks must not narrow `n == 1` (P3.705).
                if self.current_function_params.iter().any(|p| {
                    p.name == *name
                        && (matches!(&p.type_, Type::Int)
                            || matches!(
                                &p.type_,
                                Type::Custom(n) if matches!(n.as_str(), "int" | "i64")
                            ))
                }) {
                    return crate::type_inference::IntType::I64;
                }
                if self.usize_variables.contains(name) {
                    return crate::type_inference::IntType::Usize;
                }
                if let Some(t) = self.local_var_types.get(name.as_str()) {
                    if let Some(a) = Self::parser_type_to_promotion_int_type(t) {
                        if a == IntType::I64 {
                            if let Some(w) =
                                self.local_int_rust_type_name_excluding_ambiguous_int(name)
                            {
                                if w == "i32" {
                                    return IntType::I32;
                                }
                            }
                            if eng == IntType::I32 {
                                // P3.371: WJ `int` locals stay i64; inference may flip i32 after compares.
                                let wj_int_local =
                                    self.local_var_types.get(name.as_str()).is_some_and(|t| {
                                        matches!(t, Type::Int)
                                            || matches!(
                                                t,
                                                Type::Custom(n) if n == "int" || n == "i64"
                                            )
                                    });
                                if wj_int_local && !self.codegen_i32_binding_names.contains(name) {
                                    return IntType::I64;
                                }
                                return IntType::I32;
                            }
                        }
                        return a;
                    }
                }
                if let Some(p) = self
                    .current_function_params
                    .iter()
                    .find(|p| p.name == *name)
                {
                    if let Some(a) = Self::parser_type_to_promotion_int_type(&p.type_) {
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
            Expression::MethodCall { object, method, .. } => {
                if matches!(
                    method.as_str(),
                    "wrapping_mul"
                        | "wrapping_add"
                        | "wrapping_sub"
                        | "wrapping_div"
                        | "wrapping_shl"
                        | "wrapping_shr"
                        | "rotate_left"
                        | "rotate_right"
                ) {
                    let r = self.int_type_for_mixed_int_codegen(object);
                    if r != IntType::Unknown {
                        return r;
                    }
                }
                if self.expression_produces_usize(expr) {
                    return IntType::Usize;
                }
                eng
            }
            Expression::Binary {
                left, right, op, ..
            } => {
                use crate::parser::BinaryOp;
                if matches!(
                    op,
                    BinaryOp::Add
                        | BinaryOp::Sub
                        | BinaryOp::Mul
                        | BinaryOp::Div
                        | BinaryOp::Mod
                        | BinaryOp::BitAnd
                        | BinaryOp::BitOr
                        | BinaryOp::BitXor
                        | BinaryOp::Shl
                        | BinaryOp::Shr
                ) {
                    let lt = self.int_type_for_mixed_int_codegen(left);
                    let rt = self.int_type_for_mixed_int_codegen(right);
                    let lit = |e: &Expression<'ast>| {
                        matches!(
                            e,
                            Expression::Literal {
                                value: Literal::Int(_),
                                ..
                            }
                        )
                    };
                    let unified = match (lt, rt) {
                        (IntType::Usize, IntType::Usize) => IntType::Usize,
                        (IntType::Usize, _) if lit(right) => IntType::Usize,
                        (_, IntType::Usize) if lit(left) => IntType::Usize,
                        (IntType::U32, IntType::U32) => IntType::U32,
                        (IntType::U32, IntType::I64) if lit(right) => IntType::U32,
                        (IntType::I64, IntType::U32) if lit(left) => IntType::U32,
                        (IntType::U32, IntType::I32) if lit(right) => IntType::U32,
                        (IntType::I32, IntType::U32) if lit(left) => IntType::U32,
                        (IntType::I32, IntType::I32) => IntType::I32,
                        (IntType::I32, IntType::I64) if lit(right) => IntType::I32,
                        (IntType::I64, IntType::I32) if lit(left) => IntType::I32,
                        (IntType::I64, IntType::I64)
                            if lit(right)
                                && self.arithmetic_prefers_i32_ambiguous_int_local(left) =>
                        {
                            IntType::I32
                        }
                        (IntType::I64, IntType::I64)
                            if lit(left)
                                && self.arithmetic_prefers_i32_ambiguous_int_local(right) =>
                        {
                            IntType::I32
                        }
                        (IntType::I64, IntType::I64)
                            if self.function_prefers_i32_coord_locals()
                                && lit(left)
                                && self.wj_int_coord_builder_operand(right) =>
                        {
                            IntType::I32
                        }
                        (IntType::I64, IntType::I64)
                            if self.function_prefers_i32_coord_locals()
                                && lit(right)
                                && self.wj_int_coord_builder_operand(left) =>
                        {
                            IntType::I32
                        }
                        (IntType::I64, IntType::I32)
                            if self.function_prefers_i32_coord_locals()
                                && self.wj_int_coord_builder_operand(left) =>
                        {
                            IntType::I32
                        }
                        (IntType::I32, IntType::I64)
                            if self.function_prefers_i32_coord_locals()
                                && self.wj_int_coord_builder_operand(right) =>
                        {
                            IntType::I32
                        }
                        _ => promote_types(lt, rt),
                    };
                    if unified != IntType::Unknown {
                        return unified;
                    }
                }
                if self.expression_produces_usize(expr) {
                    return IntType::Usize;
                }
                eng
            }
            _ => {
                if self.expression_produces_usize(expr) {
                    return IntType::Usize;
                }
                eng
            }
        }
    }
}
