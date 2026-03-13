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
use super::CodeGenerator;

#[allow(clippy::collapsible_match, clippy::collapsible_if)]
impl<'ast> CodeGenerator<'ast> {
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
        for (i, stmt) in stmts.iter().enumerate() {
            // Track current statement index for optimization hints
            self.current_statement_idx = i;

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
                        let mut expr_str = self.generate_expression(expr);

                        // WINDJAMMER PHILOSOPHY: Auto-convert implicit returns when function returns String
                        // BUT: Don't convert if:
                        // 1. The expression explicitly uses .as_str() (user wants &str)
                        // 2. A sibling branch in an if-else uses .as_str() (type consistency)
                        let returns_string = match &self.current_function_return_type {
                            Some(Type::String) => true,
                            Some(Type::Custom(name)) if name == "String" => true,
                            _ => false,
                        };

                        // Also check if we're in a match arm that needs string conversion
                        let in_match_needing_string = self.in_match_arm_needing_string;

                        // Check if the expression explicitly returns &str via .as_str()
                        let expr_uses_as_str = expr_str.contains(".as_str()");

                        // Check if we should suppress conversion (sibling branch has .as_str())
                        let should_suppress = self.suppress_string_conversion;

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
                        if expr_str.starts_with("&") && !expr_str.ends_with(".clone()") {
                            if let Some(inner) = self.infer_expression_type(expr) {
                                if !self.is_type_copy(&inner) {
                                    expr_str = format!("({}).clone()", expr_str);
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

                        // WINDJAMMER PHILOSOPHY: Auto-add .cloned() for HashMap.get() and similar methods
                        // When returning Option<T> but method returns Option<&T>, add .cloned()
                        let returns_option_owned = self.returns_option_owned_type();
                        if returns_option_owned
                            && self.is_method_returning_option_ref(expr)
                            && !expr_str.ends_with(".cloned()")
                            && !expr_str.ends_with(".clone()")
                        {
                            expr_str = format!("{}.cloned()", expr_str);
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
                            let mut expr_str = self.generate_expression(expr);

                            // WINDJAMMER PHILOSOPHY: Auto-convert implicit returns when function returns String
                            // Same logic as Statement::Expression implicit returns
                            let returns_string = match &self.current_function_return_type {
                                Some(Type::String) => true,
                                Some(Type::Custom(name)) if name == "String" => true,
                                _ => false,
                            };

                            let in_match_needing_string = self.in_match_arm_needing_string;
                            let expr_uses_as_str = expr_str.contains(".as_str()");
                            let should_suppress = self.suppress_string_conversion;

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

                            // WINDJAMMER PHILOSOPHY: Auto-add .cloned() for HashMap.get() and similar methods
                            // When returning Option<T> but method returns Option<&T>, add .cloned()
                            let returns_option_owned = self.returns_option_owned_type();
                            if returns_option_owned
                                && self.is_method_returning_option_ref(expr)
                                && !expr_str.ends_with(".cloned()")
                                && !expr_str.ends_with(".clone()")
                            {
                                expr_str = format!("{}.cloned()", expr_str);
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

                // EXPLICIT MUTABILITY: Follow Rust/Swift/Kotlin standard
                // Users must write `let mut x = 0` when reassignment is intended
                // This prevents accidental state mutation bugs (critical for game engines)
                if needs_mut_ref {
                    // Don't add mut keyword, but we'll add &mut to the value
                } else if *mutable {
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
                                            Some(Type::Custom(type_name.to_string()))
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

                        // Auto-convert &str to String if type is String
                        let mut value_str = self.generate_expression(value);
                        let is_string_type = matches!(t, Type::String)
                            || matches!(t, Type::Custom(name) if name == "String");

                        // Convert string literals OR identifiers to String when target is String
                        if is_string_type {
                            let should_convert = matches!(
                                value,
                                Expression::Literal {
                                    value: Literal::String(_),
                                    ..
                                } | Expression::Identifier { .. }
                            );
                            if should_convert {
                                value_str = format!("{}.to_string()", value_str);
                            }
                        }
                        output.push_str(&value_str);
                    } else {
                        output.push_str(" = ");
                        if needs_mut_ref {
                            output.push_str("&mut ");
                        }

                        // EXPRESSION CONTEXT: Mark that we're generating a value that will be used
                        // This prevents adding semicolons to if-else branches when used in let bindings
                        let old_ctx = self.in_expression_context;
                        self.in_expression_context = true;

                        // WINDJAMMER PHILOSOPHY: Auto-convert string literals to String
                        // String literals assigned to variables should become String (not &str)
                        // because they may be passed to functions expecting String later.
                        // This is safe because String auto-borrows to &str when needed.
                        let mut value_str = self.generate_expression(value);

                        // TDD FIX: Vec indexing ownership inference
                        // WINDJAMMER PHILOSOPHY: "Compiler does the hard work, not the developer"
                        //
                        // Pattern: let x = vec[i]
                        // If vec[i] type is Copy → no modification needed (Rust copies automatically)
                        // If vec[i] type is Clone (not Copy):
                        //   - If only field-accessed → &vec[i] (optimize to borrow)
                        //   - If moved/returned → vec[i].clone() (need explicit clone)
                        if matches!(value, Expression::Index { .. }) {
                            if let Some(name) = var_name {
                                // TDD FIX: Only apply ownership transformations if we can infer the type
                                // WINDJAMMER PHILOSOPHY: Be conservative - better to get a clear E0507
                                // than add wrong & causing E0308
                                let indexed_type = self.infer_expression_type(value);

                                if let Some(elem_type) = indexed_type {
                                    // SUCCESS: We know the element type
                                    let is_copy = self.is_type_copy(&elem_type);

                                    if is_copy {
                                        // Copy types don't need & or .clone() - Rust copies automatically
                                        // Example: let x = numbers[0] → let x = numbers[0]
                                        // DO NOTHING - leave as-is
                                    } else {
                                        // Non-Copy type - need to decide between & and .clone()
                                        // DATA FLOW ANALYSIS: Check how variable is used
                                        if self.variable_is_only_field_accessed(name) {
                                            // Only field-accessed → optimize with borrow
                                            // Example: let frame = frames[i]; frame.x += 1;
                                            // Generate: let frame = &frames[i]
                                            // TDD FIX: Set in_borrow_context BEFORE generating expression
                                            // to prevent Expression::Index from adding .clone()
                                            // Without this: value_str = "vec[i].clone()" then "&" → "&vec[i].clone()" ❌
                                            // With this: value_str = "vec[i]" then "&" → "&vec[i]" ✅
                                            let prev_borrow_ctx = self.in_borrow_context;
                                            self.in_borrow_context = true;
                                            value_str = self.generate_expression(value);
                                            self.in_borrow_context = prev_borrow_ctx;
                                            value_str = format!("&{}", value_str);
                                        } else {
                                            // Moved/returned → Expression::Index generates & (auto-borrow)
                                            // For String: &str needs .to_string() when variable will be returned
                                            // For other non-Copy: &T needs .clone() when variable will be moved
                                            if value_str.starts_with('&') {
                                                let is_string =
                                                    matches!(elem_type, Type::String)
                                                        || matches!(elem_type, Type::Custom(n) if n == "string");
                                                if is_string {
                                                    value_str =
                                                        format!("({}).to_string()", value_str);
                                                } else {
                                                    value_str =
                                                        format!("({}).clone()", value_str);
                                                }
                                            }
                                        }
                                    }
                                } else {
                                    // CANNOT INFER: Leave as-is, let Rust give clear error
                                    // This happens when Vec is created without explicit type annotation
                                    // Example: let mask = Vec::with_capacity(size); let x = mask[i];
                                    // Better to get E0507 "cannot move" than E0308 "expected u8, found &u8"
                                }
                            }
                        } else if matches!(
                            value,
                            Expression::Literal {
                                value: Literal::String(_),
                                ..
                            }
                        ) {
                            value_str = format!("{}.to_string()", value_str);
                        }

                        output.push_str(&value_str);

                        // Restore expression context
                        self.in_expression_context = old_ctx;
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

                        // Auto-convert &str to String if type is String
                        let mut value_str = self.generate_expression(value);
                        let is_string_type = matches!(t, Type::String)
                            || matches!(t, Type::Custom(name) if name == "String");

                        // Convert string literals OR identifiers to String when target is String
                        if is_string_type {
                            let should_convert = matches!(
                                value,
                                Expression::Literal {
                                    value: Literal::String(_),
                                    ..
                                } | Expression::Identifier { .. }
                            );
                            if should_convert {
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
                        output.push_str(" = ");
                        if needs_mut_ref {
                            output.push_str("&mut ");
                        }

                        // EXPRESSION CONTEXT: Mark that we're generating a value
                        let old_ctx = self.in_expression_context;
                        self.in_expression_context = true;

                        // WINDJAMMER PHILOSOPHY: Auto-convert mutable string variables
                        // When a mutable variable is initialized with a string literal,
                        // it should be a String (not &str) because &str can't be mutated
                        let mut value_str = self.generate_expression(value);
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

                        output.push_str(&value_str);

                        // Restore expression context
                        self.in_expression_context = old_ctx;
                    }
                }

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

                    // WINDJAMMER PHILOSOPHY: Auto-add .cloned() for HashMap.get() and similar methods
                    // When returning Option<T> but method returns Option<&T>, add .cloned()
                    // Common case: fn get(&self, key: K) -> Option<V> { self.map.get(&key) }
                    let returns_option_owned = self.returns_option_owned_type();
                    if returns_option_owned
                        && self.is_method_returning_option_ref(e)
                        && !return_str.ends_with(".cloned()")
                        && !return_str.ends_with(".clone()")
                    {
                        return_str = format!("{}.cloned()", return_str);
                    }

                    // DOGFOODING FIX: Vec indexing returns &T for non-Copy, but return expects T
                    // e.g. return self.slots[idx] where slots: Vec<SaveSlot> → need .clone()
                    // Use parentheses: (&vec[idx]).clone() - . has higher precedence than &
                    if return_str.starts_with("&") && !return_str.ends_with(".clone()") {
                        let expects_owned = match &self.current_function_return_type {
                            Some(Type::Reference(_)) | Some(Type::MutableReference(_)) => false,
                            _ => true,
                        };
                        if expects_owned {
                            let inner_type = self.infer_expression_type(e).and_then(|t| {
                                match &t {
                                    Type::Reference(inner) => Some(inner.as_ref().clone()),
                                    _ => Some(t),
                                }
                            });
                            if let Some(inner) = inner_type {
                                if !self.is_type_copy(&inner) {
                                    return_str = format!("({}).clone()", return_str);
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

                let old_suppress = self.suppress_string_conversion;
                if any_branch_has_as_str {
                    self.suppress_string_conversion = true;
                }

                let mut output = self.indent();
                output.push_str("if ");
                output.push_str(&self.generate_expression(condition));
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

                self.indent_level += 1;
                output.push_str(&self.generate_block(then_block));
                self.indent_level -= 1;
                self.in_void_block = old_in_void_block;

                output.push_str(&self.indent());
                output.push('}');

                if let Some(else_b) = else_block {
                    output.push_str(" else {\n");
                    self.indent_level += 1;
                    output.push_str(&self.generate_block(else_b));
                    self.indent_level -= 1;
                    output.push_str(&self.indent());
                    output.push('}');
                }

                self.in_function_body = old_in_func_body;

                self.suppress_string_conversion = old_suppress;
                output.push('\n');
                output
            }
            Statement::Match { value, arms, .. } => self.generate_match_statement(value, arms),
            Statement::Loop { body, .. } => {
                let mut output = self.indent();
                output.push_str("loop {\n");

                self.indent_level += 1;
                for stmt in body {
                    output.push_str(&self.generate_statement(stmt));
                }
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

                // Now generate the condition - usize variables already marked
                let condition_str = self.generate_expression(condition);
                output.push_str(&condition_str);
                output.push_str(" {\n");

                self.indent_level += 1;
                for stmt in body {
                    output.push_str(&self.generate_statement(stmt));
                }
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
        eprintln!("TDD DEBUG MATCH START: arms={}, value={:?}", arms.len(), std::mem::discriminant(value));
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
        if arms.len() == 2
            && matches!(arms[1].pattern, Pattern::Wildcard)
            && arms[1].guard.is_none()
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

            let match_binds_refs_early_check = self.match_expression_binds_refs(value);
            let needs_borrow_break_check = match_binds_refs_early_check
                && self.match_scrutinee_is_self_method_call(value)
                && self.match_arms_mutate_self(arms);

            if !needs_borrow_break_check
                && (wildcard_body_is_empty || wildcard_body_stmts.is_some())
            {
                // TDD FIX: Skip redundant .as_str() when value is MethodCall with .as_str()
                // and object is already &str (same logic as in generate_expression_immut)
                eprintln!("TDD DEBUG IF-LET PATH: value is MethodCall? {}", 
                    matches!(value, Expression::MethodCall { .. }));
                let value_str = if let Expression::MethodCall { object, method, arguments, .. } = value {
                    eprintln!("TDD DEBUG IF-LET PATH: MethodCall, method={}", method);
                    if method == "as_str" && arguments.is_empty() {
                        eprintln!("TDD DEBUG IF-LET PATH: .as_str() with no args");
                        let is_already_str = match object {
                            Expression::Identifier { name, .. } => {
                                let result = self.inferred_borrowed_params.contains(name.as_str());
                                eprintln!("TDD DEBUG IF-LET PATH: Identifier {}, borrowed={}, params={:?}",
                                    name, result, self.inferred_borrowed_params);
                                result
                            }
                            Expression::MethodCall { method: inner_method, .. } => {
                                matches!(
                                    inner_method.as_str(),
                                    "as_str" | "trim" | "trim_start" | "trim_end" | 
                                    "strip_prefix" | "strip_suffix"
                                )
                            }
                            _ => false,
                        };
                        if is_already_str {
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

                let match_bound_type_entries: Vec<(String, Type)> =
                    self.infer_match_bound_types(value, &main_arm.pattern);
                for (var_name, var_type) in &match_bound_type_entries {
                    self.local_var_types
                        .insert(var_name.clone(), var_type.clone());
                }

                let mut output = self.indent();
                output.push_str("if let ");
                output.push_str(&self.generate_pattern(&main_arm.pattern));

                if let Some(guard) = &main_arm.guard {
                    output.push_str(" if ");
                    output.push_str(&self.generate_expression(guard));
                }

                output.push_str(" = ");
                output.push_str(&value_str);
                output.push_str(" {\n");

                let has_else = wildcard_body_stmts.is_some();
                let old_in_func_body = self.in_function_body;
                if !has_else {
                    self.in_function_body = false;
                }

                self.indent_level += 1;
                if let Expression::Block { statements, .. } = main_arm.body {
                    output.push_str(&self.generate_block(statements));
                } else {
                    output.push_str(&self.indent());
                    output.push_str(&self.generate_expression(main_arm.body));
                    output.push_str(";\n");
                }
                self.indent_level -= 1;

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
        
        eprintln!("TDD DEBUG MATCH CHECK: has_string_literal={}, is_tuple_match={}", 
            has_string_literal, is_tuple_match);

        // TDD FIX: Skip redundant .as_str() on &str parameters in match expressions
        eprintln!("TDD DEBUG VALUE_STR: value variant={:?}", std::mem::discriminant(value));
        if let Expression::FieldAccess { object, field, .. } = value {
            eprintln!("TDD DEBUG VALUE_STR: FieldAccess, object={:?}, field={}", 
                std::mem::discriminant(*object), field);
        }
        let value_str = if let Expression::MethodCall { object, method, arguments, .. } = value {
            eprintln!("TDD DEBUG VALUE_STR: MethodCall, method={}", method);
            if method == "as_str" && arguments.is_empty() {
                eprintln!("TDD DEBUG VALUE_STR: .as_str() with no args");
                let is_already_str = match object {
                    Expression::Identifier { name, .. } => {
                        let result = self.inferred_borrowed_params.contains(name.as_str());
                        eprintln!("TDD DEBUG VALUE_STR: Identifier {}, borrowed={}, params={:?}",
                            name, result, self.inferred_borrowed_params);
                        result
                    }
                    Expression::MethodCall { method: inner_method, .. } => {
                        matches!(
                            inner_method.as_str(),
                            "as_str" | "trim" | "trim_start" | "trim_end" | 
                            "strip_prefix" | "strip_suffix"
                        )
                    }
                    _ => false,
                };
                eprintln!("TDD DEBUG VALUE_STR: is_already_str={}", is_already_str);
                if is_already_str {
                    eprintln!("TDD DEBUG VALUE_STR: Removing redundant .as_str()");
                    self.generate_expression(object)
                } else {
                    self.generate_expression(value)
                }
            } else {
                self.generate_expression(value)
            }
        } else if let Expression::Call { function, arguments, .. } = value {
            eprintln!("TDD DEBUG: Checking Call expression");
            // Check if this is actually a method call: obj.method()
            if let Expression::FieldAccess { object, field, .. } = &**function {
                eprintln!("TDD DEBUG: Field access, field={}", field);
                if field == "as_str" && arguments.is_empty() {
                    eprintln!("TDD DEBUG: .as_str() call with no args");
                    let is_already_str = match &**object {
                        Expression::Identifier { name, .. } => {
                            let result = self.inferred_borrowed_params.contains(name.as_str());
                            eprintln!("TDD DEBUG: Identifier {}, in borrowed_params={}, params={:?}", 
                                name, result, self.inferred_borrowed_params);
                            result
                        }
                        Expression::Call { function: inner_func, .. } => {
                            // Nested method call like obj.trim().as_str()
                            if let Expression::FieldAccess { field: inner_field, .. } = *inner_func {
                                matches!(
                                    inner_field.as_str(),
                                    "as_str" | "trim" | "trim_start" | "trim_end" | 
                                    "strip_prefix" | "strip_suffix"
                                )
                            } else {
                                false
                            }
                        }
                        _ => false,
                    };
                    if is_already_str {
                        self.generate_expression(object)
                    } else {
                        self.generate_expression(value)
                    }
                } else {
                    self.generate_expression(value)
                }
            } else {
                self.generate_expression(value)
            }
        } else {
            self.generate_expression(value)
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
                eprintln!("TDD DEBUG MATCH ADD: value_str='{}', ends_with_as_str={}", 
                    value_str, value_str.ends_with(".as_str()"));
                eprintln!("TDD DEBUG MATCH ADD: borrowed_params={:?}", self.inferred_borrowed_params);
                
                if !value_str.ends_with(".as_str()") {
                    // Check if the simplified value_str is an identifier that's already &str
                    let is_borrowed_param = self.inferred_borrowed_params.contains(&value_str);
                    eprintln!("TDD DEBUG MATCH ADD: is_borrowed_param={} for value_str='{}'",
                        is_borrowed_param, value_str);
                    
                    if is_borrowed_param {
                        // Already &str, don't add .as_str()
                        eprintln!("TDD DEBUG MATCH ADD: Skipping .as_str() for borrowed param '{}'", value_str);
                        output.push_str(&value_str);
                    } else {
                        // Not &str, add .as_str()
                        eprintln!("TDD DEBUG MATCH ADD: Adding .as_str() to '{}'", value_str);
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

        let needs_string_conversion = match &self.current_function_return_type {
            Some(Type::String) => true,
            Some(Type::Custom(name)) if name == "String" => true,
            _ => arms.iter().any(|arm| {
                string_analysis::expression_produces_string(arm.body)
                    || arm_string_analysis::arm_returns_converted_string(arm.body)
            }),
        };

        let old_in_statement_match = self.in_statement_match;
        let match_is_statement = self.current_function_return_type.is_none();
        if match_is_statement {
            self.in_statement_match = true;
        }

        for arm in arms {
            output.push_str(&self.indent());
            output.push_str(&self.generate_pattern(&arm.pattern));

            if let Some(guard) = &arm.guard {
                output.push_str(" if ");
                output.push_str(&self.generate_expression(guard));
            }

            output.push_str(" => ");

            let mut bound_vars = std::collections::HashSet::new();
            self.extract_pattern_bindings(&arm.pattern, &mut bound_vars);

            let added_borrowed: Vec<String> = if match_binds_refs {
                bound_vars.iter().cloned().collect()
            } else {
                Vec::new()
            };
            for var in &added_borrowed {
                self.borrowed_iterator_vars.insert(var.clone());
            }

            self.local_variable_scopes.push(bound_vars);

            let match_bound_type_entries: Vec<(String, Type)> =
                self.infer_match_bound_types(value, &arm.pattern);
            for (var_name, var_type) in &match_bound_type_entries {
                self.local_var_types
                    .insert(var_name.clone(), var_type.clone());
            }

            let old_in_match_arm = self.in_match_arm_needing_string;
            if needs_string_conversion {
                self.in_match_arm_needing_string = true;
            }

            let mut arm_str = self.generate_expression(arm.body);

            self.in_match_arm_needing_string = old_in_match_arm;

            self.local_variable_scopes.pop();

            for (var_name, _) in &match_bound_type_entries {
                self.local_var_types.remove(var_name);
            }

            for var in &added_borrowed {
                self.borrowed_iterator_vars.remove(var);
            }
            let is_string_literal = matches!(
                &arm.body,
                Expression::Literal {
                    value: Literal::String(_),
                    ..
                }
            );

            if needs_string_conversion && is_string_literal {
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
        let needs_mut = loop_var
            .as_ref()
            .is_some_and(|var| self.loop_body_modifies_variable(body, var));

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

        output.push_str(&self.generate_expression(iterable_to_generate));
        output.push_str(" {\n");

        self.indent_level += 1;

        if is_borrowed_iterator {
            if let Some(var) = &loop_var {
                self.borrowed_iterator_vars.insert(var.clone());
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

        if let Some(var) = &loop_var {
            if let Some(iterable_type) = self.infer_expression_type(iterable) {
                if let Some(elem_type) = Self::extract_iterator_element_type(&iterable_type) {
                    self.local_var_types.insert(var.clone(), elem_type);
                }
            }
        }

        for stmt in body {
            output.push_str(&self.generate_statement(stmt));
        }

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
            output.push_str(&self.generate_expression(target));
            self.generating_assignment_target = false;

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

            let mut value_str = self.generate_expression(value);
            
            // TDD FIX: String += String doesn't work in Rust (needs String += &str)
            // If value returns String and op is Add, add & prefix for borrowing
            if matches!(op, CompoundOp::Add) {
                // Check owned string iterator vars (existing logic)
                if let Expression::Identifier { name, .. } = value {
                    if self.owned_string_iterator_vars.contains(name) {
                        value_str = format!("&{}", value_str);
                    }
                }
                
                // TDD FIX: Check if value expression returns String
                let value_type = self.infer_expression_type(value);
                if matches!(value_type, Some(Type::String)) {
                    // Don't add & for string literals (already &str)
                    let is_string_literal = matches!(
                        value,
                        Expression::Literal { value: Literal::String(_), .. }
                    );
                    // Don't double-borrow if already has &
                    let already_borrowed = value_str.starts_with('&');
                    
                    if !is_string_literal && !already_borrowed {
                        value_str = format!("&{}", value_str);
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
            let is_string_addition = matches!(op, BinaryOp::Add)
                && (right_is_string_like || target_is_string);
            
            let is_known_non_assignable = target_type.as_ref().is_some_and(|t| {
                if let Type::Custom(name) = t {
                    matches!(
                        name.as_str(),
                        "Vec2"
                            | "Vec3"
                            | "Vec4"
                            | "Color"
                            | "Quat"
                            | "Mat3"
                            | "Mat4"
                            | "Point"
                            | "Size"
                    )
                } else {
                    false
                }
            });
            let is_compound_safe = !is_known_non_assignable && !is_string_addition;

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
                    output.push_str(&self.generate_expression(target));
                    self.generating_assignment_target = false;
                    output.push(' ');
                    output.push_str(op_str);
                    output.push(' ');
                    output.push_str(&self.generate_expression(right));
                    output.push_str(";\n");
                    return output;
                }
            }
        }

        self.generating_assignment_target = true;
        output.push_str(&self.generate_expression(target));
        self.generating_assignment_target = false;
        output.push_str(" = ");

        let old_expr_ctx = self.in_expression_context;
        self.in_expression_context = true;

        let mut value_str = self.generate_expression(value);
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
                if let Some(Type::String) = target_type {
                    if !value_str.contains(".clone()") && !value_str.contains(".to_string()") {
                        // WINDJAMMER FIX: &str → String requires .to_string(), not .clone()
                        // .clone() on &str returns &str (via ToOwned trait)
                        // .to_string() converts &str to String (what we need here)
                        value_str = format!("{}.to_string()", value_str);
                    }
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

    fn extract_pattern_bindings(
        &self,
        pattern: &Pattern,
        bindings: &mut std::collections::HashSet<String>,
    ) {
        use crate::parser::EnumPatternBinding;
        match pattern {
            Pattern::Identifier(name) => {
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

    fn match_expression_binds_refs(&self, expr: &Expression) -> bool {
        match expr {
            Expression::Unary {
                op: crate::parser::UnaryOp::Ref | crate::parser::UnaryOp::MutRef,
                ..
            } => true,

            Expression::Identifier { name, .. } => {
                self.inferred_borrowed_params.contains(name.as_str())
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
            Type::Option(inner) => Self::type_contains_reference(inner),
            Type::Result(ok, _err) => Self::type_contains_reference(ok),
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
            Some(Type::Option(inner_type)) => !matches!(**inner_type, Type::Reference(_)),
            _ => false,
        }
    }

    fn is_method_returning_option_ref(&self, expr: &Expression) -> bool {
        match expr {
            Expression::MethodCall { method, .. } => {
                matches!(method.as_str(), "get" | "first" | "last")
            }
            Expression::Call { .. } => false,
            _ => false,
        }
    }
}
