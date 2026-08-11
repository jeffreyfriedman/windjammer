//! Statement Generation Module
//!
//! Handles generation of Rust code for all statement types:
//! - let/const/static declarations
//! - Assignments (including compound assignments)
//! - if/while/for/match/loop
//! - return, break, continue
//! - Expression statements
//! - Thread/Async blocks
//! - Block generation with implicit return handling

use crate::parser::*;

use super::self_analysis;
use super::string_analysis;

/// Result of looking ahead to detect Option field take/replace patterns.
enum OptionFieldPattern {
    /// Next statement is `self.field = None` — fold into `.take()`
    ClearToNone(usize),
    /// Next statement is `self.field = Some(expr)` — fold into `.replace(expr)`
    ReplaceWith(usize, String),
    /// No recognizable pattern
    None,
}
use super::CodeGenerator;

#[allow(clippy::collapsible_match, clippy::collapsible_if)]
impl<'ast> CodeGenerator<'ast> {
    /// Whether `assignment_float_target_type` should be set for the whole assignment/compound RHS
    /// (float literals + mixed f32/f64 arithmetic toward an f32 or f64 slot).
    pub(in crate::codegen::rust) fn assignment_target_needs_float_codegen_context(
        ty: &Type,
    ) -> bool {
        match ty {
            Type::Reference(inner) | Type::MutableReference(inner) => {
                Self::assignment_target_needs_float_codegen_context(inner)
            }
            Type::Custom(name) if name == "f32" || name == "f64" => true,
            // Generic `float` / `Type::Float` maps to f64 via float_literal_suffix_from_assignment_lhs.
            Type::Float => true,
            Type::Vec(inner) | Type::Array(inner, _) => {
                Self::assignment_target_needs_float_codegen_context(inner)
            }
            _ => false,
        }
    }

    pub(in crate::codegen::rust) fn is_float_numeric_type(t: &Type) -> bool {
        match t {
            Type::Float => true,
            Type::Custom(n) => n == "f32" || n == "f64",
            _ => false,
        }
    }

    pub(in crate::codegen::rust) fn is_int_numeric_type(t: &Type) -> bool {
        match t {
            Type::Int | Type::Int32 | Type::Uint => true,
            Type::Custom(n) => matches!(
                n.as_str(),
                "i32" | "u32" | "i64" | "u64" | "usize" | "isize" | "i8" | "u8" | "i16" | "u16"
            ),
            _ => false,
        }
    }

