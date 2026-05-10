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
    fn assignment_target_needs_float_codegen_context(ty: &Type) -> bool {
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

    fn is_float_numeric_type(t: &Type) -> bool {
        match t {
            Type::Float => true,
            Type::Custom(n) => n == "f32" || n == "f64",
            _ => false,
        }
    }

    fn is_int_numeric_type(t: &Type) -> bool {
        match t {
            Type::Int | Type::Int32 | Type::Uint => true,
            Type::Custom(n) => matches!(
                n.as_str(),
                "i32" | "u32" | "i64" | "u64" | "usize" | "isize" | "i8" | "u8" | "i16" | "u16"
            ),
            _ => false,
        }
    }

    fn float_type_name(t: &Type) -> &'static str {
        match t {
            Type::Custom(n) if n == "f64" => "f64",
            Type::Float => "f64",
            _ => "f32",
        }
    }

    /// Determine the concrete Rust float type name for a compound assignment target.
    /// Priority: explicit type annotation → float inference engine → assignment context → inferred type.
    fn resolve_compound_assign_float_target(&self, target: &Expression) -> Option<&'static str> {
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

    pub(crate) fn generate_block(&mut self, stmts: &[&'ast Statement<'ast>]) -> String {
        let mut output = String::new();
        let len = stmts.len();
        let saved_body = self.current_function_body.clone();
        let saved_idx = self.current_statement_idx;
        let saved_local_idx = self.current_block_local_idx;
        self.current_function_body = stmts.to_vec();
        for (i, stmt) in stmts.iter().enumerate() {
            self.current_statement_idx = self.auto_clone_counter;
            self.current_block_local_idx = i;
            self.auto_clone_counter += 1;

            let is_last = i == len - 1;
            // TDD: Track if this is the last statement (used by If handler)
            self.current_is_last_statement = is_last;
            // TDD FIX: Only optimize return statements in function body (not nested blocks)
            let should_optimize_return =
                self.in_function_body && matches!(stmt, Statement::Return { .. });
            if is_last
                && !self.in_void_block
                && (should_optimize_return
                    || matches!(
                        stmt,
                        Statement::Expression { .. }
                            | Statement::Thread { .. }
                            | Statement::Async { .. }
                    ))
            {
                // Last statement is an expression, thread/async block, or return - generate as implicit return
                match stmt {
                    Statement::Expression { expr, .. } => {
                        output.push_str(&self.indent());
                        let old_coerce_lit = self.coerce_string_literals_to_owned;
                        if self.in_function_body
                            && string_utilities::return_type_expects_owned_string(
                                &self.current_function_return_type,
                            )
                        {
                            self.coerce_string_literals_to_owned = true;
                        }
                        let mut expr_str = self.generate_expression(expr);
                        self.coerce_string_literals_to_owned = old_coerce_lit;

                        // TDD FIX: Borrowed iterator vars need deref when returned as Copy types
                        // For `for (_, val) in &vec` where val: &i32, implicit return `val` needs `*val`
                        if let Expression::Identifier { name, .. } = expr {
                            if self.borrowed_iterator_vars.contains(name) {
                                let return_type_is_copy = self
                                    .current_function_return_type
                                    .as_ref()
                                    .is_some_and(|t| self.is_type_copy(t));
                                if return_type_is_copy && !expr_str.starts_with('*') {
                                    expr_str = format!("*{}", expr_str);
                                }
                            }
                        }

                        // Deref local vars with reference types when returning Copy
                        // e.g., `let (id, name) = &items[0]; id` → id is &i32, return i32 → *id
                        // Also handles &mut refs: `let x = n; x` where n: &mut i32, return i32
                        if let Expression::Identifier { .. } = expr {
                            let expects_owned = !matches!(
                                &self.current_function_return_type,
                                Some(Type::Reference(_)) | Some(Type::MutableReference(_))
                            );
                            if expects_owned {
                                if let Some(
                                    Type::Reference(inner) | Type::MutableReference(inner),
                                ) = self.infer_expression_type(expr)
                                {
                                    if self.is_type_copy(inner.as_ref())
                                        && !expr_str.starts_with('*')
                                    {
                                        expr_str = format!("*{}", expr_str);
                                    }
                                }
                            }
                        }

                        // WINDJAMMER PHILOSOPHY: Auto-convert implicit returns when function returns String
                        // BUT: Don't convert if:
                        // 1. The expression explicitly uses .as_str() (user wants &str)
                        // 2. A sibling branch in an if-else uses .as_str() (type consistency)
                        let returns_string = string_utilities::return_type_expects_owned_string(
                            &self.current_function_return_type,
                        );

                        // Also check if we're in a match arm that needs string conversion
                        let in_match_needing_string = self.in_match_arm_needing_string;

                        // Check if the expression explicitly returns &str via .as_str()
                        let expr_uses_as_str = expr_str.contains(".as_str()");

                        // Check if we should suppress conversion (sibling branch has .as_str())
                        let should_suppress = self.suppress_string_conversion.get();

                        if (returns_string || in_match_needing_string)
                            && !expr_uses_as_str
                            && !should_suppress
                        {
                            // String literal needs .to_string()
                            if matches!(
                                expr,
                                Expression::Literal {
                                    value: Literal::String(_),
                                    ..
                                }
                            ) && !expr_str.ends_with(".to_string()")
                                && expr_str != "String::new()"
                            {
                                expr_str = format!("{}.to_string()", expr_str);
                            }
                            // param.clone() where param: &str → param.to_string()
                            // &str.clone() returns &str, but we need String
                            else if expr_str.ends_with(".clone()") {
                                if let Expression::MethodCall { method, object, .. } = expr {
                                    if method == "clone" {
                                        if let Expression::Identifier { name, .. } = &**object {
                                            // Check if this is a borrowed string parameter
                                            let is_borrowed_str_param =
                                                self.inferred_borrowed_params.contains(name);

                                            if is_borrowed_str_param {
                                                // Replace .clone() with .to_string()
                                                expr_str =
                                                    expr_str.replace(".clone()", ".to_string()");
                                            }
                                        }
                                    }
                                }
                            }
                            // self.field needs .clone() when self is borrowed
                            // BUT: Skip .clone() for Copy types (f32, i32, bool, etc.)
                            else if let Expression::FieldAccess { object, .. } = expr {
                                if let Expression::Identifier { name: obj_name, .. } = &**object {
                                    if obj_name == "self" && !expr_str.ends_with(".clone()") {
                                        let self_is_borrowed =
                                            self.current_function_params.iter().any(|p| {
                                                p.name == "self"
                                                    && matches!(
                                                        p.ownership,
                                                        crate::parser::OwnershipHint::Ref
                                                    )
                                            });
                                        if self_is_borrowed {
                                            let is_copy = self
                                                .infer_expression_type(expr)
                                                .as_ref()
                                                .is_some_and(|t| self.is_type_copy(t));
                                            if !is_copy {
                                                expr_str = format!("{}.clone()", expr_str);
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // DOGFOODING FIX: Vec indexing &vec[idx] for non-Copy needs .clone() when implicit return
                        // Applies to all return types (SaveSlot, Option<String>, etc.), not just String
                        // Use parentheses: (&vec[idx]).clone() - . has higher precedence than &
                        if expr_str.starts_with("&")
                            && !expr_str.starts_with("&mut")
                            && !expr_str.ends_with(".clone()")
                        {
                            let expects_owned = !matches!(
                                &self.current_function_return_type,
                                Some(Type::Reference(_)) | Some(Type::MutableReference(_))
                            );
                            if expects_owned {
                                if let Some(inner) = self.infer_expression_type(expr) {
                                    if !self.is_type_copy(&inner) {
                                        expr_str = format!("({}).clone()", expr_str);
                                    }
                                }
                            }
                        }

                        // FIXED: Auto-cast usize to i64 for implicit returns
                        let returns_int = match &self.current_function_return_type {
                            Some(Type::Int) => true,
                            Some(Type::Custom(name)) if name == "i64" || name == "int" => true,
                            _ => false,
                        };

                        if returns_int && self.expression_produces_usize(expr) {
                            // Implicit return of .len() - auto-cast!
                            expr_str = format!("{} as i64", expr_str);
                        }

                        let returns_option_owned = self.returns_option_owned_type();
                        if returns_option_owned
                            && self.expression_type_contains_reference(expr)
                            && !expr_str.ends_with(".cloned()")
                            && !expr_str.ends_with(".clone()")
                        {
                            if self
                                .infer_expression_type(expr)
                                .as_ref()
                                .is_some_and(Self::type_contains_mut_reference_static)
                            {
                                expr_str = format!("{}.map(|v| v.clone())", expr_str);
                            } else {
                                expr_str = format!("{}.cloned()", expr_str);
                            }
                        }

                        output.push_str(&expr_str);

                        // BUGFIX: Only add semicolon if:
                        // 1. Function returns ()
                        // 2. AND we're not in an expression context (value is not being used)
                        // This prevents adding semicolons to if-else branches when used as values
                        let returns_unit = self.current_function_return_type.is_none()
                            || matches!(self.current_function_return_type, Some(Type::Tuple(ref types)) if types.is_empty());
                        if returns_unit && !self.in_expression_context {
                            output.push(';');
                        }
                        output.push('\n');
                    }
                    Statement::Thread { body, .. } => {
                        // Generate as expression (returns JoinHandle)
                        output.push_str(&self.indent());
                        output.push_str("std::thread::spawn(move || {\n");
                        self.indent_level += 1;
                        for stmt in body {
                            output.push_str(&self.generate_statement(stmt));
                        }
                        self.indent_level -= 1;
                        output.push_str(&self.indent());
                        output.push_str("})\n");
                    }
                    Statement::Async { body, .. } => {
                        // Generate as expression (returns JoinHandle)
                        output.push_str(&self.indent());
                        output.push_str("tokio::spawn(async move {\n");
                        self.indent_level += 1;
                        for stmt in body {
                            output.push_str(&self.generate_statement(stmt));
                        }
                        self.indent_level -= 1;
                        output.push_str(&self.indent());
                        output.push_str("})\n");
                    }
                    Statement::Return { value, .. } => {
                        // TDD FIX: Convert explicit return to implicit return when last statement
                        // Avoids Clippy warning: "unneeded `return` statement"
                        if let Some(expr) = value {
                            output.push_str(&self.indent());
                            let old_coerce_lit = self.coerce_string_literals_to_owned;
                            if string_utilities::return_type_expects_owned_string(
                                &self.current_function_return_type,
                            ) {
                                self.coerce_string_literals_to_owned = true;
                            }
                            let mut expr_str = self.generate_expression(expr);
                            self.coerce_string_literals_to_owned = old_coerce_lit;

                            // TDD FIX: Borrowed iterator vars need deref when returned as Copy types
                            // For `for (_, val) in &vec` where val: &i32, `return val` needs `return *val`
                            if let Expression::Identifier { name, .. } = expr {
                                if self.borrowed_iterator_vars.contains(name) {
                                    let return_type_is_copy = self
                                        .current_function_return_type
                                        .as_ref()
                                        .is_some_and(|t| self.is_type_copy(t));
                                    if return_type_is_copy && !expr_str.starts_with('*') {
                                        expr_str = format!("*{}", expr_str);
                                    }
                                }
                            }

                            // WINDJAMMER PHILOSOPHY: Auto-convert implicit returns when function returns String
                            // Same logic as Statement::Expression implicit returns
                            let returns_string = string_utilities::return_type_expects_owned_string(
                                &self.current_function_return_type,
                            );

                            let in_match_needing_string = self.in_match_arm_needing_string;
                            let expr_uses_as_str = expr_str.contains(".as_str()");
                            let should_suppress = self.suppress_string_conversion.get();

                            if (returns_string || in_match_needing_string)
                                && !expr_uses_as_str
                                && !should_suppress
                            {
                                // String literal needs .to_string()
                                if matches!(
                                    expr,
                                    Expression::Literal {
                                        value: Literal::String(_),
                                        ..
                                    }
                                ) && !expr_str.ends_with(".to_string()")
                                    && expr_str != "String::new()"
                                {
                                    expr_str = format!("{}.to_string()", expr_str);
                                }
                                // param.clone() where param: &str → param.to_string()
                                // &str.clone() returns &str, but we need String
                                // Check if expression is identifier.clone() and identifier is a borrowed string param
                                else if expr_str.ends_with(".clone()") {
                                    if let Expression::MethodCall { method, object, .. } = expr {
                                        if method == "clone" {
                                            if let Expression::Identifier { name, .. } = &**object {
                                                // Check if this is a borrowed string parameter
                                                let is_borrowed_str_param =
                                                    self.inferred_borrowed_params.contains(name);

                                                if is_borrowed_str_param {
                                                    // Replace .clone() with .to_string()
                                                    expr_str = expr_str
                                                        .replace(".clone()", ".to_string()");
                                                }
                                            }
                                        }
                                    }
                                }
                                // self.field needs .clone() when self is borrowed
                                // BUT: Skip .clone() for Copy types (f32, i32, bool, etc.)
                                else if let Expression::FieldAccess { object, .. } = expr {
                                    if let Expression::Identifier { name: obj_name, .. } = &**object
                                    {
                                        if obj_name == "self" && !expr_str.ends_with(".clone()") {
                                            let self_is_borrowed =
                                                self.current_function_params.iter().any(|p| {
                                                    p.name == "self"
                                                        && matches!(
                                                            p.ownership,
                                                            crate::parser::OwnershipHint::Ref
                                                        )
                                                });
                                            if self_is_borrowed {
                                                let is_copy = self
                                                    .infer_expression_type(expr)
                                                    .as_ref()
                                                    .is_some_and(|t| self.is_type_copy(t));
                                                if !is_copy {
                                                    expr_str = format!("{}.clone()", expr_str);
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            // FIXED: Auto-cast usize to i64 for implicit returns
                            // Same logic as Statement::Expression implicit returns
                            let returns_int = match &self.current_function_return_type {
                                Some(Type::Int) => true,
                                Some(Type::Custom(name)) if name == "i64" || name == "int" => true,
                                _ => false,
                            };

                            if returns_int && self.expression_produces_usize(expr) {
                                // Implicit return of .len() - auto-cast!
                                expr_str = format!("{} as i64", expr_str);
                            }

                            let returns_option_owned = self.returns_option_owned_type();
                            if returns_option_owned
                                && self.expression_type_contains_reference(expr)
                                && !expr_str.ends_with(".cloned()")
                                && !expr_str.ends_with(".clone()")
                            {
                                if self
                                    .infer_expression_type(expr)
                                    .as_ref()
                                    .is_some_and(Self::type_contains_mut_reference_static)
                                {
                                    expr_str = format!("{}.map(|v| v.clone())", expr_str);
                                } else {
                                    expr_str = format!("{}.cloned()", expr_str);
                                }
                            }

                            output.push_str(&expr_str);
                            output.push('\n');
                        }
                        // Void return as last statement is omitted (block returns () implicitly)
                    }
                    _ => unreachable!(),
                }
            } else if !is_last {
                // TDD FIX: Non-last statements in a block ALWAYS need semicolons,
                // even when the block is used in an expression context (e.g., match arm body
                // inside `let _ = match ... { Arm => { expr1; expr2 } }`).
                // Temporarily clear in_expression_context so intermediate expression
                // statements get their semicolons.
                let old_expr_ctx = self.in_expression_context;
                self.in_expression_context = false;
                output.push_str(&self.generate_statement(stmt));
                self.in_expression_context = old_expr_ctx;
            } else {
                // Last statement of a non-Expression type (e.g., Statement::If used as block value):
                // Preserve in_expression_context so inner branches retain correct semicolon behavior
                output.push_str(&self.generate_statement(stmt));
            }
        }
        self.current_function_body = saved_body;
        self.current_statement_idx = saved_idx;
        self.current_block_local_idx = saved_local_idx;
        output
    }

    pub(crate) fn generate_statement(&mut self, stmt: &Statement<'ast>) -> String {
        // RECURSION GUARD: Check depth before processing statement
        if let Err(e) = self.enter_recursion("generate_statement") {
            eprintln!("{}", e);
            return format!("/* {} */", e);
        }

        // Record source mapping if location info is available
        if let Some(location) = codegen_helpers::get_statement_location(stmt) {
            self.record_mapping(&location);
        }

        let result = self.generate_statement_impl(stmt);
        self.exit_recursion();
        result
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
    fn branch_tail_suggests_owned_string_coercion(&self, block: &[&'ast Statement<'ast>]) -> bool {
        let Some(last) = block.last().copied() else {
            return false;
        };
        match last {
            Statement::Expression { expr, .. } => self.expr_suggests_owned_string_coercion(expr),
            _ => false,
        }
    }

    fn generate_statement_impl(&mut self, stmt: &Statement<'ast>) -> String {
        match stmt {
            Statement::Let {
                pattern,
                mutable,
                type_,
                value,
                location,
                ..
            } => {
                let mut output = self.indent();
                output.push_str("let ");

                // Check if we need &mut for index access on borrowed fields
                // e.g., let enemy = self.enemies[i] should be let enemy = &mut self.enemies[i]
                let needs_mut_ref = self.should_mut_borrow_index_access(value);

                // Extract variable name for optimizations (only works for simple identifiers)
                let var_name = match pattern {
                    Pattern::Identifier(name) => Some(name.as_str()),
                    _ => None,
                };

                // Mutability: explicit via `let mut`, or auto-inferred when the
                // variable is later used with a &mut self method call.
                let auto_needs_mut = !*mutable
                    && !needs_mut_ref
                    && var_name.is_some_and(|v| self.variable_needs_mut(v));
                if needs_mut_ref {
                    // Don't add mut keyword, but we'll add &mut to the value
                } else if *mutable || auto_needs_mut {
                    output.push_str("mut ");
                }

                // TDD FIX: Prefix unused let bindings with `_` to suppress warnings
                let is_unused_binding = location
                    .as_ref()
                    .is_some_and(|loc| self.unused_let_bindings.contains(&(loc.line, loc.column)));

                // Generate pattern (could be simple name or tuple)
                let pattern_str = if is_unused_binding {
                    match pattern {
                        Pattern::Identifier(name) => format!("_{}", name),
                        other => self.generate_pattern(other),
                    }
                } else {
                    self.generate_pattern(pattern)
                };
                output.push_str(&pattern_str);

                // LOCAL VARIABLE TRACKING: Add this variable to the current scope
                // This enables proper shadowing of field names
                if let Some(name) = var_name {
                    if let Some(current_scope) = self.local_variable_scopes.last_mut() {
                        current_scope.insert(name.to_string());
                    }
                } else if matches!(pattern, Pattern::Tuple(_)) {
                    let mut bound = std::collections::HashSet::new();
                    self.extract_pattern_bindings(pattern, &mut bound);
                    if let Some(current_scope) = self.local_variable_scopes.last_mut() {
                        for n in bound {
                            current_scope.insert(n);
                        }
                    }
                }

                // LOCAL VARIABLE TYPE TRACKING: Infer type from value expression or annotation
                // This enables qualified method signature lookup (e.g., stack.remove() → Stack::remove)
                if let Some(name) = var_name {
                    let inferred_type: Option<Type> = if let Some(type_) = type_ {
                        // Explicit type annotation: let x: Foo = ...
                        Some((*type_).clone())
                    } else {
                        // Infer from value expression
                        match value {
                            Expression::StructLiteral {
                                name: struct_name, ..
                            } => Some(Type::Custom(struct_name.to_string())),
                            // Literal types: let x = 25 → i32, let y = 3.14 → f32, let b = true → bool
                            Expression::Literal {
                                value: crate::parser::Literal::Int(_),
                                ..
                            } => Some(Type::Int),
                            Expression::Literal {
                                value: crate::parser::Literal::Float(_),
                                ..
                            } => Some(Type::Float),
                            Expression::Literal {
                                value: crate::parser::Literal::Bool(_),
                                ..
                            } => Some(Type::Bool),
                            Expression::Literal {
                                value: crate::parser::Literal::String(_),
                                ..
                            } => Some(Type::String),
                            Expression::Call { function, .. } => {
                                // Type::method() pattern (e.g., Foo::new())
                                if let Expression::FieldAccess { object, field, .. } = *function {
                                    if let Expression::Identifier {
                                        name: type_name, ..
                                    } = *object
                                    {
                                        if type_name
                                            .chars()
                                            .next()
                                            .is_some_and(|c| c.is_uppercase())
                                            && (field == "new"
                                                || field.starts_with("from_")
                                                || field.starts_with("with_")
                                                || field == "default")
                                        {
                                            // E0282: For Vec::new() / HashSet::new(), infer element type.
                                            // Priority: 1) return type  2) forward-scan .push()/.insert()
                                            if (type_name == "Vec" || type_name == "HashSet")
                                                && field == "new"
                                            {
                                                let elem_from_return =
                                                    match &self.current_function_return_type {
                                                        Some(Type::Vec(inner))
                                                            if type_name == "Vec" =>
                                                        {
                                                            Some(inner.as_ref().clone())
                                                        }
                                                        Some(Type::Parameterized(base, args))
                                                            if base == type_name
                                                                && !args.is_empty() =>
                                                        {
                                                            Some(args[0].clone())
                                                        }
                                                        _ => None,
                                                    };
                                                let elem_from_push = if elem_from_return.is_none() {
                                                    var_name.and_then(|vn| {
                                                            self.infer_collection_element_type_from_usage(vn)
                                                        })
                                                } else {
                                                    None
                                                };
                                                let elem_type = elem_from_return.or(elem_from_push);
                                                if let Some(inner) = elem_type {
                                                    if type_name == "Vec" {
                                                        Some(Type::Vec(Box::new(inner)))
                                                    } else {
                                                        Some(Type::Parameterized(
                                                            type_name.to_string(),
                                                            vec![inner],
                                                        ))
                                                    }
                                                } else {
                                                    Some(Type::Custom(type_name.to_string()))
                                                }
                                            } else {
                                                Some(Type::Custom(type_name.to_string()))
                                            }
                                        } else {
                                            // Not a constructor — look up return type from signature registry
                                            // e.g., MathHelper::fade(x) → return type is f32
                                            let qualified = format!("{}::{}", type_name, field);
                                            self.signature_registry
                                                .get_signature(&qualified)
                                                .and_then(|sig| sig.return_type.clone())
                                        }
                                    } else {
                                        None
                                    }
                                } else if let Expression::Identifier { name: fn_name, .. } =
                                    *function
                                {
                                    // Handle Identifier("Vec::new") path (parser emits this form)
                                    if fn_name == "Vec::new" || fn_name == "HashSet::new" {
                                        let collection_name = if fn_name.starts_with("Vec") {
                                            "Vec"
                                        } else {
                                            "HashSet"
                                        };
                                        let elem_from_return = match &self
                                            .current_function_return_type
                                        {
                                            Some(Type::Vec(inner)) if collection_name == "Vec" => {
                                                Some(inner.as_ref().clone())
                                            }
                                            Some(Type::Parameterized(base, args))
                                                if base == collection_name && !args.is_empty() =>
                                            {
                                                Some(args[0].clone())
                                            }
                                            _ => None,
                                        };
                                        let elem_from_push = if elem_from_return.is_none() {
                                            var_name.and_then(|vn| {
                                                self.infer_collection_element_type_from_usage(vn)
                                            })
                                        } else {
                                            None
                                        };
                                        let elem_type = elem_from_return.or(elem_from_push);
                                        if let Some(inner) = elem_type {
                                            if collection_name == "Vec" {
                                                Some(Type::Vec(Box::new(inner)))
                                            } else {
                                                Some(Type::Parameterized(
                                                    collection_name.to_string(),
                                                    vec![inner],
                                                ))
                                            }
                                        } else {
                                            Some(Type::Custom(collection_name.to_string()))
                                        }
                                    } else {
                                        self.infer_expression_type(value)
                                    }
                                } else {
                                    // Simple function call: look up in signature registry
                                    self.infer_expression_type(value)
                                }
                            }
                            _ => {
                                // Fall back to general expression type inference
                                // Handles if/else, binary ops, method calls, etc.
                                self.infer_expression_type(value)
                            }
                        }
                    };
                    if let Some(t) = inferred_type {
                        self.local_var_types.insert(name.to_string(), t);
                    }
                }

                // PHASE 8: Check if this variable should use SmallVec
                if let Some(name) = var_name {
                    if let Some(smallvec_opt) = self.smallvec_optimizations.get(name) {
                        // Use SmallVec with stack allocation
                        // If there's a type annotation, extract the element type
                        let elem_type = if let Some(Type::Vec(inner)) = type_ {
                            self.type_to_rust(inner)
                        } else {
                            "_".to_string() // Type inference
                        };
                        output.push_str(&format!(
                            ": SmallVec<[{}; {}]>",
                            elem_type, smallvec_opt.stack_size
                        ));
                        output.push_str(" = ");

                        // Generate the expression but wrap in smallvec! if it's a vec! macro
                        let expr_str = self.generate_expression(value);
                        if let Some(stripped) = expr_str.strip_prefix("vec!") {
                            // Replace vec! with smallvec!
                            output.push_str("smallvec!");
                            output.push_str(stripped);
                        } else {
                            // For other expressions, try to convert
                            output.push_str(&expr_str);
                            output.push_str(".into()"); // Convert Vec to SmallVec
                        }
                    } else if let Some(t) = type_ {
                        output.push_str(": ");
                        output.push_str(&self.type_to_rust(t));
                        output.push_str(" = ");

                        let is_string_type = matches!(t, Type::String)
                            || matches!(t, Type::Custom(name) if name == "String" || name == "string");

                        let old_coerce_lit = self.coerce_string_literals_to_owned;
                        if is_string_type {
                            self.coerce_string_literals_to_owned = true;
                        }
                        // Same as other `let` RHS paths: value is used (e.g. `let x: f32 = if ...`).
                        // Without this, if/else branch bodies get `expr;` and infer `()` (E0308).
                        let old_ctx = self.in_expression_context;
                        self.in_expression_context = true;

                        let prev_assign_float = self.assignment_float_target_type.take();
                        if Self::assignment_target_needs_float_codegen_context(t) {
                            self.assignment_float_target_type = Some(t.clone());
                        }
                        let prev_suppress_turbo = self.suppress_collection_turbofish;
                        let suppress_turbofish_here = matches!(t, Type::Vec(_))
                            || matches!(
                                t,
                                Type::Parameterized(base, _)
                                    if base == "HashSet" || base == "HashMap"
                            );
                        if suppress_turbofish_here {
                            self.suppress_collection_turbofish = true;
                        }

                        let prev_collect_target = self.collect_target_type.take();
                        self.collect_target_type = Some(t.clone());

                        // Auto-convert &str to String if type is String
                        let mut value_str = self.generate_expression(value);

                        self.collect_target_type = prev_collect_target;
                        self.suppress_collection_turbofish = prev_suppress_turbo;
                        self.assignment_float_target_type = prev_assign_float;

                        self.in_expression_context = old_ctx;
                        self.coerce_string_literals_to_owned = old_coerce_lit;
                        self.apply_vec_index_let_rhs_fixup(
                            var_name,
                            value,
                            Some(t),
                            &mut value_str,
                        );

                        // Convert string literals OR identifiers to String when target is String
                        if is_string_type && value_str != "String::new()" {
                            let should_convert =
                                matches!(
                                    value,
                                    Expression::Literal {
                                        value: Literal::String(s),
                                        ..
                                    } if !s.is_empty()
                                ) || matches!(value, Expression::Identifier { .. });
                            if should_convert && !value_str.ends_with(".to_string()") {
                                value_str = format!("{}.to_string()", value_str);
                            }
                            if let Expression::Literal {
                                value: Literal::String(s),
                                ..
                            } = value
                            {
                                if s.is_empty() {
                                    value_str = "String::new()".to_string();
                                }
                            }
                        }
                        output.push_str(&value_str);
                    } else {
                        // E0282: Emit type ascription for collection types.
                        // Skip when the value's type is better inferred by Rust:
                        // - Method calls may return a different type than the receiver
                        //   (e.g., Vec::into_iter() → IntoIter, not Vec)
                        // - Macro invocations (e.g., vec![1,2,3]) produce values whose
                        //   element type should be inferred from usage context, not from
                        //   Windjammer's default numeric types
                        let type_inferred_from_context = matches!(
                            value,
                            Expression::MethodCall { .. } | Expression::MacroInvocation { .. }
                        );
                        let needs_collection_ascription_sv = !type_inferred_from_context
                            && var_name.is_some_and(|vn| {
                                matches!(
                                    self.local_var_types.get(vn),
                                    Some(Type::Vec(_)) | Some(Type::Parameterized(_, _))
                                )
                            });
                        if needs_collection_ascription_sv {
                            let vn = var_name.unwrap();
                            let ty = self.local_var_types.get(vn).unwrap().clone();
                            output.push_str(": ");
                            output.push_str(&self.type_to_rust(&ty));
                        }
                        output.push_str(" = ");
                        if needs_mut_ref {
                            output.push_str("&mut ");
                        }

                        // EXPRESSION CONTEXT: Mark that we're generating a value that will be used
                        // This prevents adding semicolons to if-else branches when used in let bindings
                        let old_ctx = self.in_expression_context;
                        self.in_expression_context = true;

                        let old_suppress = self.suppress_collection_turbofish;
                        if needs_collection_ascription_sv {
                            self.suppress_collection_turbofish = true;
                        }

                        // WINDJAMMER PHILOSOPHY: Auto-convert string literals to String
                        // String literals assigned to variables should become String (not &str)
                        // because they may be passed to functions expecting String later.
                        // This is safe because String auto-borrows to &str when needed.
                        let mut value_str = self.generate_expression(value);

                        self.apply_vec_index_let_rhs_fixup(var_name, value, None, &mut value_str);
                        if let Expression::Literal {
                            value: Literal::String(s),
                            ..
                        } = value
                        {
                            if s.is_empty() {
                                value_str = "String::new()".to_string();
                            } else if !value_str.ends_with(".to_string()") {
                                value_str = format!("{}.to_string()", value_str);
                            }
                        }

                        // E0507: `let x = self.field` through `&self`/`&mut self`:
                        //   Option<T> behind &mut self → .take() (moves value, leaves None)
                        //   other non-Copy → .clone()
                        self.apply_self_field_move_fix(value, &mut value_str);

                        value_str = self.let_rhs_clone_if_mut_from_non_copy_ref(
                            *mutable,
                            value,
                            needs_mut_ref,
                            &value_str,
                        );

                        output.push_str(&value_str);

                        // Restore expression context
                        self.in_expression_context = old_ctx;
                        self.suppress_collection_turbofish = old_suppress;
                    }
                } else {
                    // No SmallVec optimization for this variable
                    if let Some(t) = type_ {
                        output.push_str(": ");
                        output.push_str(&self.type_to_rust(t));
                        output.push_str(" = ");

                        // EXPRESSION CONTEXT: Mark that we're generating a value
                        let old_ctx = self.in_expression_context;
                        self.in_expression_context = true;

                        let prev_assign_float = self.assignment_float_target_type.take();
                        if Self::assignment_target_needs_float_codegen_context(t) {
                            self.assignment_float_target_type = Some(t.clone());
                        }
                        let prev_suppress_turbo = self.suppress_collection_turbofish;
                        let suppress_turbofish_here = matches!(t, Type::Vec(_))
                            || matches!(
                                t,
                                Type::Parameterized(base, _) if base == "HashSet" || base == "HashMap"
                            );
                        if suppress_turbofish_here {
                            self.suppress_collection_turbofish = true;
                        }

                        let prev_collect_target = self.collect_target_type.take();
                        self.collect_target_type = Some(t.clone());

                        // Auto-convert &str to String if type is String
                        let mut value_str = self.generate_expression(value);

                        self.collect_target_type = prev_collect_target;
                        self.suppress_collection_turbofish = prev_suppress_turbo;
                        self.assignment_float_target_type = prev_assign_float;

                        self.apply_vec_index_let_rhs_fixup(
                            var_name,
                            value,
                            Some(t),
                            &mut value_str,
                        );
                        let is_string_type = matches!(t, Type::String)
                            || matches!(t, Type::Custom(name) if name == "String");

                        // Convert string literals OR identifiers to String when target is String
                        if is_string_type && value_str != "String::new()" {
                            if let Expression::Literal {
                                value: Literal::String(s),
                                ..
                            } = value
                            {
                                if s.is_empty() {
                                    value_str = "String::new()".to_string();
                                } else {
                                    value_str = format!("{}.to_string()", value_str);
                                }
                            } else if matches!(value, Expression::Identifier { .. }) {
                                value_str = format!("{}.to_string()", value_str);
                            }
                        }

                        if needs_mut_ref {
                            value_str = format!("&mut {}", value_str);
                        }
                        output.push_str(&value_str);

                        // Restore expression context
                        self.in_expression_context = old_ctx;
                    } else {
                        // E0282: Emit type ascription for collection types inferred from
                        // forward-scanned .push()/.insert() usage
                        let needs_collection_ascription = var_name.is_some_and(|vn| {
                            matches!(
                                self.local_var_types.get(vn),
                                Some(Type::Vec(_)) | Some(Type::Parameterized(_, _))
                            )
                        });
                        if needs_collection_ascription {
                            let vn = var_name.unwrap();
                            let ty = self.local_var_types.get(vn).unwrap().clone();
                            output.push_str(": ");
                            output.push_str(&self.type_to_rust(&ty));
                        }
                        output.push_str(" = ");
                        if needs_mut_ref {
                            output.push_str("&mut ");
                        }

                        // EXPRESSION CONTEXT: Mark that we're generating a value
                        let old_ctx = self.in_expression_context;
                        self.in_expression_context = true;

                        let old_suppress = self.suppress_collection_turbofish;
                        if needs_collection_ascription {
                            self.suppress_collection_turbofish = true;
                        }

                        // WINDJAMMER PHILOSOPHY: Auto-convert mutable string variables
                        // When a mutable variable is initialized with a string literal,
                        // it should be a String (not &str) because &str can't be mutated
                        let mut value_str = self.generate_expression(value);
                        self.apply_vec_index_let_rhs_fixup(var_name, value, None, &mut value_str);
                        if *mutable
                            && matches!(
                                value,
                                Expression::Literal {
                                    value: Literal::String(_),
                                    ..
                                }
                            )
                        {
                            value_str = format!("{}.to_string()", value_str);
                        }

                        // E0507: `let x = self.field` through `&self`/`&mut self`:
                        //   Option<T> behind &mut self → .take() (moves value, leaves None)
                        //   other non-Copy → .clone()
                        self.apply_self_field_move_fix(value, &mut value_str);

                        value_str = self.let_rhs_clone_if_mut_from_non_copy_ref(
                            *mutable,
                            value,
                            needs_mut_ref,
                            &value_str,
                        );

                        output.push_str(&value_str);

                        // Restore expression context
                        self.in_expression_context = old_ctx;
                        self.suppress_collection_turbofish = old_suppress;
                    }
                }

                self.register_tuple_let_binding_types(pattern, value);

                output.push_str(";\n");

                // Track variables assigned from .len() as usize type
                // OR variables with explicit usize type annotation
                // This enables auto-casting in comparisons with i32
                if let Some(name) = var_name {
                    let is_usize = self.expression_produces_usize(value)
                        || matches!(type_, Some(Type::Custom(s)) if s == "usize");
                    if is_usize {
                        self.usize_variables.insert(name.to_string());
                    }
                }

                output
            }
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
            Statement::Return { value: expr, .. } => {
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
                        // String literal needs .to_string()
                        if matches!(
                            e,
                            Expression::Literal {
                                value: Literal::String(_),
                                ..
                            }
                        ) && !return_str.ends_with(".to_string()")
                            && return_str != "String::new()"
                        {
                            return_str = format!("{}.to_string()", return_str);
                        }
                        // param.clone() where param: &str → param.to_string()
                        // &str.clone() returns &str, but we need String
                        else if let Expression::MethodCall { method, object, .. } = e {
                            if method == "clone" {
                                if let Expression::Identifier { name, .. } = &**object {
                                    // Check if this identifier is a borrowed string parameter
                                    let is_string_type = self.current_function_params.iter().any(|p| {
                                        p.name == *name && (
                                            matches!(p.type_, Type::String)
                                            || matches!(p.type_, Type::Custom(ref n) if n == "string")
                                        )
                                    });
                                    let is_borrowed_str_param =
                                        self.inferred_borrowed_params.contains(name)
                                            && is_string_type;

                                    if is_borrowed_str_param {
                                        // Replace .clone() with .to_string()
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
                                    let self_is_borrowed =
                                        self.current_function_params.iter().any(|p| {
                                            p.name == "self"
                                                && matches!(
                                                    p.ownership,
                                                    crate::parser::OwnershipHint::Ref
                                                )
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
                    if return_str.starts_with("&")
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
                                if self.is_type_copy(inner.as_ref()) && !return_str.starts_with('*')
                                {
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
            } => {
                // WINDJAMMER PHILOSOPHY: Check if any branch explicitly uses .as_str()
                // If so, we should NOT auto-convert string literals in other branches
                let any_branch_has_as_str = string_analysis::block_has_as_str(then_block)
                    || else_block
                        .as_ref()
                        .is_some_and(|b| string_analysis::block_has_as_str(b));

                let old_suppress = self.suppress_string_conversion.get();
                if any_branch_has_as_str {
                    self.suppress_string_conversion.set(true);
                }

                let mut output = self.indent();
                output.push_str("if ");
                let cond_str = self.generate_expression(condition);
                // Auto-deref borrowed bool in if-condition: `if r` where r: &bool → `if *r`
                let cond_str = if let Expression::Identifier { name, .. } = condition {
                    if self.inferred_borrowed_params.contains(name.as_str())
                        || self.borrowed_iterator_vars.contains(name)
                    {
                        let is_bool_ref = self
                            .infer_expression_type(condition)
                            .as_ref()
                            .is_some_and(|t| {
                                matches!(t,
                                    Type::Reference(inner) | Type::MutableReference(inner)
                                    if matches!(&**inner, Type::Bool)
                                )
                            });
                        if is_bool_ref && !cond_str.starts_with('*') {
                            format!("*{}", cond_str)
                        } else {
                            cond_str
                        }
                    } else {
                        cond_str
                    }
                } else {
                    cond_str
                };
                output.push_str(&cond_str);
                output.push_str(" {\n");

                // DOGFOODING FIX: Preserve explicit returns in if-without-else
                // In Rust, `if` without `else` must evaluate to `()`, so any value expression
                // (including implicit returns) is invalid: E0308 "if without else has incompatible types"
                //
                // Safe to optimize returns ONLY in if-else (both branches have values/returns)
                // Must preserve returns in if-without-else (then block evaluates to ())
                let old_in_func_body = self.in_function_body;
                let old_in_void_block = self.in_void_block;
                if else_block.is_none() || !self.current_is_last_statement {
                    self.in_function_body = false;
                }
                // if-without-else must evaluate to (); suppress implicit returns
                if else_block.is_none() {
                    self.in_void_block = true;
                }

                let old_coerce_lit = self.coerce_string_literals_to_owned;
                let any_branch_suggests_owned_coercion = self
                    .branch_tail_suggests_owned_string_coercion(then_block)
                    || else_block
                        .as_ref()
                        .is_some_and(|eb| self.branch_tail_suggests_owned_string_coercion(eb));
                // Coerce string literals in branches when:
                // - The enclosing function returns owned String (even if this `if` is not the last
                //   statement — otherwise `in_function_body` is cleared and inner blocks skip coercion), or
                // - We're in an expression context (`let`/`=` RHS, etc.) and a branch yields String
                //   (e.g. `parts[0].clone()` vs `"0"` while the function itself returns `()`).
                let coerce_string_in_branches = else_block.is_some()
                    && (string_utilities::return_type_expects_owned_string(&self.current_function_return_type)
                        || (self.in_expression_context && any_branch_suggests_owned_coercion));
                if coerce_string_in_branches {
                    self.coerce_string_literals_to_owned = true;
                }

                self.indent_level += 1;
                output.push_str(&self.generate_block(then_block));
                self.indent_level -= 1;
                self.in_void_block = old_in_void_block;

                output.push_str(&self.indent());
                output.push('}');

                if let Some(else_b) = else_block {
                    output.push_str(" else {\n");
                    self.indent_level += 1;
                    if coerce_string_in_branches {
                        self.coerce_string_literals_to_owned = true;
                    }
                    output.push_str(&self.generate_block(else_b));
                    self.indent_level -= 1;
                    output.push_str(&self.indent());
                    output.push('}');
                }

                self.coerce_string_literals_to_owned = old_coerce_lit;

                self.in_function_body = old_in_func_body;

                self.suppress_string_conversion.set(old_suppress);
                output.push('\n');
                output
            }
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

    fn generate_match_statement(
        &mut self,
        value: &Expression<'ast>,
        arms: &[crate::parser::MatchArm<'ast>],
    ) -> String {
        use super::arm_string_analysis;

        // TDD FIX: Optimize boolean match expressions to matches! macro
        if arms.len() == 2 && arms[0].guard.is_none() && arms[1].guard.is_none() {
            let arm0_is_true = matches!(
                arms[0].body,
                Expression::Literal {
                    value: Literal::Bool(true),
                    ..
                }
            );
            let arm0_is_false = matches!(
                arms[0].body,
                Expression::Literal {
                    value: Literal::Bool(false),
                    ..
                }
            );
            let arm1_is_true = matches!(
                arms[1].body,
                Expression::Literal {
                    value: Literal::Bool(true),
                    ..
                }
            );
            let arm1_is_false = matches!(
                arms[1].body,
                Expression::Literal {
                    value: Literal::Bool(false),
                    ..
                }
            );

            if arm0_is_true && arm1_is_false {
                let value_str = self.generate_expression(value);
                let pattern_str = self.generate_pattern(&arms[0].pattern);
                let mut output = self.indent();
                output.push_str(&format!("matches!({}, {})\n", value_str, pattern_str));
                return output;
            }

            if arm0_is_false && arm1_is_true {
                let value_str = self.generate_expression(value);
                let pattern_str = self.generate_pattern(&arms[0].pattern);
                let mut output = self.indent();
                output.push_str(&format!("!matches!({}, {})\n", value_str, pattern_str));
                return output;
            }
        }

        // TDD FIX: Detect `if let` pattern and generate `if let` instead of `match`
        // Guards require full `match` syntax (if-let doesn't support guards in Rust)
        if arms.len() == 2
            && matches!(arms[1].pattern, Pattern::Wildcard)
            && arms[1].guard.is_none()
            && arms[0].guard.is_none()
        {
            let wildcard_body_is_empty = if let Expression::Block { statements, .. } = arms[1].body
            {
                statements.is_empty()
            } else {
                false
            };

            let wildcard_body_stmts: Option<&[&Statement]> =
                if let Expression::Block { statements, .. } = arms[1].body {
                    if statements.is_empty() {
                        None
                    } else {
                        Some(statements)
                    }
                } else {
                    None
                };

            let match_binds_refs_early_check = self.match_expression_binds_refs(value)
                || self.expression_type_contains_reference(value);
            let needs_borrow_break_check = match_binds_refs_early_check
                && self.match_scrutinee_is_self_method_call(value)
                && self.match_arms_mutate_self(arms);

            if !needs_borrow_break_check
                && (wildcard_body_is_empty || wildcard_body_stmts.is_some())
            {
                let value_str = if let Expression::MethodCall {
                    object,
                    method,
                    arguments,
                    ..
                } = value
                {
                    if method == "as_str"
                        && arguments.is_empty()
                        && self.expression_produces_str_ref(object)
                    {
                        self.generate_expression(object)
                    } else {
                        self.generate_expression(value)
                    }
                } else {
                    self.generate_expression(value)
                };
                // E0507 fix: if let Some(x) / EnumVar(x) = borrowed.field must use & / &mut
                let scrutinee_ref_prefix = if matches!(
                    &arms[0].pattern,
                    Pattern::EnumVariant(_, binding)
                        if !matches!(binding, crate::parser::EnumPatternBinding::None)
                ) {
                    let is_some = matches!(
                        &arms[0].pattern,
                        Pattern::EnumVariant(name, _) if name == "Some" || name.ends_with("::Some")
                    );
                    if is_some {
                        self.effective_option_scrutinee_ref_prefix(value, Some(&arms[0]))
                    } else {
                        self.option_scrutinee_ref_prefix(value)
                    }
                } else {
                    ""
                };
                let value_str = if scrutinee_ref_prefix.is_empty() {
                    value_str
                } else if scrutinee_ref_prefix == "&mut "
                    && value_str.starts_with('&')
                    && !value_str.starts_with("&mut")
                {
                    format!("&mut {}", &value_str[1..])
                } else {
                    let base = if value_str.ends_with(".clone()") {
                        value_str[..value_str.len() - 8].to_string()
                    } else {
                        value_str
                    };
                    format!("{}{}", scrutinee_ref_prefix, base)
                };
                let main_arm = &arms[0];

                let mut bound_vars = std::collections::HashSet::new();
                self.extract_pattern_bindings(&main_arm.pattern, &mut bound_vars);

                let added_borrowed: Vec<String> = if match_binds_refs_early_check {
                    bound_vars.iter().cloned().collect()
                } else {
                    Vec::new()
                };
                for var in &added_borrowed {
                    self.borrowed_iterator_vars.insert(var.clone());
                }

                self.local_variable_scopes.push(bound_vars);

                let mut match_bound_type_entries: Vec<(String, Type)> =
                    self.infer_match_bound_types(value, &main_arm.pattern);
                // The codegen prepends & or &mut to the scrutinee, but
                // `infer_match_bound_types` only sees the AST expression
                // (without the ref prefix).  Wrap the inferred binding types
                // so downstream `let x = binding` can trigger `.clone()`.
                if scrutinee_ref_prefix == "&mut " {
                    for entry in &mut match_bound_type_entries {
                        if !matches!(entry.1, Type::Reference(_) | Type::MutableReference(_)) {
                            entry.1 = Type::MutableReference(Box::new(entry.1.clone()));
                        }
                    }
                } else if scrutinee_ref_prefix == "& " || scrutinee_ref_prefix == "&" {
                    for entry in &mut match_bound_type_entries {
                        if !matches!(entry.1, Type::Reference(_) | Type::MutableReference(_)) {
                            entry.1 = Type::Reference(Box::new(entry.1.clone()));
                        }
                    }
                }
                for (var_name, var_type) in &match_bound_type_entries {
                    self.local_var_types
                        .insert(var_name.clone(), var_type.clone());
                }

                // Upgrade pattern bindings to `mut` when the body mutates them.
                // Only use ref mut when the scrutinee is &mut (mutable borrow).
                // When &self (immutable), ref mut is invalid.
                let scrutinee_is_mut_ref = scrutinee_ref_prefix.contains("mut");
                let upgraded_pattern = if let Expression::Block { statements, .. } = main_arm.body {
                    self.upgrade_pattern_mut_bindings(
                        &main_arm.pattern,
                        statements.as_slice(),
                        scrutinee_is_mut_ref,
                    )
                } else {
                    main_arm.pattern.clone()
                };

                let mut output = self.indent();
                output.push_str("if let ");
                output.push_str(&self.generate_pattern(&upgraded_pattern));

                if let Some(guard) = &main_arm.guard {
                    output.push_str(" if ");
                    output.push_str(&self.generate_expression(guard));
                }

                output.push_str(" = ");
                output.push_str(&value_str);
                output.push_str(" {\n");

                let has_else = wildcard_body_stmts.is_some();
                let old_in_func_body = self.in_function_body;
                let old_in_void_block = self.in_void_block;
                if !has_else {
                    self.in_function_body = false;
                    self.in_void_block = true;
                }

                self.indent_level += 1;
                if let Expression::Block { statements, .. } = main_arm.body {
                    // Check the last statement for simple binding return needing deref
                    if match_binds_refs_early_check {
                        if let Some(last_stmt) = statements.last() {
                            if let crate::parser::Statement::Expression { expr, .. } = last_stmt {
                                if let Expression::Identifier { name, .. } = expr {
                                    if added_borrowed.contains(name) {
                                        let binding_type = match_bound_type_entries
                                            .iter()
                                            .find(|(n, _)| n == name)
                                            .map(|(_, t)| t);
                                        let is_copy =
                                            binding_type.is_some_and(|t| self.is_type_copy(t));
                                        // Generate all but last, then the derefed last
                                        let all_but_last = &statements[..statements.len() - 1];
                                        output.push_str(&self.generate_block(all_but_last));
                                        output.push_str(&self.indent());
                                        let expr_str = self.generate_expression(expr);
                                        if is_copy {
                                            output.push_str(&format!("*{}\n", expr_str));
                                        } else {
                                            output.push_str(&format!("{}.clone()\n", expr_str));
                                        }
                                        self.indent_level -= 1;
                                        self.in_void_block = old_in_void_block;

                                        output.push_str(&self.indent());
                                        output.push('}');

                                        if let Some(else_stmts) = wildcard_body_stmts {
                                            output.push_str(" else {\n");
                                            self.indent_level += 1;
                                            output.push_str(&self.generate_block(else_stmts));
                                            self.indent_level -= 1;
                                            output.push_str(&self.indent());
                                            output.push('}');
                                        }
                                        self.in_function_body = old_in_func_body;
                                        for var in &added_borrowed {
                                            self.borrowed_iterator_vars.remove(var);
                                        }
                                        self.local_variable_scopes.pop();
                                        for (var_name, _) in &match_bound_type_entries {
                                            self.local_var_types.remove(var_name);
                                        }
                                        return output;
                                    }
                                }
                            }
                        }
                    }
                    output.push_str(&self.generate_block(statements));
                } else {
                    // Simple expression body — check for deref
                    let mut body_str = self.generate_expression(main_arm.body);
                    if match_binds_refs_early_check {
                        if let Expression::Identifier { name, .. } = main_arm.body {
                            if added_borrowed.contains(name) {
                                let binding_type = match_bound_type_entries
                                    .iter()
                                    .find(|(n, _)| n == name)
                                    .map(|(_, t)| t);
                                let is_copy = binding_type.is_some_and(|t| self.is_type_copy(t));
                                if is_copy {
                                    body_str = format!("*{}", body_str);
                                } else {
                                    body_str = format!("{}.clone()", body_str);
                                }
                            }
                        }
                    }
                    output.push_str(&self.indent());
                    output.push_str(&body_str);
                    output.push_str(";\n");
                }
                self.indent_level -= 1;
                self.in_void_block = old_in_void_block;

                output.push_str(&self.indent());
                output.push('}');

                if let Some(else_stmts) = wildcard_body_stmts {
                    output.push_str(" else {\n");
                    self.indent_level += 1;
                    output.push_str(&self.generate_block(else_stmts));
                    self.indent_level -= 1;
                    output.push_str(&self.indent());
                    output.push('}');
                }

                self.in_function_body = old_in_func_body;

                output.push('\n');

                self.local_variable_scopes.pop();
                for (var_name, _) in &match_bound_type_entries {
                    self.local_var_types.remove(var_name);
                }
                for var in &added_borrowed {
                    self.borrowed_iterator_vars.remove(var);
                }

                return output;
            }
        }

        let has_string_literal = arms
            .iter()
            .any(|arm| pattern_analysis::pattern_has_string_literal(&arm.pattern));

        let is_tuple_match = arms
            .iter()
            .any(|arm| matches!(arm.pattern, Pattern::Tuple(_)));

        let value_str = if let Expression::MethodCall {
            object,
            method,
            arguments,
            ..
        } = value
        {
            if method == "as_str"
                && arguments.is_empty()
                && self.expression_produces_str_ref(object)
            {
                self.generate_expression(object)
            } else {
                self.generate_expression(value)
            }
        } else if let Expression::Call {
            function,
            arguments,
            ..
        } = value
        {
            if let Expression::FieldAccess { object, field, .. } = &**function {
                if field == "as_str"
                    && arguments.is_empty()
                    && self.expression_produces_str_ref(object)
                {
                    self.generate_expression(object)
                } else {
                    self.generate_expression(value)
                }
            } else {
                self.generate_expression(value)
            }
        } else {
            self.generate_expression(value)
        };

        // E0507 fix: match on Option behind a borrow needs & / &mut scrutinee
        let some_arm = arms.iter().find(|arm| {
            matches!(&arm.pattern, Pattern::EnumVariant(name, _) if name == "Some" || name.ends_with("::Some"))
        });
        let match_scrutinee_ref_prefix: &str;
        let value_str = if let Some(arm) = some_arm {
            let p = self.effective_option_scrutinee_ref_prefix(value, Some(arm));
            match_scrutinee_ref_prefix = p;
            if p.is_empty() {
                value_str
            } else if p == "&mut " && value_str.starts_with('&') && !value_str.starts_with("&mut") {
                format!("&mut {}", &value_str[1..])
            } else {
                let base = if value_str.ends_with(".clone()") {
                    value_str[..value_str.len() - 8].to_string()
                } else {
                    value_str
                };
                format!("{}{}", p, base)
            }
        } else {
            match_scrutinee_ref_prefix = "";
            value_str
        };

        // E0507 fix (generalized): non-Option enum patterns with bindings
        // behind a borrowed parameter also need & prefix.
        let value_str = if some_arm.is_none() {
            let has_non_option_binding = arms.iter().any(|arm| {
                matches!(
                    &arm.pattern,
                    Pattern::EnumVariant(name, binding)
                        if !matches!(binding, crate::parser::EnumPatternBinding::None)
                           && name != "Some" && !name.ends_with("::Some")
                           && name != "None" && !name.ends_with("::None")
                )
            });
            if has_non_option_binding {
                let root = self.root_identifier_of_field_or_index_chain(value);
                if let Some(root_name) = root {
                    let already_owned = value_str.ends_with(".clone()");
                    if already_owned {
                        value_str
                    } else if self.inferred_mut_borrowed_params.contains(root_name) {
                        format!("&mut {}", value_str)
                    } else if self.inferred_borrowed_params.contains(root_name) {
                        // Check if the underlying value type (not the reference) is Copy.
                        // For `e: &E` the expression type is `&E` (Copy since refs are Copy),
                        // but we need to know if `E` itself is Copy to decide the prefix:
                        //   - Copy inner type: use `*e` (deref to get owned Copy value)
                        //   - Non-Copy inner type: use `e` (let match ergonomics handle it)
                        let inner_type_is_copy = if root_name == "self" {
                            self.current_struct_name
                                .as_ref()
                                .is_some_and(|sn| self.is_type_copy(&Type::Custom(sn.clone())))
                        } else {
                            self.infer_expression_type(value)
                                .map(|t| match &t {
                                    Type::Reference(inner) | Type::MutableReference(inner) => {
                                        self.is_type_copy(inner)
                                    }
                                    _ => self.is_type_copy(&t),
                                })
                                .unwrap_or(false)
                        };
                        if inner_type_is_copy {
                            if matches!(value, Expression::FieldAccess { .. }) {
                                value_str
                            } else {
                                format!("*{}", value_str)
                            }
                        } else {
                            // Non-Copy type behind shared ref: need & prefix to prevent
                            // moving out of the borrow. Match ergonomics will auto-ref bindings.
                            format!("&{}", value_str)
                        }
                    } else {
                        value_str
                    }
                } else {
                    value_str
                }
            } else {
                value_str
            }
        } else {
            value_str
        };

        let match_binds_refs_early = self.match_expression_binds_refs(value);
        let needs_borrow_break = match_binds_refs_early
            && self.match_scrutinee_is_self_method_call(value)
            && self.match_arms_mutate_self(arms);

        let mut output = self.indent();

        if needs_borrow_break {
            output.push_str(&format!(
                "let __match_borrow_break = {}.map(|__v| __v.to_owned());\n",
                value_str
            ));
            output.push_str(&self.indent());
            output.push_str("match __match_borrow_break.as_ref()");
        } else {
            output.push_str("match ");
            if has_string_literal && !is_tuple_match {
                // TDD FIX: Don't add .as_str() if value_str already has it OR if it's already &str
                // value_str may have been simplified (redundant .as_str() removed)
                if !value_str.ends_with(".as_str()") {
                    // Check if the simplified value_str is an identifier that's already &str
                    let is_borrowed_param = self.inferred_borrowed_params.contains(&value_str);
                    let is_string_type_param = self.current_function_params.iter().any(|p| {
                        p.name == value_str
                            && (matches!(p.type_, crate::parser::Type::String)
                                || matches!(p.type_, crate::parser::Type::Custom(ref n) if n == "str" || n == "string" || n == "&str"))
                    });
                    if is_borrowed_param || is_string_type_param {
                        // Already &str, don't add .as_str()
                        output.push_str(&value_str);
                    } else {
                        // Not &str, add .as_str()
                        output.push_str(&format!("{}.as_str()", value_str));
                    }
                } else {
                    // Already has .as_str()
                    output.push_str(&value_str);
                }
            } else {
                output.push_str(&value_str);
            }
        }

        output.push_str(" {\n");

        self.indent_level += 1;

        let match_binds_refs = self.match_expression_binds_refs(value);

        let needs_string_conversion =
            string_utilities::return_type_expects_owned_string(&self.current_function_return_type)
                || arms.iter().any(|arm| {
                    string_analysis::expression_produces_string(arm.body)
                        || arm_string_analysis::arm_returns_converted_string(arm.body)
                });

        let old_in_statement_match = self.in_statement_match;
        let match_is_statement = self.current_function_return_type.is_none();
        if match_is_statement {
            self.in_statement_match = true;
        }

        // If any arm has an empty body (returns ()), treat all arms as void
        // to prevent type mismatches between () and non-() return values.
        let has_void_arm = arms.iter().any(
            |arm| matches!(arm.body, Expression::Block { statements, .. } if statements.is_empty()),
        );

        let scrutinee_type_has_ref = self.expression_type_contains_reference(value);
        // When the scrutinee has been dereferenced (`*self`, `*e`, etc.) for a Copy type,
        // the match operates on an owned value and pattern bindings are owned — NOT refs.
        // Generalized from the original `value_str == "*self"` to handle all Copy params.
        let owned_bindings_from_copy_deref = if let Some(deref_name) = value_str.strip_prefix('*') {
            if deref_name == "self" {
                self.current_struct_name
                    .as_ref()
                    .is_some_and(|sn| self.is_type_copy(&Type::Custom(sn.clone())))
            } else if let Some(ty) = self.infer_expression_type(value) {
                let inner = match &ty {
                    Type::Reference(inner) | Type::MutableReference(inner) => inner.as_ref(),
                    other => other,
                };
                self.is_type_copy(inner) && !Self::type_contains_reference(inner)
            } else {
                false
            }
        } else {
            false
        };

        for arm in arms {
            // Upgrade pattern bindings to `mut` when the arm body mutates them
            let body_stmts: &[&Statement<'ast>] =
                if let Expression::Block { statements, .. } = arm.body {
                    statements.as_slice()
                } else {
                    &[]
                };
            let upgraded_pattern = self.upgrade_pattern_mut_bindings(
                &arm.pattern,
                body_stmts,
                match_scrutinee_ref_prefix.contains("mut"),
            );

            output.push_str(&self.indent());
            output.push_str(&self.generate_pattern(&upgraded_pattern));

            if let Some(guard) = &arm.guard {
                output.push_str(" if ");
                output.push_str(&self.generate_expression(guard));
            }

            output.push_str(" => ");

            let mut bound_vars = std::collections::HashSet::new();
            self.extract_pattern_bindings(&arm.pattern, &mut bound_vars);

            // TDD FIX for E0614: Track match arm bindings as OWNED values
            // Match arm bindings extract owned values from enums, NOT references
            // This prevents incorrectly adding * to Copy types like i32 in comparisons
            for var in &bound_vars {
                self.match_arm_bindings.insert(var.clone());
            }

            let added_borrowed: Vec<String> = if (match_binds_refs || scrutinee_type_has_ref)
                && !owned_bindings_from_copy_deref
            {
                bound_vars.iter().cloned().collect()
            } else {
                Vec::new()
            };
            for var in &added_borrowed {
                self.borrowed_iterator_vars.insert(var.clone());
            }

            // Clone bound_vars before moving it, so we can clean up match_arm_bindings later
            let bound_vars_for_cleanup = bound_vars.clone();

            self.local_variable_scopes.push(bound_vars);

            let mut match_bound_type_entries: Vec<(String, Type)> =
                self.infer_match_bound_types(value, &arm.pattern);
            // Wrap binding types with the ref kind matching the
            // generated scrutinee prefix (see if-let equivalent above).
            if match_scrutinee_ref_prefix == "&mut " {
                for entry in &mut match_bound_type_entries {
                    if !matches!(entry.1, Type::Reference(_) | Type::MutableReference(_)) {
                        entry.1 = Type::MutableReference(Box::new(entry.1.clone()));
                    }
                }
            } else if match_scrutinee_ref_prefix == "& " || match_scrutinee_ref_prefix == "&" {
                for entry in &mut match_bound_type_entries {
                    if !matches!(entry.1, Type::Reference(_) | Type::MutableReference(_)) {
                        entry.1 = Type::Reference(Box::new(entry.1.clone()));
                    }
                }
            }
            for (var_name, var_type) in &match_bound_type_entries {
                self.local_var_types
                    .insert(var_name.clone(), var_type.clone());
            }

            let old_in_match_arm = self.in_match_arm_needing_string;
            if needs_string_conversion {
                self.in_match_arm_needing_string = true;
            }

            let old_void_block = self.in_void_block;
            if has_void_arm {
                self.in_void_block = true;
            }
            let mut arm_str = self.generate_expression(arm.body);
            self.in_void_block = old_void_block;

            self.in_match_arm_needing_string = old_in_match_arm;

            if (match_binds_refs || scrutinee_type_has_ref) && !arm_str.ends_with(".clone()") {
                // Extract the binding name from either a direct identifier
                // or a block whose only/last statement is an expression identifier
                let binding_name: Option<&str> =
                    if let Expression::Identifier { name, .. } = arm.body {
                        Some(name)
                    } else if let Expression::Block { statements, .. } = arm.body {
                        if let Some(Statement::Expression { expr, .. }) = statements.last() {
                            if let Expression::Identifier { name, .. } = expr {
                                Some(name)
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                let is_simple_binding_return =
                    binding_name.is_some_and(|n| added_borrowed.contains(&n.to_string()));
                if is_simple_binding_return {
                    let bname = binding_name.unwrap();
                    let binding_type = match_bound_type_entries
                        .iter()
                        .find(|(n, _)| n == bname)
                        .map(|(_, t)| t);
                    let inner_type = match binding_type {
                        Some(Type::Reference(inner)) | Some(Type::MutableReference(inner)) => {
                            Some(inner.as_ref())
                        }
                        other => other,
                    };
                    let is_copy = inner_type.is_some_and(|t| self.is_type_copy(t));
                    // For block bodies, replace the binding name inside the
                    // generated string since we can't prefix the whole block.
                    let deref_expr = if is_copy {
                        format!("*{}", bname)
                    } else {
                        format!("{}.clone()", bname)
                    };
                    if let Expression::Identifier { .. } = arm.body {
                        arm_str = deref_expr;
                    } else {
                        // Block body: replace the last occurrence of the
                        // bare binding with its dereffed version
                        if let Some(pos) = arm_str.rfind(bname) {
                            let after = pos + bname.len();
                            if after >= arm_str.len()
                                || !arm_str[after..after + 1]
                                    .chars()
                                    .next()
                                    .unwrap_or(' ')
                                    .is_alphanumeric()
                            {
                                arm_str.replace_range(pos..after, &deref_expr);
                            }
                        }
                    }
                }
            }

            self.local_variable_scopes.pop();

            for (var_name, _) in &match_bound_type_entries {
                self.local_var_types.remove(var_name);
            }

            for var in &added_borrowed {
                self.borrowed_iterator_vars.remove(var);
            }

            // TDD FIX: Clean up match arm bindings after each arm
            for var in &bound_vars_for_cleanup {
                self.match_arm_bindings.remove(var);
            }
            let is_string_literal = matches!(
                &arm.body,
                Expression::Literal {
                    value: Literal::String(_),
                    ..
                }
            );

            if needs_string_conversion && is_string_literal && !arm_str.ends_with(".to_string()") {
                arm_str = format!("{}.to_string()", arm_str);
            }

            output.push_str(&arm_str);
            output.push_str(",\n");
        }

        self.in_statement_match = old_in_statement_match;

        self.indent_level -= 1;

        output.push_str(&self.indent());
        output.push_str("}\n");
        output
    }

    fn generate_for_statement(
        &mut self,
        pattern: &Pattern,
        iterable: &Expression<'ast>,
        body: &[&'ast Statement<'ast>],
        location: &crate::parser::ast::SourceLocation,
    ) -> String {
        let mut output = self.indent();
        output.push_str("for ");

        let pattern_str = self.pattern_to_rust(pattern);
        let loop_var = pattern_analysis::extract_pattern_identifier(pattern);

        // TDD FIX: Check if ANY binding in the pattern is mutated (not just simple identifier)
        // For tuple patterns like (id, val), extract ALL bindings and check each one
        let mut all_pattern_bindings = std::collections::HashSet::new();
        self.extract_pattern_bindings(pattern, &mut all_pattern_bindings);

        let needs_mut = if let Some(var) = loop_var.as_ref() {
            // Simple identifier pattern: check if it's mutated
            self.loop_body_modifies_variable(body, var)
                || self.loop_body_calls_mut_dispatch_method(iterable, body, var)
        } else {
            // Tuple or complex pattern: check if ANY binding is mutated
            all_pattern_bindings.iter().any(|var| {
                self.loop_body_modifies_variable(body, var)
                    || self.loop_body_calls_mut_dispatch_method(iterable, body, var)
            })
        };

        let needs_borrow = self.should_borrow_for_iteration(iterable);
        let needs_mut_borrow = needs_mut && needs_borrow;

        let iterable_already_mut_ref = matches!(
            iterable,
            Expression::Unary {
                op: UnaryOp::MutRef,
                ..
            }
        );
        if needs_mut && !needs_mut_borrow && !iterable_already_mut_ref {
            output.push_str("mut ");
        }

        let is_unused_loop_var = location
            .as_ref()
            .is_some_and(|loc| self.unused_let_bindings.contains(&(loc.line, loc.column)));
        let display_pattern = if is_unused_loop_var {
            format!("_{}", pattern_str)
        } else {
            pattern_str
        };
        output.push_str(&display_pattern);
        output.push_str(" in ");

        let is_borrowed_iterator = needs_borrow || self.is_iterating_over_borrowed(iterable);

        if needs_mut_borrow {
            output.push_str("&mut ");
        } else if needs_borrow {
            output.push('&');
        }

        let iterable_to_generate = if let Expression::Unary {
            op: crate::parser::UnaryOp::Ref,
            operand,
            ..
        } = iterable
        {
            if let Expression::Identifier { name, .. } = &**operand {
                if self.inferred_borrowed_params.contains(name) {
                    operand
                } else {
                    iterable
                }
            } else {
                iterable
            }
        } else {
            iterable
        };

        // Suppress auto-clone on the iterable: for-loops iterate by reference
        // when `&` is prepended, so cloning a Vec<Box<dyn Trait>> or Vec<T>
        // is unnecessary and fails when T doesn't implement Clone.
        let prev_field_access = self.in_field_access_object;
        self.in_field_access_object = true;
        output.push_str(&self.generate_expression(iterable_to_generate));
        self.in_field_access_object = prev_field_access;
        output.push_str(" {\n");

        self.indent_level += 1;

        // TDD FIX: Track ALL bound variables in tuple patterns for explicit deref fix
        // For `for (id, value) in items`, both `id` and `value` need to be tracked
        if is_borrowed_iterator {
            let mut all_bindings = std::collections::HashSet::new();
            self.extract_pattern_bindings(pattern, &mut all_bindings);
            for var in all_bindings {
                self.borrowed_iterator_vars.insert(var.clone());
                // Track mutable borrows separately for compound assignment deref
                if needs_mut_borrow {
                    self.mut_borrowed_iterator_vars.insert(var);
                }
            }
        }

        let is_owned_string_iterator = !is_borrowed_iterator;
        if is_owned_string_iterator {
            if let Some(var) = &loop_var {
                self.owned_string_iterator_vars.insert(var.clone());
            }
        }

        if let Some(var) = &loop_var {
            if let Expression::Range { end, .. } = iterable {
                if self.expression_produces_usize(end) {
                    self.usize_variables.insert(var.clone());
                }
            }
        }

        // TDD FIX: Track types for ALL bound variables (simple and tuple patterns)
        if let Some(iterable_type) = self.infer_expression_type(iterable) {
            if let Some(elem_type) = Self::extract_iterator_element_type(&iterable_type) {
                match pattern {
                    Pattern::Identifier(var) => {
                        self.local_var_types.insert(var.clone(), elem_type);
                    }
                    Pattern::Tuple(patterns) => {
                        // elem_type should be Tuple with matching arity
                        if let Type::Tuple(tuple_types) = &elem_type {
                            for (pat, ty) in patterns.iter().zip(tuple_types.iter()) {
                                if let Pattern::Identifier(var) = pat {
                                    self.local_var_types.insert(var.clone(), ty.clone());
                                }
                            }
                        }
                    }
                    _ => {
                        // For other patterns, use the old loop_var approach
                        if let Some(var) = &loop_var {
                            self.local_var_types.insert(var.clone(), elem_type);
                        }
                    }
                }
            }
        }

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

        if is_borrowed_iterator {
            if let Some(var) = &loop_var {
                self.borrowed_iterator_vars.remove(var);
            }
        }
        if is_owned_string_iterator {
            if let Some(var) = &loop_var {
                self.owned_string_iterator_vars.remove(var);
            }
        }
        if let Some(var) = &loop_var {
            self.local_var_types.remove(var);
        }

        self.indent_level -= 1;

        output.push_str(&self.indent());
        output.push_str("}\n");
        output
    }

    fn generate_assignment_statement(
        &mut self,
        target: &Expression<'ast>,
        value: &Expression<'ast>,
        compound_op: &Option<CompoundOp>,
    ) -> String {
        let mut output = self.indent();

        if let Some(op) = compound_op {
            self.generating_assignment_target = true;
            let target_str = self.generate_expression(target);
            self.generating_assignment_target = false;

            // TDD FIX: Compound assignments on mutable references need dereference operator
            // For loop variables bound from &mut iteration are &mut T, so `var += x` must become `*var += x`
            let needs_deref = if let Expression::Identifier { name, .. } = target {
                self.mut_borrowed_iterator_vars.contains(name)
            } else {
                false
            };

            if needs_deref {
                output.push('*');
            }
            output.push_str(&target_str);

            output.push_str(match op {
                CompoundOp::Add => " += ",
                CompoundOp::Sub => " -= ",
                CompoundOp::Mul => " *= ",
                CompoundOp::Div => " /= ",
                CompoundOp::Mod => " %= ",
                CompoundOp::BitAnd => " &= ",
                CompoundOp::BitOr => " |= ",
                CompoundOp::BitXor => " ^= ",
                CompoundOp::Shl => " <<= ",
                CompoundOp::Shr => " >>= ",
            });

            let prev_assign_ty = self.assignment_float_target_type.take();
            let tgt_ty = self.infer_expression_type(target);
            if tgt_ty
                .as_ref()
                .is_some_and(Self::assignment_target_needs_float_codegen_context)
            {
                self.assignment_float_target_type = tgt_ty.clone();
            }
            let mut value_str = self.generate_expression(value);

            // Mixed int/float compound assignment: `f32 += i32` → `f32 += i32 as f32`
            // Only cast when the target is genuinely a float type (not int).
            if matches!(
                op,
                CompoundOp::Add
                    | CompoundOp::Sub
                    | CompoundOp::Mul
                    | CompoundOp::Div
                    | CompoundOp::Mod
            ) {
                let val_ty = self.infer_expression_type(value);
                let tgt_is_int = tgt_ty.as_ref().is_some_and(Self::is_int_numeric_type);
                if !tgt_is_int {
                    if let Some(v) = &val_ty {
                        if Self::is_int_numeric_type(v) {
                            let float_name = self.resolve_compound_assign_float_target(target);
                            if let Some(fname) = float_name {
                                if value_str.contains(" as ")
                                    || matches!(value, Expression::Binary { .. })
                                {
                                    value_str = format!("({}) as {}", value_str, fname);
                                } else {
                                    value_str = format!("{} as {}", value_str, fname);
                                }
                            }
                        }
                    }
                }
            }

            self.assignment_float_target_type = prev_assign_ty;

            // String += String doesn't work in Rust (needs String += &str).
            // Only add & when the RHS is NOT a Copy type — Copy types (i32, f32, etc.)
            // work directly in compound assignments without borrowing.
            if matches!(op, CompoundOp::Add) {
                let value_is_copy = self
                    .infer_expression_type(value)
                    .as_ref()
                    .is_some_and(|t| self.is_type_copy(t));

                if !value_is_copy {
                    if let Expression::Identifier { name, .. } = value {
                        if self.owned_string_iterator_vars.contains(name) {
                            value_str = format!("&{}", value_str);
                        }
                    }

                    let value_type = self.infer_expression_type(value);
                    if matches!(value_type, Some(Type::String)) {
                        let is_string_literal = matches!(
                            value,
                            Expression::Literal {
                                value: Literal::String(_),
                                ..
                            }
                        );
                        let already_borrowed = value_str.starts_with('&');

                        if !is_string_literal && !already_borrowed {
                            value_str = format!("&{}", value_str);
                        }
                    }
                }
            }

            output.push_str(&value_str);
            output.push_str(";\n");
            return output;
        }

        if let Expression::Binary {
            left, right, op, ..
        } = value
        {
            let targets_match = match (target, &**left) {
                (
                    Expression::Identifier { name: t, .. },
                    Expression::Identifier { name: l, .. },
                ) => t == l,
                (Expression::FieldAccess { .. }, Expression::FieldAccess { .. })
                | (Expression::Index { .. }, Expression::Index { .. }) => {
                    self.generate_expression(target) == self.generate_expression(left)
                }
                _ => false,
            };

            let target_type = self.infer_expression_type(target);
            let right_type = self.infer_expression_type(right);

            // TDD FIX: String += String/&str doesn't work in Rust (needs String += &str with explicit &)
            // Disable compound assignment if EITHER:
            // 1. Right side is String/&str (needs borrowing)
            // 2. Target is String (likely string concatenation)
            let right_is_string_like = match &right_type {
                Some(Type::String) => true,
                Some(Type::Reference(inner)) => matches!(&**inner, Type::String),
                _ => false,
            };
            let target_is_string = matches!(&target_type, Some(Type::String));
            let is_string_addition =
                matches!(op, BinaryOp::Add) && (right_is_string_like || target_is_string);

            let target_supports_compound_assign = target_type.as_ref().is_some_and(|t| {
                matches!(
                    t,
                    Type::Int | Type::Int32 | Type::Uint | Type::Float | Type::Bool
                ) || matches!(t, Type::Custom(name) if crate::type_classification::is_numeric_type(name))
            });
            let is_compound_safe = target_supports_compound_assign && !is_string_addition;

            if targets_match && is_compound_safe {
                let compound_op_str = match op {
                    BinaryOp::Add => Some("+="),
                    BinaryOp::Sub => Some("-="),
                    BinaryOp::Mul => Some("*="),
                    BinaryOp::Div => Some("/="),
                    BinaryOp::Mod => Some("%="),
                    BinaryOp::BitAnd => Some("&="),
                    BinaryOp::BitOr => Some("|="),
                    BinaryOp::BitXor => Some("^="),
                    BinaryOp::Shl => Some("<<="),
                    BinaryOp::Shr => Some(">>="),
                    _ => None,
                };

                if let Some(op_str) = compound_op_str {
                    self.generating_assignment_target = true;
                    let target_str = self.generate_expression(target);
                    self.generating_assignment_target = false;

                    // TDD FIX: Compound assignments on mutable references need deref operator
                    let needs_deref = if let Expression::Identifier { name, .. } = target {
                        self.mut_borrowed_iterator_vars.contains(name)
                    } else {
                        false
                    };

                    if needs_deref {
                        output.push('*');
                    }
                    output.push_str(&target_str);
                    output.push(' ');
                    output.push_str(op_str);
                    output.push(' ');
                    let prev_assign_ty = self.assignment_float_target_type.take();
                    let tgt_ty = self.infer_expression_type(target);
                    if tgt_ty
                        .as_ref()
                        .is_some_and(Self::assignment_target_needs_float_codegen_context)
                    {
                        self.assignment_float_target_type = tgt_ty.clone();
                    }
                    let mut right_str = self.generate_expression(right);

                    // Mixed int/float: cast RHS integer to target float type
                    // Only cast when the target is genuinely a float type (not int).
                    let synth_tgt_is_int = tgt_ty.as_ref().is_some_and(Self::is_int_numeric_type);
                    if !synth_tgt_is_int {
                        if matches!(
                            op,
                            BinaryOp::Add
                                | BinaryOp::Sub
                                | BinaryOp::Mul
                                | BinaryOp::Div
                                | BinaryOp::Mod
                        ) {
                            let rhs_ty = self.infer_expression_type(right);
                            if let Some(v) = &rhs_ty {
                                if Self::is_int_numeric_type(v) {
                                    let tgt_float =
                                        self.resolve_compound_assign_float_target(target);
                                    if let Some(float_name) = tgt_float {
                                        if right_str.contains(" as ")
                                            || matches!(&**right, Expression::Binary { .. })
                                        {
                                            right_str =
                                                format!("({}) as {}", right_str, float_name);
                                        } else {
                                            right_str = format!("{} as {}", right_str, float_name);
                                        }
                                    }
                                }
                            }
                        }
                    }

                    self.assignment_float_target_type = prev_assign_ty;

                    output.push_str(&right_str);
                    output.push_str(";\n");
                    return output;
                }
            }
        }

        self.generating_assignment_target = true;
        let target_str = self.generate_expression(target);
        self.generating_assignment_target = false;

        // TDD FIX: Regular assignments on mutable references need deref operator
        // For loop variables bound from &mut iteration are &mut T, so `var = x` must become `*var = x`
        let needs_deref = if let Expression::Identifier { name, .. } = target {
            self.mut_borrowed_iterator_vars.contains(name)
        } else {
            false
        };

        if needs_deref {
            output.push('*');
        }
        output.push_str(&target_str);
        output.push_str(" = ");

        let old_expr_ctx = self.in_expression_context;
        self.in_expression_context = true;

        let prev_assign_ty = self.assignment_float_target_type.take();
        let tgt_ty = self.infer_expression_type(target);
        if tgt_ty
            .as_ref()
            .is_some_and(Self::assignment_target_needs_float_codegen_context)
        {
            self.assignment_float_target_type = tgt_ty.clone();
        }
        let mut value_str = self.generate_expression(value);
        self.assignment_float_target_type = prev_assign_ty;
        if matches!(
            value,
            Expression::Literal {
                value: Literal::String(_),
                ..
            }
        ) {
            value_str = format!("{}.to_string()", value_str);
        }

        if let Expression::Identifier { ref name, .. } = value {
            if self.inferred_borrowed_params.contains(name) {
                let target_type = self.infer_expression_type(target);
                let assignment_target_is_text = target_type
                    .as_ref()
                    .is_some_and(crate::codegen::rust::types::is_windjammer_text_type);
                if assignment_target_is_text {
                    if !value_str.contains(".clone()") && !value_str.contains(".to_string()") {
                        value_str = format!("{}.to_string()", value_str);
                    }
                }
            }
            // E0308 FIX: match-bound variables from &/&mut scrutinees are references.
            // When assigning to a Copy-type field (e.g. self.x = min_x where min_x: &mut f32),
            // auto-deref the value.
            if self.borrowed_iterator_vars.contains(name) && !value_str.starts_with('*') {
                let target_type = self.infer_expression_type(target);
                if target_type.as_ref().is_some_and(|t| self.is_type_copy(t)) {
                    value_str = format!("*{}", value_str);
                }
            }
        }

        // Auto-clone when assigning one self field from another self field.
        // In Rust, `self.a = self.b` is E0507 when self is &mut self and b is non-Copy,
        // because you can't move out of a mutable reference. Clone solves this.
        if !value_str.ends_with(".clone()") && !value_str.ends_with(".to_string()") {
            let target_is_self_field = matches!(target, Expression::FieldAccess { object, .. }
                    if matches!(&**object, Expression::Identifier { name, .. } if name == "self"));
            let value_is_self_field = matches!(value, Expression::FieldAccess { object, .. }
                    if matches!(&**object, Expression::Identifier { name, .. } if name == "self"));

            if target_is_self_field && value_is_self_field {
                let val_type = self.infer_expression_type(value);
                let is_copy = val_type.as_ref().is_some_and(|t| self.is_type_copy(t));
                if !is_copy {
                    value_str = format!("{}.clone()", value_str);
                }
            }
        }

        if self.expression_produces_usize(value) {
            let target_type = self.get_assignment_target_type(target);

            match target_type.as_deref() {
                Some("usize") => {}
                Some("int") | Some("i64") => {
                    value_str = format!("(({}) as i64)", value_str);
                }
                Some("i32") => {
                    value_str = format!("(({}) as i32)", value_str);
                }
                _ => {}
            }
        }

        output.push_str(&value_str);

        self.in_expression_context = old_expr_ctx;

        output.push_str(";\n");
        output
    }

    fn pattern_to_rust(&self, pattern: &Pattern) -> String {
        use crate::parser::EnumPatternBinding;
        match pattern {
            Pattern::Wildcard => "_".to_string(),
            Pattern::Identifier(name) => name.clone(),
            Pattern::MutBinding(name) => format!("mut {}", name),
            Pattern::Reference(inner) => format!("&{}", self.pattern_to_rust(inner)),
            Pattern::Ref(name) => format!("ref {}", name),
            Pattern::RefMut(name) => format!("ref mut {}", name),
            Pattern::Tuple(patterns) => {
                let rust_patterns: Vec<String> =
                    patterns.iter().map(|p| self.pattern_to_rust(p)).collect();
                format!("({})", rust_patterns.join(", "))
            }
            Pattern::EnumVariant(variant, binding) => match binding {
                EnumPatternBinding::Single(name) => format!("{}({})", variant, name),
                EnumPatternBinding::Wildcard => format!("{}(_)", variant),
                EnumPatternBinding::None => variant.clone(),
                EnumPatternBinding::Tuple(patterns) => {
                    let rust_patterns: Vec<String> =
                        patterns.iter().map(|p| self.pattern_to_rust(p)).collect();
                    format!("{}({})", variant, rust_patterns.join(", "))
                }
                EnumPatternBinding::Struct(fields, has_wildcard) => {
                    if fields.is_empty() {
                        format!("{} {{ .. }}", variant)
                    } else {
                        let field_strs: Vec<String> = fields
                            .iter()
                            .map(|(name, pat)| format!("{}: {}", name, self.pattern_to_rust(pat)))
                            .collect();
                        if *has_wildcard {
                            format!("{} {{ {}, .. }}", variant, field_strs.join(", "))
                        } else {
                            format!("{} {{ {} }}", variant, field_strs.join(", "))
                        }
                    }
                }
            },
            Pattern::Literal(lit) => self.generate_literal(lit),
            Pattern::Or(patterns) => {
                let rust_patterns: Vec<String> =
                    patterns.iter().map(|p| self.pattern_to_rust(p)).collect();
                rust_patterns.join(" | ")
            }
        }
    }

    pub(crate) fn generate_pattern(&self, pattern: &Pattern) -> String {
        use crate::parser::EnumPatternBinding;
        match pattern {
            Pattern::Wildcard => "_".to_string(),
            Pattern::Identifier(name) => name.clone(),
            Pattern::MutBinding(name) => format!("mut {}", name),
            Pattern::Reference(inner) => format!("&{}", self.generate_pattern(inner)),
            Pattern::Ref(name) => format!("ref {}", name),
            Pattern::RefMut(name) => format!("ref mut {}", name),
            Pattern::EnumVariant(name, binding) => match binding {
                EnumPatternBinding::Single(b) => format!("{}({})", name, b),
                EnumPatternBinding::Wildcard => format!("{}(_)", name),
                EnumPatternBinding::None => name.clone(),
                EnumPatternBinding::Tuple(patterns) => {
                    let rust_patterns: Vec<String> =
                        patterns.iter().map(|p| self.generate_pattern(p)).collect();
                    format!("{}({})", name, rust_patterns.join(", "))
                }
                EnumPatternBinding::Struct(fields, has_wildcard) => {
                    if fields.is_empty() {
                        format!("{} {{ .. }}", name)
                    } else {
                        let field_strs: Vec<String> = fields
                            .iter()
                            .map(|(n, pat)| {
                                if let Pattern::Identifier(binding) = pat {
                                    if binding == n {
                                        return n.clone();
                                    }
                                }
                                format!("{}: {}", n, self.generate_pattern(pat))
                            })
                            .collect();
                        if *has_wildcard {
                            format!("{} {{ {}, .. }}", name, field_strs.join(", "))
                        } else {
                            format!("{} {{ {} }}", name, field_strs.join(", "))
                        }
                    }
                }
            },
            Pattern::Literal(lit) => self.generate_literal(lit),
            Pattern::Tuple(patterns) => {
                let pattern_strs: Vec<String> =
                    patterns.iter().map(|p| self.generate_pattern(p)).collect();
                format!("({})", pattern_strs.join(", "))
            }
            Pattern::Or(patterns) => {
                let pattern_strs: Vec<String> =
                    patterns.iter().map(|p| self.generate_pattern(p)).collect();
                pattern_strs.join(" | ")
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
    fn apply_vec_index_let_rhs_fixup(
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
    fn let_rhs_clone_if_mut_from_non_copy_ref(
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
    fn apply_self_field_move_fix(&self, value: &Expression<'ast>, value_str: &mut String) {
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
    fn register_tuple_let_binding_types(
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

    fn type_contains_reference(ty: &Type) -> bool {
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
    fn option_scrutinee_ref_prefix(&self, value: &Expression<'ast>) -> &'static str {
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
    fn effective_option_scrutinee_ref_prefix(
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

    fn some_pattern_single_binding<'p>(pattern: &'p Pattern<'p>) -> Option<&'p str> {
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
    fn match_scrutinee_is_self_field(&self, expr: &Expression) -> bool {
        match expr {
            Expression::FieldAccess { object, .. } => {
                matches!(&**object, Expression::Identifier { name, .. } if name == "self")
                    || self.match_scrutinee_is_self_field(object)
            }
            Expression::Index { object, .. } => self.match_scrutinee_is_self_field(object),
            _ => false,
        }
    }

    fn match_scrutinee_is_self_method_call(&self, expr: &Expression) -> bool {
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

    fn match_arms_mutate_self(&self, arms: &[crate::parser::MatchArm<'ast>]) -> bool {
        let ctx = self_analysis::AnalysisContext::new(&[], &self.current_struct_fields);
        arms.iter()
            .any(|arm| self_analysis::expression_mutates_fields(&ctx, arm.body))
    }

    fn get_assignment_target_type(&self, target: &Expression) -> Option<String> {
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

    fn returns_option_owned_type(&self) -> bool {
        match &self.current_function_return_type {
            Some(Type::Option(inner_type)) => {
                !matches!(**inner_type, Type::Reference(_) | Type::MutableReference(_))
            }
            _ => false,
        }
    }
}
