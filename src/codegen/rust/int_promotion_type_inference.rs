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
            matches!(t, Type::Int)
                || matches!(t, Type::Custom(n) if n == "int" || n == "i64")
        }) {
            return true;
        }
        if let Expression::Identifier { name, .. } = i64_side {
            if self.local_var_types.get(name.as_str()).is_some_and(|t| {
                matches!(t, Type::Int)
                    || matches!(t, Type::Custom(n) if n == "int" || n == "i64")
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
            || self
                .infer_expression_type(expr)
                .as_ref()
                .is_some_and(|t| {
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
        // P3.329: Call/MethodCall WJ `int` results must not become usize via later
        // substring formals (`let plus_pos = find_tz_sign(...)`).
        if matches!(value, Expression::Call { .. } | Expression::MethodCall { .. }) {
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
        if emitted_rhs.ends_with("_usize")
            || emitted_rhs.contains("_usize")
            || self.int_type_for_mixed_int_codegen(value) == IntType::Usize
        {
            self.local_var_types
                .insert(name.to_string(), Type::Custom("usize".into()));
            self.usize_variables.insert(name.to_string());
            return;
        }
        // WDB-302: Custom-return builders register untyped `let mut total = 0` as Int32
        // while the literal still emits `_i64`. Emitted suffix wins so compound assign
        // does not append `as i32` onto `triangles as i64`, and `total / 3` peers i64
        // (not `3_u64` from an outer `as u64`).
        if emitted_rhs.ends_with("_i64") {
            self.local_var_types.insert(name.to_string(), Type::Int);
            self.codegen_i32_binding_names.remove(name);
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

    /// WDB-305: untyped `let mut best_count = 0` defaults to return-width i64 while later
    /// `best_count = count` assigns a `u32` (CDLP majority). Prefer the later assign width.
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
                return Self::parser_type_as_u32_or_i32_peer(ty);
            }
            for p in &self.current_function_params {
                if p.name == *name {
                    return Self::parser_type_as_u32_or_i32_peer(&p.type_);
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
        if self.infer_expression_type(expr).is_some_and(|t| {
            matches!(t, Type::Int)
                || matches!(t, Type::Custom(n) if n == "int" || n == "i64")
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
        if self.infer_expression_type(expr).is_some_and(|t| {
            matches!(t, Type::Uint) || matches!(t, Type::Custom(n) if n == "u32")
        }) {
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
                                // P3.371: WJ `int` locals stay i64; inference may flip i32 after compares.
                                let wj_int_local = self.local_var_types.get(name.as_str()).is_some_and(
                                    |t| {
                                        matches!(t, Type::Int)
                                            || matches!(
                                                t,
                                                Type::Custom(n) if n == "int" || n == "i64"
                                            )
                                    },
                                );
                                if wj_int_local && !self.codegen_i32_binding_names.contains(name) {
                                    return IntType::I64;
                                }
                                return IntType::I32;
                            }
                        }
                        return a;
                    }
                }
                if let Some(p) = self.current_function_params.iter().find(|p| p.name == *name) {
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
            Expression::Binary { left, right, op, .. } => {
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