    pub(in crate::codegen::rust) fn float_type_name(t: &Type) -> &'static str {
        match t {
            Type::Custom(n) if n == "f64" => "f64",
            Type::Float => "f64",
            _ => "f32",
        }
    }

    /// Determine the concrete Rust float type name for a compound assignment target.
    /// Priority: explicit type annotation → float inference engine → assignment context → inferred type.
    pub(in crate::codegen::rust) fn resolve_compound_assign_float_target(
        &self,
        target: &Expression,
    ) -> Option<&'static str> {
        // 1. Check local_var_types for explicit type annotation
        if let Expression::Identifier { name, .. } = target {
            if let Some(local_ty) = self.local_var_types.get(name) {
                if Self::is_int_numeric_type(local_ty) {
                    return None;
                }
                if matches!(local_ty, Type::Custom(n) if n == "f32") {
                    return Some("f32");
                }
                if matches!(local_ty, Type::Custom(n) if n == "f64") {
                    return Some("f64");
                }
            }
        }
        // 2. Use float inference engine (distinguishes f32 vs f64 precisely)
        {
            use crate::type_inference::FloatType;
            let ft = if let Some(ni) = &self.numeric_inference {
                ni.get_float_type(target)
            } else {
                FloatType::Unknown
            };
            match ft {
                FloatType::F32 => return Some("f32"),
                FloatType::F64 => return Some("f64"),
                FloatType::Unknown => {}
            }
        }
        // 3. Check the assignment_float_target_type context
        if let Some(ref aft) = self.assignment_float_target_type {
            if matches!(aft, Type::Custom(n) if n == "f32") {
                return Some("f32");
            }
            if Self::is_float_numeric_type(aft) {
                return Some(Self::float_type_name(aft));
            }
        }
        // 4. Infer from target expression type (fallback; may not distinguish f32/f64)
        let tgt_ty = self.infer_expression_type(target);
        if let Some(ref t) = tgt_ty {
            if Self::is_int_numeric_type(t) {
                return None;
            }
            if Self::is_float_numeric_type(t) {
                return Some(Self::float_type_name(t));
            }
        }
        None
    }

    /// Generate a statement with automatic source tracking
    #[allow(dead_code)]
    pub(super) fn generate_statement_tracked(&mut self, stmt: &Statement<'ast>) -> String {
        let code = self.generate_statement(stmt);
        self.track_generated_lines(&code);
        code
    }

    /// Whether an expression's value should be treated as owned `String` for if/else branch coercion.
    fn expr_suggests_owned_string_coercion(&self, expr: &Expression<'ast>) -> bool {
        if string_analysis::expression_produces_string(expr) {
            return true;
        }
        self.infer_expression_type(expr).as_ref().is_some_and(|t| {
            matches!(t, Type::String)
                || matches!(t, Type::Custom(n) if n == "String" || n == "string")
        })
    }

    /// Last value-producing expression in an if/else branch suggests owned `String` (e.g. `.clone()` on `String`).
    pub(in crate::codegen::rust) fn branch_tail_suggests_owned_string_coercion(
        &self,
        block: &[&'ast Statement<'ast>],
    ) -> bool {
        let Some(last) = block.last().copied() else {
            return false;
        };
        match last {
            Statement::Expression { expr, .. } => self.expr_suggests_owned_string_coercion(expr),
            _ => false,
        }
    }

    pub(in crate::codegen::rust) fn generate_statement_impl(
        &mut self,
        stmt: &Statement<'ast>,
    ) -> String {
        match stmt {
            Statement::Let {
                pattern,
                mutable,
                type_,
                value,
                location,
                ..
            } => self.generate_let_statement(pattern, *mutable, type_, value, location),
            Statement::Const {
                name, type_, value, ..
            } => self.generate_const_statement(name.as_str(), type_, value),
            Statement::Static {
                name,
                mutable,
                type_,
                value,
                ..
            } => self.generate_static_statement(name.as_str(), *mutable, type_, value),
            Statement::Return { value: expr, .. } => self.generate_return_statement(expr),
            Statement::Expression { expr, .. } => {
                let mut output = self.indent();
                let expr_str = self.generate_expression(expr);
                output.push_str(&expr_str);

                // TDD FIX: Only add semicolon if not in expression context
                // This prevents semicolons in if-else branches when used as values
                // e.g., `x = if cond { Some(42) } else { None }` (not `{ Some(42); }`)
                if !self.in_expression_context {
                    output.push_str(";\n");
                } else {
                    output.push('\n');
                }
                output
            }
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => self.generate_if_statement(condition, then_block, else_block),
            Statement::Match { value, arms, .. } => {
                if std::env::var("WJ_DEBUG_FIND_PATTERN").is_ok() {
                    if let Expression::MethodCall { method, .. } = value {
                        if method == "find" {
                        }
                    }
                }
                self.generate_match_statement(value, arms)
            }
            Statement::Loop { body, .. } => self.generate_loop_statement(body),
            Statement::While {
                condition, body, ..
            } => self.generate_while_statement(condition, body),
            Statement::For {
                pattern,
                iterable,
                body,
                location,
                ..
            } => self.generate_for_statement(pattern, iterable, body, location),
            Statement::Break { .. } => self.generate_break_statement(),
            Statement::Continue { .. } => self.generate_continue_statement(),
            Statement::Use { path, alias, .. } => {
                let mut output = self.indent();
                output.push_str("use ");
                output.push_str(&path.join("::"));
                if let Some(alias_name) = alias {
                    output.push_str(" as ");
                    output.push_str(alias_name);
                }
                output.push_str(";\n");
                output
            }
            Statement::Assignment {
                target,
                value,
                compound_op,
                ..
            } => self.generate_assignment_statement(target, value, compound_op),
            Statement::Thread { body, .. } => self.generate_thread_statement(body),
            Statement::Async { body, .. } => self.generate_async_statement(body),
            Statement::Defer { statement: _, .. } => {
                // Defer is not directly supported in Rust
                // We'll generate a comment for now
                let mut output = self.indent();
                output.push_str("// TODO: defer not yet implemented\n");
                output.push_str(&self.generate_statement(stmt));
                output
            }
        }
    }

    pub(in crate::codegen::rust) fn apply_vec_index_let_rhs_fixup(
        &mut self,
        var_name: Option<&str>,
        value: &Expression<'ast>,
        type_annotation: Option<&Type>,
        value_str: &mut String,
    ) {
        if !matches!(value, Expression::Index { .. }) {
            return;
        }
        let Some(name) = var_name else {
            return;
        };

        let elem_type = self
            .infer_expression_type(value)
            .or_else(|| type_annotation.cloned());

        if let Some(ref elem_type) = elem_type {
            if self.is_type_copy(elem_type) {
                return;
            }
        }
        // When elem_type is None (can't infer), still apply clone if the generated
        // code looks like a plain index access (no & prefix, no .clone() suffix).
        // This is safe because .clone() on a Copy type is a no-op, and for non-Copy
        // types it prevents E0507 "cannot move out of index".

        if self.variable_is_only_field_accessed(name) {
            let prev_borrow_ctx = self.in_borrow_context;
            self.in_borrow_context = true;
            *value_str = self.generate_expression(value);
            self.in_borrow_context = prev_borrow_ctx;
            *value_str = format!("&{}", *value_str);
            self.borrowed_iterator_vars.insert(name.to_string());
            return;
        }

        if value_str.starts_with("&mut ") {
            return;
        }
        if value_str.ends_with(".clone()") || value_str.ends_with(".to_string()") {
            return;
        }

        let is_string = matches!(elem_type, Some(Type::String))
            || matches!(elem_type, Some(Type::Custom(ref n)) if n == "string");

        if value_str.starts_with('&') {
            if is_string {
                *value_str = format!("({}).to_string()", *value_str);
            } else {
                let base = value_str
                    .strip_prefix('&')
                    .map(str::trim_start)
                    .unwrap_or(value_str.as_str());
                *value_str = format!("{}.clone()", base);
            }
        } else {
            *value_str = format!("{}.clone()", *value_str);
        }
    }

    /// `let mut x = y` when `y` is an `&T` binding (`if let` / `match` on `&vec[i]`, non-Copy `T`)
    /// and `T` is not `Copy` — produce an owned value (e.g. `clips.clone()`) for mutation.
    pub(in crate::codegen::rust) fn let_rhs_clone_if_mut_from_non_copy_ref(
        &self,
        mutable: bool,
        value: &Expression<'ast>,
        needs_mut_ref: bool,
        value_str: &str,
    ) -> String {
        if !mutable || needs_mut_ref || !matches!(value, Expression::Identifier { .. }) {
            return value_str.to_string();
        }
        if value_str.contains(".clone()") || value_str.ends_with(".to_string()") {
            return value_str.to_string();
        }
        if let Expression::Identifier { name, .. } = value {
            if self.inferred_borrowed_params.contains(name) {
                let is_copy = self
                    .current_function_params
                    .iter()
                    .find(|p| p.name == *name)
                    .map(|p| self.is_type_copy(&p.type_))
                    .unwrap_or(false);
                if !is_copy {
                    return format!("{}.clone()", value_str);
                }
            }
        }
        let Some(ty) = self.infer_expression_type(value) else {
            return value_str.to_string();
        };
        match ty {
            Type::Reference(inner) | Type::MutableReference(inner) => {
                if self.is_type_copy(inner.as_ref()) {
                    value_str.to_string()
                } else {
                    format!("{}.clone()", value_str)
                }
            }
            _ => value_str.to_string(),
        }
    }

    /// E0507 fix for `let x = self.field` behind borrowed self.
    ///
    /// Patterns recognized:
    /// - `let prev = self.field; self.field = None`  → `let prev = self.field.take()`
    /// - `let prev = self.field; self.field = Some(v)` → `let prev = self.field.replace(v)`
    /// - `let mut x = self.field; …; self.field = x` (writeback on `&mut self`) → bare move
    /// - Other non-Copy behind &self/&mut self → `.clone()`
    ///
    /// `let name = item.name` through an owned non-Copy param → `.clone()` (E0507).
    pub(in crate::codegen::rust) fn apply_owned_param_field_extract_clone(
        &self,
        value: &Expression<'ast>,
        value_str: &mut String,
    ) {
        if !matches!(value, Expression::FieldAccess { .. }) {
            return;
        }
        let Some(root) = self.root_identifier_of_field_or_index_chain(value) else {
            return;
        };
        if root == "self" {
            return;
        }
        let is_owned_param = self.current_function_params.iter().any(|p| {
            p.name == root
                && !self.inferred_borrowed_params.contains(&p.name)
                && !self.inferred_mut_borrowed_params.contains(&p.name)
                && !matches!(&p.type_, Type::Reference(_) | Type::MutableReference(_))
        });
        if !is_owned_param {
            return;
        }
        let Some(ty) = self.infer_expression_type(value) else {
            return;
        };
        if self.is_type_copy(&ty) || value_str.ends_with(".clone()") {
            return;
        }
        *value_str = format!("{}.clone()", value_str);
    }

    pub(in crate::codegen::rust) fn apply_self_field_move_fix(
        &mut self,
        value: &Expression<'ast>,
        value_str: &mut String,
        binding_name: Option<&str>,
    ) {
        if !matches!(value, Expression::FieldAccess { .. }) {
            return;
        }
        let Some(root) = self.root_identifier_of_field_or_index_chain(value) else {
            return;
        };
        let root_is_mut = self.inferred_mut_borrowed_params.contains(root);
        let root_is_shared = self.inferred_borrowed_params.contains(root);
        // WDB-090: mut-borrowed params (not only `self`) need take/clear / writeback rewriting.
        // Shared `&T` still clones non-Copy fields below via the final branch when root is self.
        let is_self_shared_or_mut = root == "self" && (root_is_shared || root_is_mut);
        if !root_is_mut && !is_self_shared_or_mut {
            return;
        }
        let Some(ty) = self.infer_expression_type(value) else {
            return;
        };
        if self.is_type_copy(&ty) || value_str.ends_with(".take()") {
            return;
        }
        let is_option =
            matches!(&ty, Type::Option(_)) || matches!(&ty, Type::Custom(n) if n == "Option");
        if is_option && root_is_mut {
            // Strip any .clone() that auto_clone inserted prematurely
            if value_str.ends_with(".clone()") {
                value_str.truncate(value_str.len() - ".clone()".len());
            }

            // Look ahead: does the next statement clear or replace this field?
            let field_path = self.extract_field_access_path_string(value);
            match self.next_statement_option_pattern(&field_path) {
                OptionFieldPattern::ClearToNone(next_idx) => {
                    *value_str = format!("{}.take()", value_str);
                    self.skip_block_indices.insert(next_idx);
                }
                OptionFieldPattern::ReplaceWith(next_idx, replacement) => {
                    let replacement_str = replacement.to_string();
                    *value_str = format!("{}.replace({})", value_str, replacement_str);
                    self.skip_block_indices.insert(next_idx);
                }
                OptionFieldPattern::None => {
                    *value_str = format!("{}.take()", value_str);
                }
            }
            return;
        }

        let field_path = self.extract_field_access_path_string(value);

        // WDB-090: `let v = csr.field; csr.field = Vec::new()` → `std::mem::take(&mut csr.field)`.
        if root_is_mut && self.type_supports_mem_take(&ty) {
            if let Some(next_idx) = self.next_statement_clears_field_to_default(&field_path) {
                if value_str.ends_with(".clone()") {
                    value_str.truncate(value_str.len() - ".clone()".len());
                }
                let base = value_str.trim_start_matches('&').trim();
                *value_str = format!("std::mem::take(&mut {base})");
                self.skip_block_indices.insert(next_idx);
                return;
            }
        }

        if root_is_mut
            && binding_name.is_some_and(|b| {
                self.block_writes_back_field_to_binding(&field_path, b)
            })
        {
            // Extract-assign writeback behind `&mut`: cannot bare-move (E0507).
            // Prefer `mem::take` when the field type derives/implements Default
            // (regression-042 `SimNetwork`); otherwise `.clone()` (engines without Default).
            if value_str.ends_with(".clone()") {
                value_str.truncate(value_str.len() - ".clone()".len());
            }
            if self.type_supports_mem_take(&ty) {
                let base = value_str.trim_start_matches('&').trim();
                *value_str = format!("std::mem::take(&mut {base})");
            } else if !value_str.ends_with(".clone()") {
                *value_str = format!("{}.clone()", value_str);
            }
        } else if !value_str.ends_with(".clone()") {
            *value_str = format!("{}.clone()", value_str);
        }
    }

    /// Next stmt is `field = Vec::new()` / empty Default constructor — fold into `mem::take`.
    fn next_statement_clears_field_to_default(&self, field_path: &str) -> Option<usize> {
        let next_idx = self.current_block_local_idx + 1;
        if next_idx >= self.current_function_body.len() {
            return None;
        }
        let next_stmt = self.current_function_body[next_idx];
        match next_stmt {
            Statement::Assignment {
                target,
                value,
                compound_op: None,
                ..
            } => {
                let target_path = self.extract_field_access_path_string(target);
                if target_path != field_path {
                    return None;
                }
                if self.expression_is_empty_default_constructor(value) {
                    Some(next_idx)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Empty stdlib collection / `String` constructors — Default-empty clears for writeback.
    /// Type-driven via [`is_stdlib_default_empty_type`]; not an ownership/coercion heuristic.
    fn expression_is_empty_default_constructor(&self, expr: &Expression<'_>) -> bool {
        if crate::codegen::rust::call_site_borrow::expression_is_vec_new_constructor(expr) {
            return true;
        }
        match expr {
            Expression::Call {
                function,
                arguments,
                ..
            } if arguments.is_empty() => {
                let type_name = match function {
                    Expression::FieldAccess { object, .. } => match object {
                        Expression::Identifier { name, .. } => Some(name.as_str()),
                        _ => None,
                    },
                    Expression::Identifier { name, .. } => name
                        .rsplit_once("::")
                        .map(|(ty, _)| ty),
                    _ => None,
                };
                type_name.is_some_and(|ty| {
                    crate::codegen::rust::stdlib_method_traits::is_stdlib_default_empty_type(ty)
                })
            }
            _ => false,
        }
    }

    /// Call-arg writeback behind `&mut self`: `let r = f(self.field); self.field = r.sub`.
    /// Prefer `std::mem::take`; fall back to `.clone()` when the type lacks Default.
    /// Returns `None` when this is not a writeback site (or owned `self` can bare-move).
    pub(in crate::codegen::rust) fn try_self_field_writeback_owned_arg(
        &self,
        arg_expr: &Expression<'ast>,
        arg_str: &str,
    ) -> Option<String> {
        if !matches!(arg_expr, Expression::FieldAccess { .. }) {
            return None;
        }
        if arg_str.contains("std::mem::take(") || arg_str.ends_with(".take()") {
            return None;
        }
        let root = self.root_identifier_of_field_or_index_chain(arg_expr)?;
        if root != "self" {
            return None;
        }
        let field_path = self.extract_field_access_path_string(arg_expr);
        if field_path.is_empty() {
            return None;
        }
        let body: Vec<&Statement<'_>> = self.current_function_body.iter().copied().collect();
        if !crate::auto_clone::AutoCloneAnalysis::self_field_has_writeback(
            &body,
            &field_path,
            self.current_statement_idx,
        ) {
            return None;
        }
        // Owned `self`: bare move is valid — leave the arg as-is.
        if !self.inferred_mut_borrowed_params.contains("self")
            && !self.inferred_borrowed_params.contains("self")
        {
            return None;
        }
        let Some(ty) = self.infer_expression_type(arg_expr) else {
            return None;
        };
        if self.is_type_copy(&ty) {
            return None;
        }
        let mut base = arg_str.to_string();
        if base.ends_with(".clone()") {
            base.truncate(base.len() - ".clone()".len());
        }
        let base = base.trim_start_matches('&').trim().to_string();
        if self.inferred_mut_borrowed_params.contains("self") && self.type_supports_mem_take(&ty)
        {
            Some(format!("std::mem::take(&mut {base})"))
        } else {
            Some(format!("{base}.clone()"))
        }
    }

    /// True when `ty` is safe for `std::mem::take` (Default in std or auto-derived).
    pub(in crate::codegen::rust) fn type_supports_mem_take(&self, ty: &Type) -> bool {
        match ty {
            Type::Int
            | Type::Int32
            | Type::Uint
            | Type::Float
            | Type::Bool
            | Type::String => true,
            Type::Vec(_) | Type::Option(_) => true,
            Type::Tuple(elems) => elems.iter().all(|t| self.type_supports_mem_take(t)),
            Type::Parameterized(base, args)
                if matches!(base.as_str(), "Vec" | "Option" | "Box" | "HashMap" | "HashSet") =>
            {
                args.iter().all(|a| self.type_supports_mem_take(a))
            }
            Type::Custom(name)
                if crate::type_classification::is_copy_primitive(name) || name == "String" =>
            {
                true
            }
            Type::Custom(name) => {
                let Some(fields) = self.lookup_struct_field_types(name) else {
                    return false;
                };
                !fields.is_empty() && fields.values().all(|ft| self.type_supports_mem_take(ft))
            }
            Type::Reference(inner) | Type::MutableReference(inner) => {
                self.type_supports_mem_take(inner)
            }
            _ => false,
        }
    }

    /// True when a later assignment in the current function body writes `binding` back to `field_path`.
    fn block_writes_back_field_to_binding(&self, field_path: &str, binding: &str) -> bool {
        let start = self.current_block_local_idx.saturating_add(1);
        for stmt in self.current_function_body.iter().skip(start) {
            if self.statement_writes_binding_to_field(stmt, field_path, binding) {
                return true;
            }
        }
        false
    }

    fn statement_writes_binding_to_field(
        &self,
        stmt: &Statement<'ast>,
        field_path: &str,
        binding: &str,
    ) -> bool {
        match stmt {
            Statement::Assignment {
                target,
                value,
                compound_op: None,
                ..
            } => {
                let target_path = self.extract_field_access_path_string(target);
                if target_path != field_path {
                    return false;
                }
                // `self.field = binding` or `self.field = binding.subfield` (call writeback).
                let value_path = self.extract_field_access_path_string(value);
                value_path == binding
                    || value_path.starts_with(&format!("{binding}."))
            }
            Statement::If {
                then_block,
                else_block,
                ..
            } => {
                then_block
                    .iter()
                    .any(|s| self.statement_writes_binding_to_field(s, field_path, binding))
                    || else_block.as_ref().is_some_and(|eb| {
                        eb.iter()
                            .any(|s| self.statement_writes_binding_to_field(s, field_path, binding))
                    })
            }
            Statement::While { body, .. }
            | Statement::For { body, .. }
            | Statement::Loop { body, .. } => body
                .iter()
                .any(|s| self.statement_writes_binding_to_field(s, field_path, binding)),
            // Match arms are expressions; writeback typically lives in loop/if bodies.
            _ => false,
        }
    }

    /// Extract the string representation of a field access path (e.g., "self.weapon").
    fn extract_field_access_path_string(&self, expr: &Expression<'ast>) -> String {
        match expr {
            Expression::FieldAccess { object, field, .. } => {
                let obj = self.extract_field_access_path_string(object);
                format!("{}.{}", obj, field)
            }
            Expression::Identifier { name, .. } => name.to_string(),
            _ => String::new(),
        }
    }

    /// Check if the next statement in the current block assigns None or Some(...)
    /// to the same field that was just read.
    fn next_statement_option_pattern(&self, field_path: &str) -> OptionFieldPattern {
        let next_idx = self.current_block_local_idx + 1;
        if next_idx >= self.current_function_body.len() {
            return OptionFieldPattern::None;
        }
        let next_stmt = self.current_function_body[next_idx];
        match next_stmt {
            Statement::Assignment {
                target,
                value,
                compound_op: None,
                ..
            } => {
                let target_path = self.extract_field_access_path_string(target);
                if target_path != field_path {
                    return OptionFieldPattern::None;
                }
                if self.expression_is_none(value) {
                    OptionFieldPattern::ClearToNone(next_idx)
                } else if let Some(inner) = self.expression_unwrap_some(value) {
                    OptionFieldPattern::ReplaceWith(next_idx, inner.to_string())
                } else {
                    OptionFieldPattern::None
                }
            }
            _ => OptionFieldPattern::None,
        }
    }

    fn expression_is_none(&self, expr: &Expression) -> bool {
        matches!(
            expr,
            Expression::Identifier { name, .. } if name == "None"
        ) || matches!(
            expr,
            Expression::Call { function, arguments, .. }
            if matches!(&**function, Expression::Identifier { name, .. } if name == "None")
               && arguments.is_empty()
        )
    }

    fn expression_unwrap_some(&self, expr: &Expression<'ast>) -> Option<String> {
        if let Expression::Call {
            function,
            arguments,
            ..
        } = expr
        {
            if let Expression::Identifier { name, .. } = &**function {
                if name == "Some" && arguments.len() == 1 {
                    let inner_str = self.generate_expression_immut(arguments[0].1);
                    return Some(inner_str);
                }
            }
        }
        Option::None
    }

    /// Tuple `let (a, b) = rhs`: register each binding's type for comparisons / codegen.
    /// When `rhs` is `vec[i]` and the element is non-Copy, Index codegen emits `&vec[i]` and Rust
    /// gives `&T` per field — mirror that as `Type::Reference` so `balance_eq_operands_for_rust`
    /// fixes `&String == String` (E0277).
    pub(in crate::codegen::rust) fn register_tuple_let_binding_types(
        &mut self,
        pattern: &Pattern<'ast>,
        value: &Expression<'ast>,
    ) {
        let Pattern::Tuple(patterns) = pattern else {
            return;
        };
        let Some(tuple_ty) = self.infer_expression_type(value) else {
            return;
        };
        let Type::Tuple(ref elem_tys) = tuple_ty else {
            return;
        };
        if patterns.len() != elem_tys.len() {
            return;
        }
        let yields_refs = self.tuple_let_rhs_yields_ref_bindings(value, &tuple_ty);
        for (pat, elem_ty) in patterns.iter().zip(elem_tys.iter()) {
            if let Pattern::Identifier(name) = pat {
                let ty = if yields_refs {
                    Type::Reference(Box::new(elem_ty.clone()))
                } else {
                    elem_ty.clone()
                };
                self.local_var_types.insert(name.clone(), ty);
            }
        }
    }

    fn tuple_let_rhs_yields_ref_bindings(
        &self,
        value: &Expression<'ast>,
        element_type: &Type,
    ) -> bool {
        matches!(value, Expression::Index { .. }) && !self.is_type_copy(element_type)
    }

    #[allow(dead_code)] // Used by upcoming borrow-prefix work
    fn identifier_is_borrowed_or_self(&self, name: &str) -> bool {
        if self.inferred_borrowed_params.contains(name)
            || self.inferred_mut_borrowed_params.contains(name)
        {
            return true;
        }
        if name == "self" && self.in_impl_block {
            return self.current_function_params.iter().any(|p| {
                p.name == "self"
                    && (matches!(&p.type_, crate::parser::Type::Reference(_))
                        || matches!(&p.type_, crate::parser::Type::MutableReference(_)))
            }) || self.inferred_borrowed_params.contains("self")
                || self.inferred_mut_borrowed_params.contains("self");
        }
        false
    }

    pub(in crate::codegen::rust) fn type_contains_reference(ty: &Type) -> bool {
        match ty {
            Type::Reference(_) | Type::MutableReference(_) => true,
            Type::Option(inner) | Type::Vec(inner) => Self::type_contains_reference(inner),
            Type::Result(ok, err) => {
                Self::type_contains_reference(ok) || Self::type_contains_reference(err)
            }
            Type::Tuple(elems) => elems.iter().any(Self::type_contains_reference),
            _ => false,
        }
    }

    /// Leftmost identifier in a chain of field accesses / indexing, e.g. `node.children` → `node`.
    pub(crate) fn root_identifier_of_field_or_index_chain<'e>(
        &self,
        expr: &'e Expression<'ast>,
    ) -> Option<&'e str> {
        match expr {
            Expression::Identifier { name, .. } => Some(name.as_str()),
            Expression::FieldAccess { object, .. } | Expression::Index { object, .. } => {
                self.root_identifier_of_field_or_index_chain(object)
            }
            _ => None,
        }
    }

    /// True when the scrutinee root (`self`, a param, …) is emitted as `&mut` in Rust.
    pub(in crate::codegen::rust) fn option_scrutinee_root_allows_mut_borrow(
        &self,
        value: &Expression<'ast>,
    ) -> bool {
        let Some(root) = self.root_identifier_of_field_or_index_chain(value) else {
            return false;
        };
        self.inferred_mut_borrowed_params.contains(root)
    }

    /// `&` / `&mut` prefix for matching on `Option` when the scrutinee lives behind a borrow.
    pub(in crate::codegen::rust) fn option_scrutinee_ref_prefix(
        &self,
        value: &Expression<'ast>,
    ) -> &'static str {
        let Some(root) = self.root_identifier_of_field_or_index_chain(value) else {
            return "";
        };
        if self.inferred_mut_borrowed_params.contains(root) {
            "&mut "
        } else if self.inferred_borrowed_params.contains(root) {
            "&"
        } else if root == "self" {
            let self_is_mut_borrowed = self.current_function_params.iter().any(|p| {
                p.name == "self" && matches!(p.ownership, crate::parser::OwnershipHint::Mut)
            });
            if self_is_mut_borrowed {
                return "&mut ";
            }
            let self_is_borrowed = self.current_function_params.iter().any(|p| {
                p.name == "self" && matches!(p.ownership, crate::parser::OwnershipHint::Ref)
            });
            if self_is_borrowed {
                "&"
            } else {
                ""
            }
        } else {
            ""
        }
    }

    /// Strip `&` / `&mut` so `.clone()` produces an owned value, not `&(expr.clone())`.
    pub(in crate::codegen::rust) fn strip_leading_borrow_prefix(&self, expr_str: &str) -> String {
        if let Some(stripped) = expr_str.strip_prefix("&mut ") {
            stripped.to_string()
        } else if let Some(stripped) = expr_str.strip_prefix("& ") {
            stripped.to_string()
        } else if expr_str.starts_with('&') && !expr_str.starts_with("&&") {
            expr_str[1..].trim_start().to_string()
        } else {
            expr_str.to_string()
        }
    }

    /// When `&self` + `if let Some(x) = self.opt` but the arm calls mutating methods on `x`, use `&mut`.
    /// Also upgrades indexed Option params (`slots[i]`) when the Some-binding is mutated.
    pub(in crate::codegen::rust) fn effective_option_scrutinee_ref_prefix(
        &self,
        value: &Expression<'ast>,
        some_arm: Option<&MatchArm<'ast>>,
    ) -> &'static str {
        if let Some(arm) = some_arm {
            if self.option_match_needs_mut_scrutinee_for_some_arm(arm, value)
                && self.option_scrutinee_root_allows_mut_borrow(value)
            {
                return "&mut ";
            }
        }
        let base = self.option_scrutinee_ref_prefix(value);
        // Indexed Option mutation (`slots[i]` + field write): prefer `Some(ref mut x) = slots[i]`
        // on an `&mut Vec` formal — do not emit a redundant `&mut slots[i]` prefix (match
        // ergonomics + IndexMut already yield a mutable place).
        if self.option_indexed_scrutinee_needs_mut_opt(some_arm, value) {
            return "";
        }
        if base == "&" {
            // When the Option's inner type is Copy and the arm body doesn't mutate
            // the binding, no `&` prefix is needed — Option<Copy> auto-copies.
            if let Some(Type::Option(inner)) = self.infer_expression_type(value) {
                if self.is_type_copy(&inner) {
                    if let Some(arm) = some_arm {
                        if self.option_arm_mutates_some_binding(arm, value)
                            && self.option_scrutinee_root_allows_mut_borrow(value)
                        {
                            return "&mut ";
                        }
                    }
                    return "";
                }
            }
        }
        if base == "&mut " && !self.option_scrutinee_root_allows_mut_borrow(value) {
            return "";
        }
        if base == "&mut " {
            if let Some(Type::Option(inner)) = self.infer_expression_type(value) {
                // For Copy inner types, strip &mut UNLESS the body mutates the binding
                // (field writes or mutating method calls).
                if self.is_type_copy(&inner) {
                    let body_mutates = some_arm
                        .is_some_and(|arm| self.option_arm_mutates_some_binding(arm, value));
                    if !body_mutates {
                        return "";
                    }
                }
                if !self.is_type_copy(&inner) {
                    let body_mutates = some_arm
                        .is_some_and(|arm| self.option_arm_mutates_some_binding(arm, value));
                    if !body_mutates && !self.match_scrutinee_is_self_field(value) {
                        return "";
                    }
                    // Non-Copy `Option<T>` on `&mut self.field`: keep `&mut` — never downgrade to `&`.
                }
            }
        }
        base
    }

    pub(in crate::codegen::rust) fn some_pattern_single_binding<'p>(
        pattern: &'p Pattern<'p>,
    ) -> Option<&'p str> {
        match pattern {
            Pattern::EnumVariant(v, EnumPatternBinding::Single(name)) => {
                let is_some = v == "Some" || v.ends_with("::Some");
                if is_some {
                    Some(name.as_str())
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// True when the Some-arm mutates its binding (field write or mutating method).
    fn option_arm_mutates_some_binding(
        &self,
        arm: &MatchArm<'ast>,
        scrutinee: &Expression<'ast>,
    ) -> bool {
        let Some(b) = Self::some_pattern_single_binding(&arm.pattern) else {
            return false;
        };
        if let Expression::Block { statements, .. } = arm.body {
            if statements
                .iter()
                .any(|s| self.statement_mutates_variable_field(s, b))
            {
                return true;
            }
            if statements
                .iter()
                .any(|s| self.statement_nonreadonly_method_call_on_var(s, b))
            {
                return true;
            }
        } else if self.expr_binding_receives_mutating_method_call(arm.body, b) {
            return true;
        }
        if let Some(Type::Option(inner)) = self.infer_expression_type(scrutinee) {
            return self.binding_receives_mutating_call_with_sig_check(
                arm.body,
                b,
                inner.as_ref(),
            );
        }
        false
    }

    /// Indexed Option (`slots[i]` / `self.slots[i]`) whose Some-binding is mutated.
    pub(in crate::codegen::rust) fn option_indexed_scrutinee_needs_mut_opt(
        &self,
        some_arm: Option<&MatchArm<'ast>>,
        scrutinee: &Expression<'ast>,
    ) -> bool {
        some_arm.is_some_and(|arm| self.option_indexed_scrutinee_needs_mut(arm, scrutinee))
    }

    /// Indexed Option (`slots[i]` / `self.slots[i]`) whose Some-binding is mutated.
    fn option_indexed_scrutinee_needs_mut(
        &self,
        arm: &MatchArm<'ast>,
        scrutinee: &Expression<'ast>,
    ) -> bool {
        if !Self::expr_has_index_access(scrutinee) {
            return false;
        }
        self.option_arm_mutates_some_binding(arm, scrutinee)
    }

    fn expr_has_index_access(expr: &Expression<'_>) -> bool {
        match expr {
            Expression::Index { .. } => true,
            Expression::FieldAccess { object, .. } => Self::expr_has_index_access(object),
            _ => false,
        }
    }

    fn option_match_needs_mut_scrutinee_for_some_arm(
        &self,
        main_arm: &MatchArm<'ast>,
        scrutinee: &Expression<'ast>,
    ) -> bool {
        if !self.match_scrutinee_is_self_field(scrutinee) {
            return false;
        }
        let Some(b) = Self::some_pattern_single_binding(&main_arm.pattern) else {
            return false;
        };
        if self.expr_binding_receives_mutating_method_call(main_arm.body, b) {
            return true;
        }
        if let Some(Type::Option(inner)) = self.infer_expression_type(scrutinee) {
            return self.binding_receives_mutating_call_with_sig_check(
                main_arm.body,
                b,
                inner.as_ref(),
            );
        }
        false
    }

    /// Check if expression is self.field (or self.field.subfield) - traces to self
    pub(in crate::codegen::rust) fn match_scrutinee_is_self_field(
        &self,
        expr: &Expression,
    ) -> bool {
        match expr {
            Expression::FieldAccess { object, .. } => {
                matches!(&**object, Expression::Identifier { name, .. } if name == "self")
                    || self.match_scrutinee_is_self_field(object)
            }
            Expression::Index { object, .. } => self.match_scrutinee_is_self_field(object),
            _ => false,
        }
    }

    pub(in crate::codegen::rust) fn match_scrutinee_is_self_method_call(
        &self,
        expr: &Expression,
    ) -> bool {
        match expr {
            Expression::MethodCall { object, .. } => {
                if let Expression::Identifier { name, .. } = &**object {
                    if name == "self" {
                        return true;
                    }
                }
                if let Expression::FieldAccess {
                    object: inner_obj, ..
                } = &**object
                {
                    if let Expression::Identifier { name, .. } = &**inner_obj {
                        if name == "self" {
                            return true;
                        }
                    }
                }
                false
            }
            _ => false,
        }
    }

    /// `if let Some(x) = map.get_mut(k)` must match directly — borrow-break to owned
    /// values prevents mutating through the binding (voxel_world chunk mutation).
    pub(in crate::codegen::rust) fn match_scrutinee_allows_mut_binding_directly(
        &self,
        expr: &Expression,
    ) -> bool {
        if let Some(Type::Option(inner)) = self.infer_expression_type(expr) {
            return matches!(inner.as_ref(), Type::MutableReference(_));
        }
        false
    }

    pub(in crate::codegen::rust) fn match_arms_mutate_self(
        &self,
        arms: &[crate::parser::MatchArm<'ast>],
    ) -> bool {
        let ctx = self_analysis::AnalysisContext::new(&[], &self.current_struct_fields);
        arms.iter()
            .any(|arm| self_analysis::expression_mutates_fields(&ctx, arm.body))
    }

    /// True when any match/if-let arm calls `self.some_method(...)`.
    pub(in crate::codegen::rust) fn match_arms_call_self_method(
        &self,
        arms: &[crate::parser::MatchArm<'ast>],
    ) -> bool {
        arms.iter()
            .any(|arm| self.expression_calls_self_method(arm.body))
    }

    fn expression_calls_self_method(&self, expr: &Expression<'ast>) -> bool {
        match expr {
            Expression::MethodCall { object, .. } => {
                matches!(&**object, Expression::Identifier { name, .. } if name == "self")
                    || self.expression_calls_self_method(object)
            }
            Expression::Block { statements, .. } => statements
                .iter()
                .any(|s| self.statement_calls_self_method(s)),
            _ => false,
        }
    }

    fn statement_calls_self_method(&self, stmt: &Statement<'ast>) -> bool {
        match stmt {
            Statement::Expression { expr, .. } => self.expression_calls_self_method(expr),
            Statement::Let { value, .. } => self.expression_calls_self_method(value),
            Statement::Return {
                value: Some(expr), ..
            } => self.expression_calls_self_method(expr),
            Statement::If {
                then_block,
                else_block,
                ..
            } => {
                then_block
                    .iter()
                    .any(|s| self.statement_calls_self_method(s))
                    || else_block
                        .as_ref()
                        .is_some_and(|b| b.iter().any(|s| self.statement_calls_self_method(s)))
            }
            Statement::Match { arms, .. } => arms
                .iter()
                .any(|arm| self.expression_calls_self_method(arm.body)),
            Statement::For { body, .. } | Statement::While { body, .. } => {
                body.iter().any(|s| self.statement_calls_self_method(s))
            }
            _ => false,
        }
    }

    /// Borrow-break on `self.field` / `self.field[i]` when arms need `&mut self` — clone enum out.
    pub(in crate::codegen::rust) fn match_borrow_break_yields_owned_clone(
        &self,
        expr: &Expression,
    ) -> bool {
        if !self.match_scrutinee_is_self_field(expr) {
            return false;
        }
        if self.match_scrutinee_option_yields_copy(expr)
            || self.match_borrow_break_yields_owned_copy_option(expr)
        {
            return false;
        }
        true
    }

    /// True when matching on `Option<&T>` where `T: Copy` (e.g. `map.get(&key)` → use `.copied()`).
    /// Dual AST forms for `recv.method(...)`: `MethodCall` or `Call(FieldAccess)`.
    fn method_call_receiver_and_name<'b>(
        expr: &'b Expression<'ast>,
    ) -> Option<(&'b Expression<'ast>, &'b str)> {
        match expr {
            Expression::MethodCall { object, method, .. } => Some((*object, method.as_str())),
            Expression::Call {
                function:
                    Expression::FieldAccess {
                        object,
                        field,
                        ..
                    },
                ..
            } => Some((*object, field.as_str())),
            _ => None,
        }
    }

    pub(in crate::codegen::rust) fn match_scrutinee_option_yields_copy(
        &self,
        expr: &Expression<'ast>,
    ) -> bool {
        if let Some(ty) = self.infer_expression_type(expr) {
            if let Type::Option(inner) = ty {
                let pointee = match inner.as_ref() {
                    Type::Reference(r) | Type::MutableReference(r) => Some(r.as_ref()),
                    _ => None,
                };
                if let Some(pointee) = pointee {
                    if self.is_type_copy(pointee) {
                        return true;
                    }
                }
            }
        }
        // Fallback: map shared-get returns `Option<&V>` in Rust. When inference loses the
        // inner Reference wrap, still use `.copied()` for Copy `V` via signature registry.
        if let Some((object, method)) = Self::method_call_receiver_and_name(expr) {
            let receiver_type = self
                .infer_type_name(object)
                .or_else(|| self.infer_indexed_element_type_name(object));
            if crate::codegen::rust::stdlib_method_traits::is_map_shared_get_call(
                method,
                receiver_type.as_deref(),
                &self.signature_registry,
            ) {
                if let Some(obj_ty) = self.infer_expression_type(object) {
                    let bare = match &obj_ty {
                        Type::Reference(inner) | Type::MutableReference(inner) => inner.as_ref(),
                        other => other,
                    };
                    if let Type::Parameterized(_, args) = bare {
                        if args.len() >= 2 && self.is_type_copy(&args[1]) {
                            return true;
                        }
                    }
                }
                // Struct field `self.map: Map<K, V>` — value type may only live in
                // the field-type registry when expression inference loses generics.
                if let Expression::FieldAccess {
                    object: root,
                    field,
                    ..
                } = object
                {
                    if matches!(root, Expression::Identifier { name, .. } if name == "self") {
                        if let Some(struct_name) = &self.current_struct_name {
                            let base = struct_name.split('<').next().unwrap_or(struct_name);
                            if let Some(fields) = self.lookup_struct_field_types(base) {
                                if let Some(Type::Parameterized(_, args)) =
                                    fields.get(field.as_str())
                                {
                                    if args.len() >= 2 && self.is_type_copy(&args[1]) {
                                        return true;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        false
    }

    /// Borrow-break on `self.method()` returning `Option<&Copy>` should use `.copied()`.
    pub(in crate::codegen::rust) fn match_borrow_break_yields_ref_copy_binding(
        &self,
        expr: &Expression<'ast>,
    ) -> bool {
        if self.match_scrutinee_option_yields_copy(expr) {
            return true;
        }
        let Some((object, method)) = Self::method_call_receiver_and_name(expr) else {
            return false;
        };
        // `self.field.get(key)` on Map/HashMap<_, Copy>: prefer field-type registry.
        // Local signature registries often lack `Map::get` on the first multipass;
        // fall back to stdlib `HashMap::get` / field generics.
        if let Expression::FieldAccess {
            object: root,
            field,
            ..
        } = object
        {
            if matches!(root, Expression::Identifier { name, .. } if name == "self") {
                if let Some(struct_name) = &self.current_struct_name {
                    let base = struct_name.split('<').next().unwrap_or(struct_name);
                    if let Some(fields) = self.lookup_struct_field_types(base) {
                        if let Some(field_ty) = fields.get(field.as_str()) {
                            let bare = match field_ty {
                                Type::Reference(inner) | Type::MutableReference(inner) => {
                                    inner.as_ref()
                                }
                                other => other,
                            };
                            if let Type::Parameterized(map_name, args) = bare {
                                if crate::codegen::rust::stdlib_method_traits::is_map_type_name(
                                    map_name,
                                ) && args.len() >= 2
                                    && self.is_type_copy(&args[1])
                                {
                                    let stdlib = crate::analyzer::SignatureRegistry::stdlib();
                                    let is_shared_get = crate::codegen::rust::stdlib_method_traits::is_map_shared_get_call(
                                        method,
                                        Some(map_name.as_str()),
                                        &self.signature_registry,
                                    ) || crate::codegen::rust::stdlib_method_traits::is_map_shared_get_call(
                                        method,
                                        Some(map_name.as_str()),
                                        &stdlib,
                                    ) || crate::codegen::rust::stdlib_method_traits::is_map_shared_get_call(
                                        method,
                                        Some("HashMap"),
                                        &stdlib,
                                    );
                                    if is_shared_get {
                                        return true;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        let receiver_type = self
            .infer_type_name(object)
            .or_else(|| self.infer_indexed_element_type_name(object));
        crate::codegen::rust::stdlib_method_traits::is_map_shared_get_call(
            method,
            receiver_type.as_deref(),
            &self.signature_registry,
        ) || crate::codegen::rust::stdlib_method_traits::is_map_shared_get_call(
            method,
            receiver_type.as_deref(),
            &crate::analyzer::SignatureRegistry::stdlib(),
        )
    }

    /// Borrow-break on `self.method()` returning owned `Option<Copy>` — match directly, no `.copied()`.
    pub(in crate::codegen::rust) fn match_borrow_break_yields_owned_copy_option(
        &self,
        expr: &Expression,
    ) -> bool {
        let Some(ty) = self.infer_expression_type(expr) else {
            return false;
        };
        let Type::Option(inner) = ty else {
            return false;
        };
        match inner.as_ref() {
            Type::Reference(_) | Type::MutableReference(_) => false,
            other => self.is_type_copy(other),
        }
    }

    /// Borrow-break when scrutinee yields owned `Option<T>` (Copy or not) — bind the
    /// Option by value and match it. Avoids `.map(|v| v.to_owned()).as_ref()` which
    /// forces cloning the method receiver (`self.network.clone().poll(...)`).
    pub(in crate::codegen::rust) fn match_borrow_break_yields_owned_option(
        &self,
        expr: &Expression,
    ) -> bool {
        let Some(ty) = self.infer_expression_type(expr) else {
            return false;
        };
        let Type::Option(inner) = ty else {
            return false;
        };
        !matches!(
            inner.as_ref(),
            Type::Reference(_) | Type::MutableReference(_)
        )
    }

    pub(in crate::codegen::rust) fn get_assignment_target_type(
        &self,
        target: &Expression,
    ) -> Option<String> {
        match target {
            Expression::FieldAccess { object, field, .. } => {
                if matches!(&**object, Expression::Identifier { name, .. } if name == "self") {
                    if let Some(struct_name) = &self.current_struct_name {
                        let base_name = struct_name.split('<').next().unwrap_or(struct_name);
                        if let Some(usize_fields) = self.usize_struct_fields.get(base_name) {
                            if usize_fields.contains(field) {
                                return Some("usize".to_string());
                            }
                        }
                        if let Some(fields) = self.lookup_struct_field_types(base_name) {
                            if let Some(ty) = fields.get(field.as_str()) {
                                if matches!(ty, Type::Custom(n) if n == "usize") {
                                    return Some("usize".to_string());
                                }
                                if matches!(ty, Type::Custom(n) if n == "f32") {
                                    return Some("f32".to_string());
                                }
                                if matches!(ty, Type::Custom(n) if n == "i32") {
                                    return Some("i32".to_string());
                                }
                                if matches!(ty, Type::Custom(n) if n == "i64" || n == "int") {
                                    return Some("i64".to_string());
                                }
                            }
                        }
                    }
                }
            }
            Expression::Identifier { name, .. } => {
                if self.usize_variables.contains(name) {
                    return Some("usize".to_string());
                }
                if let Some(ty) = self.local_var_types.get(name) {
                    return Self::assignment_cast_target_for_type(ty);
                }
                return None;
            }
            _ => {}
        }
        None
    }

    fn assignment_cast_target_for_type(ty: &Type) -> Option<String> {
        match ty {
            Type::Int => Some("int".to_string()),
            Type::Custom(name) if name == "int" || name == "i64" => Some("int".to_string()),
            Type::Custom(name) if name == "i32" => Some("i32".to_string()),
            Type::Custom(name) if name == "usize" => Some("usize".to_string()),
            Type::Option(inner) => Self::assignment_cast_target_for_type(inner),
            _ => None,
        }
    }

    pub(in crate::codegen::rust) fn returns_option_owned_type(&self) -> bool {
        match &self.current_function_return_type {
            Some(Type::Option(inner_type)) => {
                !matches!(**inner_type, Type::Reference(_) | Type::MutableReference(_))
            }
            _ => false,
        }
    }

    /// `if let Some(x) = self.opt { self.opt = Some(x) }` needs owned clone borrow-break.
    pub(in crate::codegen::rust) fn option_arm_reassigns_scrutinee_field(
        &self,
        scrutinee: &Expression,
        body: &Expression<'ast>,
    ) -> bool {
        if !self.match_scrutinee_is_self_field(scrutinee) {
            return false;
        }
        self.expression_assigns_to_expression(body, scrutinee)
    }

    /// Owned `__match_borrow_break` temps need `let mut` when an arm mutates a
    /// pattern binding or writebacks through the scrutinee (E0596 otherwise).
    pub(in crate::codegen::rust) fn match_borrow_break_temp_needs_mut(
        &self,
        arms: &[crate::parser::MatchArm<'ast>],
        scrutinee: &Expression,
    ) -> bool {
        arms.iter().any(|arm| {
            if self.option_arm_reassigns_scrutinee_field(scrutinee, arm.body) {
                return true;
            }
            if self.option_arm_mutates_some_binding(arm, scrutinee) {
                return true;
            }
            let body_stmts: &[&Statement] = if let Expression::Block { statements, .. } = arm.body {
                statements.as_slice()
            } else {
                &[]
            };
            let mut bound = std::collections::HashSet::new();
            self.extract_pattern_bindings(&arm.pattern, &mut bound);
            bound.iter().any(|name| {
                body_stmts
                    .iter()
                    .any(|s| self.statement_mutates_variable_field(s, name))
                    || body_stmts
                        .iter()
                        .any(|s| self.statement_nonreadonly_method_call_on_var(s, name))
            })
        })
    }

    fn expression_assigns_to_expression(
        &self,
        expr: &Expression<'ast>,
        target: &Expression,
    ) -> bool {
        match expr {
            Expression::Block { statements, .. } => statements
                .iter()
                .any(|s| self.statement_assigns_to_expression(s, target)),
            _ => false,
        }
    }

    fn statement_assigns_to_expression(&self, stmt: &Statement<'ast>, target: &Expression) -> bool {
        match stmt {
            Statement::Assignment { target: lhs, .. } => {
                self.expressions_equivalent_for_assign(lhs, target)
            }
            Statement::Expression { expr, .. } => {
                self.expression_assigns_to_expression(expr, target)
            }
            Statement::If {
                then_block,
                else_block,
                ..
            } => {
                then_block
                    .iter()
                    .any(|s| self.statement_assigns_to_expression(s, target))
                    || else_block.as_ref().is_some_and(|b| {
                        b.iter()
                            .any(|s| self.statement_assigns_to_expression(s, target))
                    })
            }
            _ => false,
        }
    }

    fn expressions_equivalent_for_assign(&self, left: &Expression, target: &Expression) -> bool {
        match (left, target) {
            (
                Expression::FieldAccess {
                    object: l_obj,
                    field: l_field,
                    ..
                },
                Expression::FieldAccess {
                    object: r_obj,
                    field: r_field,
                    ..
                },
            ) => l_field == r_field && self.expressions_equivalent_for_assign(l_obj, r_obj),
            (Expression::Identifier { name: l, .. }, Expression::Identifier { name: r, .. }) => {
                l == r
            }
            (
                Expression::Index {
                    object: l_obj,
                    index: l_idx,
                    ..
                },
                Expression::Index {
                    object: r_obj,
                    index: r_idx,
                    ..
                },
            ) => {
                self.expressions_equivalent_for_assign(l_obj, r_obj)
                    && self.expressions_equivalent_for_assign(l_idx, r_idx)
            }
            // `self.slots[i as usize] = ...` must match scrutinee `self.slots[i as usize]`
            // so option reassignment forces owned clone borrow-break (not overlapping `&mut`).
            (
                Expression::Cast {
                    expr: l_expr,
                    type_: l_ty,
                    ..
                },
                Expression::Cast {
                    expr: r_expr,
                    type_: r_ty,
                    ..
                },
            ) => l_ty == r_ty && self.expressions_equivalent_for_assign(l_expr, r_expr),
            (
                Expression::Unary {
                    op: l_op,
                    operand: l_operand,
                    ..
                },
                Expression::Unary {
                    op: r_op,
                    operand: r_operand,
                    ..
                },
            ) => l_op == r_op && self.expressions_equivalent_for_assign(l_operand, r_operand),
            (
                Expression::Binary {
                    left: l_left,
                    op: l_op,
                    right: l_right,
                    ..
                },
                Expression::Binary {
                    left: r_left,
                    op: r_op,
                    right: r_right,
                    ..
                },
            ) => {
                l_op == r_op
                    && self.expressions_equivalent_for_assign(l_left, r_left)
                    && self.expressions_equivalent_for_assign(l_right, r_right)
            }
            _ => false,
        }
    }
}
