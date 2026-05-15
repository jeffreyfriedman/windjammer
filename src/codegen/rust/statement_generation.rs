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

use crate::parser::ast::CompoundOp;
use crate::parser::*;

use super::codegen_helpers;
use super::pattern_analysis;
use super::self_analysis;
use super::string_analysis;
use super::string_utilities;
use super::CodeGenerator;

#[allow(clippy::collapsible_match, clippy::collapsible_if)]
impl<'ast> CodeGenerator<'ast> {
    /// Whether `assignment_float_target_type` should be set for the whole assignment/compound RHS
    /// (float literals + mixed f32/f64 arithmetic toward an f32 or f64 slot).
    pub(in crate::codegen::rust) fn assignment_target_needs_float_codegen_context(ty: &Type) -> bool {
        match ty {
            Type::Reference(inner) | Type::MutableReference(inner) => {
                Self::assignment_target_needs_float_codegen_context(inner)
            }
            Type::Custom(name) if name == "f32" || name == "f64" => true,
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
    pub(in crate::codegen::rust) fn resolve_compound_assign_float_target(&self, target: &Expression) -> Option<&'static str> {
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
        if let Some(fi) = &self.float_inference {
            use crate::type_inference::FloatType;
            match fi.get_float_type(target) {
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
    pub(in crate::codegen::rust) fn branch_tail_suggests_owned_string_coercion(&self, block: &[&'ast Statement<'ast>]) -> bool {
        let Some(last) = block.last().copied() else {
            return false;
        };
        match last {
            Statement::Expression { expr, .. } => self.expr_suggests_owned_string_coercion(expr),
            _ => false,
        }
    }

    pub(in crate::codegen::rust) fn generate_statement_impl(&mut self, stmt: &Statement<'ast>) -> String {
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
            } => {
                let mut output = self.indent();

                // Special case: string constants should use &'static str, not String
                let rust_type = if matches!(type_, Type::String)
                    && matches!(
                        value,
                        Expression::Literal {
                            value: Literal::String(_),
                            ..
                        }
                    ) {
                    "&'static str".to_string()
                } else {
                    self.type_to_rust(type_)
                };

                output.push_str(&format!(
                    "const {}: {} = {};\n",
                    name,
                    rust_type,
                    self.generate_expression(value)
                ));
                output
            }
            Statement::Static {
                name,
                mutable,
                type_,
                value,
                ..
            } => {
                let mut output = self.indent();
                if *mutable {
                    output.push_str(&format!(
                        "static mut {}: {} = {};\n",
                        name,
                        self.type_to_rust(type_),
                        self.generate_expression(value)
                    ));
                } else {
                    output.push_str(&format!(
                        "static {}: {} = {};\n",
                        name,
                        self.type_to_rust(type_),
                        self.generate_expression(value)
                    ));
                }
                output
            }
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
            Statement::Match { value, arms, .. } => self.generate_match_statement(value, arms),
            Statement::Loop { body, .. } => {
                let mut output = self.indent();
                output.push_str("loop {\n");

                self.indent_level += 1;
                let saved_idx = self.current_statement_idx;
                let saved_local_idx = self.current_block_local_idx;
                for (i, stmt) in body.iter().enumerate() {
                    self.current_statement_idx = self.auto_clone_counter;
                    self.current_block_local_idx = i;
                    self.auto_clone_counter += 1;
                    output.push_str(&self.generate_statement(stmt));
                }
                self.current_statement_idx = saved_idx;
                self.current_block_local_idx = saved_local_idx;
                self.indent_level -= 1;

                output.push_str(&self.indent());
                output.push_str("}\n");
                output
            }
            Statement::While {
                condition, body, ..
            } => {
                // TDD FIX (Bug #3): Before generating while condition expression,
                // check if it compares a variable to .len() - if so, mark that variable as usize
                // This must happen BEFORE generate_expression to prevent `as i64` cast
                self.mark_usize_variables_in_condition(condition);

                let mut output = self.indent();
                output.push_str("while ");

                let condition_str = self.generate_expression(condition);
                output.push_str(&condition_str);
                output.push_str(" {\n");

                self.indent_level += 1;
                let saved_body = self.current_function_body.clone();
                let saved_idx = self.current_statement_idx;
                let saved_local_idx = self.current_block_local_idx;
                self.current_function_body = body.to_vec();
                for (i, stmt) in body.iter().enumerate() {
                    self.current_statement_idx = self.auto_clone_counter;
                    self.current_block_local_idx = i;
                    self.auto_clone_counter += 1;
                    output.push_str(&self.generate_statement(stmt));
                }
                self.current_function_body = saved_body;
                self.current_statement_idx = saved_idx;
                self.current_block_local_idx = saved_local_idx;
                self.indent_level -= 1;

                output.push_str(&self.indent());
                output.push_str("}\n");
                output
            }
            Statement::For {
                pattern,
                iterable,
                body,
                location,
                ..
            } => self.generate_for_statement(pattern, iterable, body, location),
            Statement::Break { .. } => {
                let mut output = self.indent();
                output.push_str("break;\n");
                output
            }
            Statement::Continue { .. } => {
                let mut output = self.indent();
                output.push_str("continue;\n");
                output
            }
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
            Statement::Thread { body, .. } => {
                // Transpile to std::thread::spawn for parallelism
                // When used as a statement, discard the JoinHandle
                let mut output = self.indent();
                output.push_str("let _ = std::thread::spawn(move || {\n");

                self.indent_level += 1;
                for stmt in body {
                    output.push_str(&self.generate_statement(stmt));
                }
                self.indent_level -= 1;

                output.push_str(&self.indent());
                output.push_str("});\n");
                output
            }
            Statement::Async { body, .. } => {
                // Transpile to tokio::spawn for async concurrency
                // When used as a statement, discard the JoinHandle
                let mut output = self.indent();
                output.push_str("let _ = tokio::spawn(async move {\n");

                self.indent_level += 1;
                for stmt in body {
                    output.push_str(&self.generate_statement(stmt));
                }
                self.indent_level -= 1;

                output.push_str(&self.indent());
                output.push_str("});\n");
                output
            }
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

    pub(super) fn extract_pattern_bindings(
        &self,
        pattern: &Pattern,
        bindings: &mut std::collections::HashSet<String>,
    ) {
        use crate::parser::EnumPatternBinding;
        match pattern {
            Pattern::Identifier(name) | Pattern::MutBinding(name) => {
                bindings.insert(name.clone());
            }
            Pattern::Reference(inner) => {
                self.extract_pattern_bindings(inner, bindings);
            }
            Pattern::Ref(name) | Pattern::RefMut(name) => {
                bindings.insert(name.clone());
            }
            Pattern::EnumVariant(_name, binding) => match binding {
                EnumPatternBinding::Single(var_name) => {
                    bindings.insert(var_name.clone());
                }
                EnumPatternBinding::Tuple(patterns) => {
                    for pat in patterns {
                        self.extract_pattern_bindings(pat, bindings);
                    }
                }
                EnumPatternBinding::Struct(fields, _) => {
                    for (_field_name, pat) in fields {
                        self.extract_pattern_bindings(pat, bindings);
                    }
                }
                EnumPatternBinding::Wildcard | EnumPatternBinding::None => {}
            },
            Pattern::Tuple(patterns) => {
                for pat in patterns {
                    self.extract_pattern_bindings(pat, bindings);
                }
            }
            Pattern::Or(patterns) => {
                for pat in patterns {
                    self.extract_pattern_bindings(pat, bindings);
                }
            }
            Pattern::Wildcard | Pattern::Literal(_) => {}
        }
    }

    /// Upgrade pattern bindings to `mut` when the body mutates them.
    /// E.g. `if let Some(v) = opt { v.push(1) }` → `if let Some(mut v) = ...`
    /// When `scrutinee_is_ref` is true, use `ref mut` instead of `mut` (borrowed context).
    pub(super) fn upgrade_pattern_mut_bindings<'s>(
        &self,
        pattern: &Pattern<'s>,
        body_stmts: &[&Statement<'s>],
        scrutinee_is_ref: bool,
    ) -> Pattern<'s> {
        use crate::parser::EnumPatternBinding;
        match pattern {
            Pattern::Identifier(name) => {
                let is_mutated = body_stmts.iter().any(|stmt| {
                    self.statement_mutates_variable_field(stmt, name)
                        || (scrutinee_is_ref
                            && self.statement_nonreadonly_method_call_on_var(stmt, name))
                });
                if is_mutated {
                    if scrutinee_is_ref {
                        Pattern::RefMut(name.clone())
                    } else {
                        Pattern::MutBinding(name.clone())
                    }
                } else {
                    pattern.clone()
                }
            }
            Pattern::EnumVariant(variant, binding) => {
                let new_binding = match binding {
                    EnumPatternBinding::Single(name) => {
                        let is_mutated = body_stmts.iter().any(|stmt| {
                            self.statement_mutates_variable_field(stmt, name)
                                || (scrutinee_is_ref
                                    && self.statement_nonreadonly_method_call_on_var(stmt, name))
                        });
                        if is_mutated {
                            if scrutinee_is_ref {
                                EnumPatternBinding::Tuple(vec![Pattern::RefMut(name.clone())])
                            } else {
                                EnumPatternBinding::Tuple(vec![Pattern::MutBinding(name.clone())])
                            }
                        } else {
                            binding.clone()
                        }
                    }
                    EnumPatternBinding::Tuple(patterns) => {
                        let new_patterns: Vec<Pattern<'s>> = patterns
                            .iter()
                            .map(|p| {
                                self.upgrade_pattern_mut_bindings(p, body_stmts, scrutinee_is_ref)
                            })
                            .collect();
                        EnumPatternBinding::Tuple(new_patterns)
                    }
                    other => other.clone(),
                };
                Pattern::EnumVariant(variant.clone(), new_binding)
            }
            Pattern::Tuple(patterns) => {
                let new_patterns: Vec<Pattern<'s>> = patterns
                    .iter()
                    .map(|p| self.upgrade_pattern_mut_bindings(p, body_stmts, scrutinee_is_ref))
                    .collect();
                Pattern::Tuple(new_patterns)
            }
            _ => pattern.clone(),
        }
    }

    /// E0507: `let x = vec[i]` must not lower to a plain `vec[i]` move when the element type is not
    /// `Copy`. Prefer `&vec[i]` when the binding is only used for field reads; otherwise
    /// `vec[i].clone()` (or `(&vec[i]).clone()` → `vec[i].clone()` after stripping the leading `&`).
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
    /// - Option<T> behind &mut self → `.take()` (atomically moves value out, leaves None)
    /// - Other non-Copy behind &self/&mut self → `.clone()`
    pub(in crate::codegen::rust) fn apply_self_field_move_fix(&self, value: &Expression<'ast>, value_str: &mut String) {
        if !matches!(value, Expression::FieldAccess { .. }) {
            return;
        }
        let root = self.root_identifier_of_field_or_index_chain(value);
        let is_self_borrowed = root.is_some_and(|r| {
            r == "self"
                && (self.inferred_borrowed_params.contains("self")
                    || self.inferred_mut_borrowed_params.contains("self"))
        });
        if !is_self_borrowed {
            return;
        }
        let Some(ty) = self.infer_expression_type(value) else {
            return;
        };
        if self.is_type_copy(&ty)
            || value_str.ends_with(".clone()")
            || value_str.ends_with(".take()")
        {
            return;
        }
        let is_option =
            matches!(&ty, Type::Option(_)) || matches!(&ty, Type::Custom(n) if n == "Option");
        let self_is_mut = self.inferred_mut_borrowed_params.contains("self");
        if is_option && self_is_mut {
            *value_str = format!("{}.take()", value_str);
        } else {
            *value_str = format!("{}.clone()", value_str);
        }
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

    pub(super) fn match_expression_binds_refs(&self, expr: &Expression) -> bool {
        // When the scrutinee evaluates to a Copy type (or &CopyType where
        // CopyType has no inner references), match ergonomics auto-copy the
        // value, so pattern bindings are owned — not refs. Skip ONLY for
        // types without inner references (e.g. a Copy enum with i32 payloads,
        // NOT Option<&str> which borrows through the reference).
        if let Some(ty) = self.infer_expression_type(expr) {
            let inner = match &ty {
                Type::Reference(inner) | Type::MutableReference(inner) => inner.as_ref(),
                other => other,
            };
            if self.is_type_copy(inner) && !Self::type_contains_reference(inner) {
                return false;
            }
        }

        match expr {
            Expression::Unary {
                op: crate::parser::UnaryOp::Ref | crate::parser::UnaryOp::MutRef,
                ..
            } => true,

            Expression::Identifier { name, .. } => self.identifier_is_borrowed_or_self(name),

            Expression::FieldAccess { .. } | Expression::Index { .. } => {
                if let Some(root) = self.root_identifier_of_field_or_index_chain(expr) {
                    if self.identifier_is_borrowed_or_self(root) {
                        return true;
                    }
                }
                false
            }

            Expression::MethodCall { method, object, .. } => {
                let type_name = self.infer_type_name(object);
                let sig = if let Some(ref type_name) = type_name {
                    let qualified = format!("{}::{}", type_name, method);
                    self.signature_registry.get_signature(&qualified)
                } else {
                    self.signature_registry.get_signature(method)
                };
                if let Some(sig) = sig {
                    if let Some(ref ret_type) = sig.return_type {
                        Self::type_contains_reference(ret_type)
                    } else {
                        false
                    }
                } else {
                    false
                }
            }

            Expression::Call { function, .. } => {
                let func_name =
                    crate::codegen::rust::ast_utilities::extract_function_name(function);
                if !func_name.is_empty() {
                    if let Some(sig) = self.signature_registry.get_signature(&func_name) {
                        if let Some(ref ret_type) = sig.return_type {
                            return Self::type_contains_reference(ret_type);
                        }
                    }
                }
                false
            }

            _ => false,
        }
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

    /// `&` / `&mut` prefix for matching on `Option` when the scrutinee lives behind a borrow.
    pub(in crate::codegen::rust) fn option_scrutinee_ref_prefix(&self, value: &Expression<'ast>) -> &'static str {
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

    /// When `&self` + `if let Some(x) = self.opt` but the arm calls mutating methods on `x`, use `&mut`.
    pub(in crate::codegen::rust) fn effective_option_scrutinee_ref_prefix(
        &self,
        value: &Expression<'ast>,
        some_arm: Option<&MatchArm<'ast>>,
    ) -> &'static str {
        let base = self.option_scrutinee_ref_prefix(value);
        if base == "&" {
            if let Some(arm) = some_arm {
                if self.option_match_needs_mut_scrutinee_for_some_arm(arm, value) {
                    return "&mut ";
                }
            }
            // When the Option's inner type is Copy and the arm body doesn't mutate
            // the binding, no `&` prefix is needed — Option<Copy> auto-copies.
            if let Some(Type::Option(inner)) = self.infer_expression_type(value) {
                if self.is_type_copy(&inner) {
                    return "";
                }
            }
        }
        if base == "&mut " {
            // For Copy inner types, strip &mut UNLESS the body calls
            // mutating methods on the binding.  Copy values auto-copy on
            // destructure, so comparisons/arithmetic/returns work with owned
            // T.  But if the body calls methods that take &mut self on the
            // binding, we need &mut to (a) compile and (b) propagate
            // mutations back through self.field.
            if let Some(Type::Option(inner)) = self.infer_expression_type(value) {
                if self.is_type_copy(&inner) {
                    let body_mutates = some_arm
                        .and_then(|arm| {
                            Self::some_pattern_single_binding(&arm.pattern)
                                .map(|b| {
                                    self.binding_receives_mutating_call_with_sig_check(
                                        arm.body, b, &inner,
                                    )
                                })
                        })
                        .unwrap_or(false);
                    if !body_mutates {
                        return "";
                    }
                }
            }
        }
        base
    }

    pub(in crate::codegen::rust) fn some_pattern_single_binding<'p>(pattern: &'p Pattern<'p>) -> Option<&'p str> {
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
        self.expr_binding_receives_mutating_method_call(main_arm.body, b)
    }

    fn statement_binding_mut_method_scan(&self, stmt: &Statement<'ast>, binding: &str) -> bool {
        match stmt {
            Statement::Assignment { target, .. } => {
                super::self_analysis::expression_references_variable_or_field(target, binding)
            }
            Statement::Expression { expr, .. } => {
                self.expr_binding_receives_mutating_method_call(expr, binding)
            }
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.expr_binding_receives_mutating_method_call(condition, binding)
                    || then_block
                        .iter()
                        .any(|s| self.statement_binding_mut_method_scan(s, binding))
                    || else_block.as_ref().is_some_and(|b| {
                        b.iter()
                            .any(|s| self.statement_binding_mut_method_scan(s, binding))
                    })
            }
            Statement::While {
                condition, body, ..
            } => {
                self.expr_binding_receives_mutating_method_call(condition, binding)
                    || body
                        .iter()
                        .any(|s| self.statement_binding_mut_method_scan(s, binding))
            }
            Statement::For { body, .. } => body
                .iter()
                .any(|s| self.statement_binding_mut_method_scan(s, binding)),
            Statement::Loop { body, .. } => body
                .iter()
                .any(|s| self.statement_binding_mut_method_scan(s, binding)),
            Statement::Return {
                value: Some(expr), ..
            } => self.expr_binding_receives_mutating_method_call(expr, binding),
            Statement::Let { value, .. } => {
                self.expr_binding_receives_mutating_method_call(value, binding)
            }
            Statement::Match { value, arms, .. } => {
                self.expr_binding_receives_mutating_method_call(value, binding)
                    || arms.iter().any(|arm| {
                        self.expr_binding_receives_mutating_method_call(arm.body, binding)
                    })
            }
            _ => false,
        }
    }

    fn expr_binding_receives_mutating_method_call(
        &self,
        expr: &Expression<'ast>,
        binding: &str,
    ) -> bool {
        match expr {
            Expression::Block { statements, .. } => statements
                .iter()
                .any(|s| self.statement_binding_mut_method_scan(s, binding)),
            Expression::MethodCall { object, method, .. } => {
                if let Expression::Identifier { name, .. } = &**object {
                    if name == binding && self.codegen_method_likely_mutates_receiver(method) {
                        return true;
                    }
                }
                self.expr_binding_receives_mutating_method_call(object, binding)
            }
            Expression::Binary { left, right, .. } => {
                self.expr_binding_receives_mutating_method_call(left, binding)
                    || self.expr_binding_receives_mutating_method_call(right, binding)
            }
            Expression::Unary { operand, .. } => {
                self.expr_binding_receives_mutating_method_call(operand, binding)
            }
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                if let Expression::FieldAccess { object, field, .. } = &**function {
                    if let Expression::Identifier { name, .. } = &**object {
                        if name == binding && self.codegen_method_likely_mutates_receiver(field) {
                            return true;
                        }
                    }
                }
                self.expr_binding_receives_mutating_method_call(function, binding)
                    || arguments
                        .iter()
                        .any(|(_, a)| self.expr_binding_receives_mutating_method_call(a, binding))
            }
            _ => false,
        }
    }

    fn codegen_method_likely_mutates_receiver(&self, method: &str) -> bool {
        crate::method_registry::mutates_receiver(method)
    }

    /// Like `expr_binding_receives_mutating_method_call` but also consults
    /// the signature registry for user-defined methods on `binding_type`.
    fn binding_receives_mutating_call_with_sig_check(
        &self,
        expr: &Expression<'ast>,
        binding: &str,
        binding_type: &Type,
    ) -> bool {
        match expr {
            Expression::Block { statements, .. } => statements.iter().any(|s| {
                self.stmt_binding_mut_call_with_sig(s, binding, binding_type)
            }),
            Expression::MethodCall { object, method, .. } => {
                if let Expression::Identifier { name, .. } = &**object {
                    if name == binding
                        && self.method_mutates_via_registry_or_sig(method, binding_type)
                    {
                        return true;
                    }
                }
                self.binding_receives_mutating_call_with_sig_check(object, binding, binding_type)
            }
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                if let Expression::FieldAccess { object, field, .. } = &**function {
                    if let Expression::Identifier { name, .. } = &**object {
                        if name == binding
                            && self.method_mutates_via_registry_or_sig(field, binding_type)
                        {
                            return true;
                        }
                    }
                }
                self.binding_receives_mutating_call_with_sig_check(function, binding, binding_type)
                    || arguments.iter().any(|(_, a)| {
                        self.binding_receives_mutating_call_with_sig_check(a, binding, binding_type)
                    })
            }
            Expression::Binary { left, right, .. } => {
                self.binding_receives_mutating_call_with_sig_check(left, binding, binding_type)
                    || self.binding_receives_mutating_call_with_sig_check(
                        right,
                        binding,
                        binding_type,
                    )
            }
            Expression::Unary { operand, .. } => {
                self.binding_receives_mutating_call_with_sig_check(operand, binding, binding_type)
            }
            _ => false,
        }
    }

    fn stmt_binding_mut_call_with_sig(
        &self,
        stmt: &Statement<'ast>,
        binding: &str,
        binding_type: &Type,
    ) -> bool {
        match stmt {
            Statement::Assignment { target, .. } => {
                super::self_analysis::expression_references_variable_or_field(target, binding)
            }
            Statement::Expression { expr, .. } => {
                self.binding_receives_mutating_call_with_sig_check(expr, binding, binding_type)
            }
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.binding_receives_mutating_call_with_sig_check(
                    condition,
                    binding,
                    binding_type,
                ) || then_block
                    .iter()
                    .any(|s| self.stmt_binding_mut_call_with_sig(s, binding, binding_type))
                    || else_block.as_ref().is_some_and(|b| {
                        b.iter()
                            .any(|s| self.stmt_binding_mut_call_with_sig(s, binding, binding_type))
                    })
            }
            Statement::Match { value, arms, .. } => {
                self.binding_receives_mutating_call_with_sig_check(value, binding, binding_type)
                    || arms.iter().any(|arm| {
                        self.binding_receives_mutating_call_with_sig_check(
                            arm.body,
                            binding,
                            binding_type,
                        )
                    })
            }
            Statement::Return { value, .. } => value
                .map(|v| {
                    self.binding_receives_mutating_call_with_sig_check(v, binding, binding_type)
                })
                .unwrap_or(false),
            _ => false,
        }
    }

    /// Check if a method on the given type is known to mutate its receiver,
    /// using both the stdlib method registry and the signature registry.
    fn method_mutates_via_registry_or_sig(&self, method: &str, receiver_type: &Type) -> bool {
        if crate::method_registry::mutates_receiver(method) {
            return true;
        }
        let type_name = match receiver_type {
            Type::Custom(name) => name.as_str(),
            _ => return false,
        };
        let qualified = format!("{}::{}", type_name, method);
        if let Some(sig) = self.signature_registry.get_signature(&qualified) {
            if sig.has_self_receiver && !sig.param_ownership.is_empty() {
                return sig.param_ownership[0]
                    == crate::analyzer::OwnershipMode::MutBorrowed;
            }
        }
        false
    }

    /// Check if expression is self.field (or self.field.subfield) - traces to self
    pub(in crate::codegen::rust) fn match_scrutinee_is_self_field(&self, expr: &Expression) -> bool {
        match expr {
            Expression::FieldAccess { object, .. } => {
                matches!(&**object, Expression::Identifier { name, .. } if name == "self")
                    || self.match_scrutinee_is_self_field(object)
            }
            Expression::Index { object, .. } => self.match_scrutinee_is_self_field(object),
            _ => false,
        }
    }

    pub(in crate::codegen::rust) fn match_scrutinee_is_self_method_call(&self, expr: &Expression) -> bool {
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

    pub(in crate::codegen::rust) fn match_arms_mutate_self(&self, arms: &[crate::parser::MatchArm<'ast>]) -> bool {
        let ctx = self_analysis::AnalysisContext::new(&[], &self.current_struct_fields);
        arms.iter()
            .any(|arm| self_analysis::expression_mutates_fields(&ctx, arm.body))
    }

    pub(in crate::codegen::rust) fn get_assignment_target_type(&self, target: &Expression) -> Option<String> {
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
                        return Some("i64".to_string());
                    }
                }
            }
            Expression::Identifier { name, .. } => {
                if self.usize_variables.contains(name) {
                    return Some("usize".to_string());
                }
                return None;
            }
            _ => {}
        }
        None
    }

    pub(in crate::codegen::rust) fn returns_option_owned_type(&self) -> bool {
        match &self.current_function_return_type {
            Some(Type::Option(inner_type)) => {
                !matches!(**inner_type, Type::Reference(_) | Type::MutableReference(_))
            }
            _ => false,
        }
    }
}
