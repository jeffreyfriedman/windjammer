//! Parameter ownership helpers: usage paths through statements/expressions and type compatibility.

use std::collections::HashSet;

use crate::parser::*;

use super::{Analyzer, OwnershipMode, SignatureRegistry};

impl<'ast> Analyzer<'ast> {
    pub(crate) fn infer_parameter_ownership(
        &self,
        param_name: &str,
        param_type: &Type,
        body: &[&'ast Statement<'ast>],
        return_type: &Option<Type>,
        registry: &SignatureRegistry,
        current_func_name: &str,
        func: &FunctionDecl<'ast>,
    ) -> Result<OwnershipMode, String> {
        // 0a. Generic type parameters and impl Trait always stay Owned.
        // Adding & would change trait bounds: `impl Foo` -> `&impl Foo` breaks dispatch.
        // Generic types like T, G, S should always be passed by value.
        if Self::is_generic_type_param(param_type) {
            return Ok(OwnershipMode::Owned);
        }

        // 0b. Explicit Rust type `String` (not Windjammer `string`) stays Owned.
        // When user writes `path: String`, they're explicitly requesting Rust's String type,
        // which should be respected as owned. Do NOT infer it as borrowed.
        // This is different from `path: string` (lowercase), which can infer to &str.
        if matches!(param_type, Type::Custom(name) if name == "String") {
            return Ok(OwnershipMode::Owned);
        }

        // 0c. Return-type-aware ownership: When return type contains param type, we need Owned.
        // Bug: save_migration.wj - migrate(data) -> Result<GameSaveData, string> was inferring
        // &GameSaveData because we only read data fields. But we assign to current_data and
        // return that - we need to own the input to produce the output.
        // Handles: fn(T) -> T, fn(T) -> Result<T,E>, fn(T) -> Option<T>
        //
        // TDD FIX: Skip when param is ONLY used as &param (e.g., a + &b + &c).
        // For concatenate(a, b, c) -> a + &b + &c, b and c are borrowed, not consumed.
        // param_type_matches_return would incorrectly infer Owned for all string params.
        if !self.is_only_used_as_borrow(param_name, body) {
            if let Some(return_type) = return_type {
                if self.param_type_matches_return(param_type, return_type) {
                    // Windjammer `string` / `str` parameters: return type also being string-like
                    // does NOT mean the parameter is consumed into the return value.
                    // Example: find_translation(lang, key: string) -> string only compares `key`;
                    // inferring Owned for `key` breaks callers that pass the same String twice (E0382).
                    // Non-string types keep the broader rule (transform/migrate still get Owned).
                    let string_like = matches!(param_type, Type::String);
                    if self.is_returned(param_name, body)
                        || self.is_stored(param_name, body)
                    {
                        return Ok(OwnershipMode::Owned);
                    } else if !string_like
                        && self.param_is_consumed_into_return(param_name, body)
                    {
                        return Ok(OwnershipMode::Owned);
                    }
                }
            }
        }

        // WINDJAMMER DESIGN: String parameters infer to &str (not &String!)
        //
        // When a string parameter is read-only, we generate `&str` (idiomatic Rust):
        // - Accepts both `String` and `&str` via deref coercion
        // - No `&String` anti-pattern (Clippy-approved)
        // - Zero-cost for read-only access
        //
        // Strings are treated like any other type in ownership inference.
        // The codegen layer will emit `&str` for Borrowed String parameters.

        // Multi-pass registry-aware inference

        // 1. Check if parameter is mutated (uses registry for method call detection)
        if self.is_mutated(param_name, body, registry) {
            return Ok(OwnershipMode::MutBorrowed);
        }

        // 2. Check if parameter is returned (escapes function)
        if self.is_returned(param_name, body) {
            return Ok(OwnershipMode::Owned);
        }

        // 2.1. Returning a non-Copy self field (e.g. `fn id(self) -> String { self.id }`)
        // WINDJAMMER FIX: Do NOT make self Owned just because a non-Copy field is returned.
        // Instead, keep self as Borrowed (&self) and the codegen will auto-clone the
        // returned field. This prevents cascading E0382 errors at callsites where the
        // caller uses the object again after calling a getter.
        // The old behavior (Owned self for getters) was correct Rust semantics but bad
        // Windjammer ergonomics -- every caller had to .clone() the object before calling
        // a simple getter.

        // 2.3. WINDJAMMER FIX: Check if parameter is used in if/else expression
        // When a parameter appears in an if/else that's assigned or returned,
        // it needs to be owned to match the other branch's ownership
        if self.is_used_in_if_else_expression(param_name, body) {
            return Ok(OwnershipMode::Owned);
        }

        // THE WINDJAMMER WAY: Removed aggressive string optimization
        // When a user writes `text: string`, they mean `String` (owned), period.
        // Do NOT auto-convert to `&str` just because it's "only passed to read-only functions".
        // That breaks API contracts and causes confusing type errors.
        //
        // Smart inference is OFF for explicit type annotations.
        // (Future: Could add #[optimize] annotation for user-requested optimization)

        // THE WINDJAMMER WAY: Removed aggressive string optimization
        // When a user writes `text: string`, they mean `String` (owned), period.
        // Do NOT auto-convert to `&str` just because it's "only passed to read-only functions".
        // That breaks API contracts and causes confusing type errors.
        //
        // Smart inference is OFF for explicit type annotations.
        // (Future: Could add #[optimize] annotation for user-requested optimization)

        // 3. Check if parameter is stored in a struct or collection
        if self.is_stored(param_name, body) {
            return Ok(OwnershipMode::Owned);
        }

        // 4. Check if parameter is used in arithmetic binary operations (for Copy types)
        // TDD FIX (Bug #5): Comparison operators (==, !=, <, >, <=, >=) work with borrowed
        // values, so we should only force Owned for arithmetic operations (Add, Sub, Mul, Div).
        if self.is_used_in_arithmetic_op(param_name, body) {
            return Ok(OwnershipMode::Owned);
        }

        // 5. Check if parameter is pattern matched with field extraction
        if self.is_pattern_matched_with_fields(param_name, body) {
            return Ok(OwnershipMode::Owned);
        }

        // 6. TDD: Check if parameter is iterated over in a for loop
        if self.is_iterated_over(param_name, body) {
            return Ok(OwnershipMode::Owned);
        }

        // 6b. TDD: Check if parameter calls a method that takes `self` by value (consuming).
        // When `a.as_float()` is called and `as_float` takes owned `self`, `a` is consumed
        // and must be owned. Without this check, `a` defaults to Borrowed (&a), producing
        // E0507 errors because you can't move out of a shared reference.
        if self.calls_consuming_method(param_name, body, registry) {
            return Ok(OwnershipMode::Owned);
        }

        // 7. MULTI-PASS OWNERSHIP INFERENCE (The Proper Solution!)
        // Check if parameter is passed as an argument to another function/method.
        // Look up the callee's signature in the registry and match the ownership mode.
        //
        // THE WINDJAMMER WAY: The compiler does the work, not the user.
        // - Pass 1: No registry yet → infer from local usage (comparisons → Borrowed)
        // - Pass 2+: Look up callee signatures → match their ownership
        // - Convergence: Ownership propagates until stable
        //
        // Example: fn wrapper(id: string) { has_item(id) }
        // - Pass 1: has_item doesn't exist, id only used in pass-through → Borrowed
        // - Pass 2: has_item exists with id: &String → wrapper matches Borrowed
        // - Pass 3: No changes → CONVERGED ✅
        //
        // IMPORTANT: Only use registry if callees expect stricter ownership than local usage
        if let Some(pass_through_mode) = self.infer_passthrough_ownership(
            param_name,
            param_type,
            body,
            registry,
            current_func_name,
            func,
        ) {
            match pass_through_mode {
                OwnershipMode::Borrowed => return Ok(OwnershipMode::Borrowed),
                OwnershipMode::MutBorrowed => return Ok(OwnershipMode::MutBorrowed),
                OwnershipMode::Owned => {
                    return Ok(OwnershipMode::Owned);
                }
            }
        }

        // 8. Default ownership: Borrowed (THE WINDJAMMER WAY!)
        //
        // **PHILOSOPHY**: The compiler does the work, not the user.
        // - Default to **Borrowed** for read-only parameters
        // - The checks above handle all consuming cases (mutated, returned,
        //   stored, iterated, used in binary ops, pattern matched)
        // - If none of those apply, the parameter is truly read-only
        // - Read-only non-Copy parameters should be &T in generated Rust
        // - Copy types are overridden to Owned in build_signature
        //
        // This matches the Windjammer philosophy: users write `data: Vec<f32>`
        // and the compiler infers `&Vec<f32>` when data is only read.
        // Call sites naturally pass `&self.data` which matches `&Vec<f32>`.
        //
        // Dogfooding evidence: 6+ E0308 errors in windjammer-game-editor
        // from read-only params generating owned types while call sites pass &T.
        Ok(OwnershipMode::Borrowed)
    }

    /// TDD: Check if parameter is ONLY used as &param or &mut param (never consumed directly).
    /// Example: a + &b + &c - b and c are only used as &b, &c → true for b and c.
    /// Used to avoid param_type_matches_return incorrectly inferring Owned for string params
    /// in concatenation: fn(a, b, c) -> a + &b + &c.
    pub(crate) fn is_only_used_as_borrow(&self, param_name: &str, body: &[&'ast Statement<'ast>]) -> bool {
        for stmt in body {
            if !self.stmt_param_only_borrowed(param_name, stmt, false) {
                return false;
            }
        }
        true
    }

    pub(crate) fn stmt_param_only_borrowed(
        &self,
        param_name: &str,
        stmt: &Statement,
        _inside_ref: bool,
    ) -> bool {
        match stmt {
            Statement::Let { value, .. } => self.expr_param_only_borrowed(param_name, value, false),
            Statement::Return { value, .. } => value
                .as_ref()
                .is_none_or(|e| self.expr_param_only_borrowed(param_name, e, false)),
            Statement::Expression { expr, .. } => {
                self.expr_param_only_borrowed(param_name, expr, false)
            }
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.expr_param_only_borrowed(param_name, condition, false)
                    && then_block
                        .iter()
                        .all(|s| self.stmt_param_only_borrowed(param_name, s, false))
                    && else_block.as_ref().is_none_or(|b| {
                        b.iter()
                            .all(|s| self.stmt_param_only_borrowed(param_name, s, false))
                    })
            }
            Statement::While {
                condition, body, ..
            } => {
                self.expr_param_only_borrowed(param_name, condition, false)
                    && body
                        .iter()
                        .all(|s| self.stmt_param_only_borrowed(param_name, s, false))
            }
            Statement::For { iterable, body, .. } => {
                self.expr_param_only_borrowed(param_name, iterable, false)
                    && body
                        .iter()
                        .all(|s| self.stmt_param_only_borrowed(param_name, s, false))
            }
            _ => true,
        }
    }

    pub(crate) fn expr_param_only_borrowed(
        &self,
        param_name: &str,
        expr: &Expression,
        inside_ref: bool,
    ) -> bool {
        match expr {
            Expression::Identifier { name, .. } if name == param_name => {
                // Found param: must be inside & or &mut to be "only borrowed"
                inside_ref
            }
            Expression::Unary {
                op: crate::parser::UnaryOp::Ref | crate::parser::UnaryOp::MutRef,
                operand,
                ..
            } => self.expr_param_only_borrowed(param_name, operand, true),
            Expression::Binary { left, right, .. } => {
                self.expr_param_only_borrowed(param_name, left, false)
                    && self.expr_param_only_borrowed(param_name, right, false)
            }
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                self.expr_param_only_borrowed(param_name, function, false)
                    && arguments
                        .iter()
                        .all(|(_, a)| self.expr_param_only_borrowed(param_name, a, false))
            }
            Expression::MethodCall {
                object, arguments, ..
            } => {
                self.expr_param_only_borrowed(param_name, object, false)
                    && arguments
                        .iter()
                        .all(|(_, a)| self.expr_param_only_borrowed(param_name, a, false))
            }
            Expression::FieldAccess { object, .. } => {
                self.expr_param_only_borrowed(param_name, object, false)
            }
            Expression::Index { object, index, .. } => {
                self.expr_param_only_borrowed(param_name, object, false)
                    && self.expr_param_only_borrowed(param_name, index, false)
            }
            Expression::Block { statements, .. } => statements
                .iter()
                .all(|s| self.stmt_param_only_borrowed(param_name, s, false)),
            Expression::Tuple { elements, .. } | Expression::Array { elements, .. } => elements
                .iter()
                .all(|e| self.expr_param_only_borrowed(param_name, e, false)),
            Expression::StructLiteral { fields, .. } => fields
                .iter()
                .all(|(_, v)| self.expr_param_only_borrowed(param_name, v, false)),
            Expression::TryOp { expr, .. } => {
                self.expr_param_only_borrowed(param_name, expr, false)
            }
            _ => true,
        }
    }

    pub(crate) fn is_used_in_if_else_expression(
        &self,
        name: &str,
        statements: &[&'ast Statement<'ast>],
    ) -> bool {
        // Check if parameter is used in an if/else expression
        // Example:
        //   let x = if cond { Thing::new(...) } else { param }
        //
        // When param is in an if/else that gets assigned or returned,
        // it needs to be owned to match the other branch's type

        for stmt in statements {
            if self.stmt_has_if_else_with_param(name, stmt) {
                return true;
            }
        }
        false
    }

    pub(crate) fn stmt_has_if_else_with_param(&self, name: &str, stmt: &Statement) -> bool {
        match stmt {
            Statement::Let { value, .. } => {
                // let x = if ... { ... } else { param }
                self.expr_is_if_else_with_param(name, value)
            }
            Statement::Return {
                value: Some(expr), ..
            } => {
                // return if ... { ... } else { param }
                self.expr_is_if_else_with_param(name, expr)
            }
            Statement::Expression { expr, .. } => {
                // Implicit return or assignment
                self.expr_is_if_else_with_param(name, expr)
            }
            Statement::If {
                then_block,
                else_block,
                ..
            } => {
                // Check nested if statements
                self.stmts_have_if_else_with_param(name, then_block)
                    || else_block
                        .as_ref()
                        .is_some_and(|block| self.stmts_have_if_else_with_param(name, block))
            }
            _ => false,
        }
    }

    pub(crate) fn stmts_have_if_else_with_param(&self, name: &str, stmts: &[&'ast Statement<'ast>]) -> bool {
        stmts
            .iter()
            .any(|stmt| self.stmt_has_if_else_with_param(name, stmt))
    }

    pub(crate) fn expr_is_if_else_with_param(&self, name: &str, expr: &Expression) -> bool {
        match expr {
            Expression::Block { statements, .. } => {
                // Check if block contains an if statement with the parameter
                for stmt in statements {
                    if let Statement::If {
                        then_block,
                        else_block,
                        ..
                    } = stmt
                    {
                        let in_then = self.stmts_mention_identifier(name, then_block);
                        let in_else = else_block
                            .as_ref()
                            .is_some_and(|block| self.stmts_mention_identifier(name, block));
                        if in_then || in_else {
                            return true;
                        }
                    }
                }
                false
            }
            _ => false,
        }
    }

    pub(crate) fn stmts_mention_identifier(&self, name: &str, stmts: &[&'ast Statement<'ast>]) -> bool {
        stmts
            .iter()
            .any(|stmt| self.stmt_mentions_identifier(name, stmt))
    }

    pub(crate) fn stmt_mentions_identifier(&self, name: &str, stmt: &Statement) -> bool {
        match stmt {
            Statement::Expression { expr, .. } => self.expr_mentions_identifier(name, expr),
            Statement::Return {
                value: Some(expr), ..
            } => self.expr_mentions_identifier(name, expr),
            Statement::Let { value, .. } => self.expr_mentions_identifier(name, value),
            _ => false,
        }
    }

    pub(crate) fn expr_mentions_identifier(&self, name: &str, expr: &Expression) -> bool {
        match expr {
            Expression::Identifier { name: id, .. } => id == name,
            Expression::FieldAccess { object, .. } => self.expr_mentions_identifier(name, object),
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                self.expr_mentions_identifier(name, function)
                    || arguments
                        .iter()
                        .any(|(_, arg)| self.expr_mentions_identifier(name, arg))
            }
            Expression::Binary { left, right, .. } => {
                self.expr_mentions_identifier(name, left)
                    || self.expr_mentions_identifier(name, right)
            }
            Expression::Unary { operand, .. } => self.expr_mentions_identifier(name, operand),
            Expression::TryOp { expr, .. } => self.expr_mentions_identifier(name, expr),
            _ => false,
        }
    }
    pub(crate) fn is_returned(&self, name: &str, statements: &[&'ast Statement<'ast>]) -> bool {
        let len = statements.len();
        for (i, stmt) in statements.iter().enumerate() {
            let is_last = i == len - 1;
            match stmt {
                Statement::Return {
                    value: Some(expr), ..
                } => {
                    // Check if parameter is returned directly or wrapped in Some/Ok/Err/tuple
                    if self.expression_uses_identifier_for_return(name, expr) {
                        return true;
                    }
                }
                // CRITICAL: Handle implicit returns (last expression without semicolon)
                // In Windjammer/Rust, the last expression in a block is the return value
                Statement::Expression { expr, .. } if is_last => {
                    // Skip ONLY void-returning function calls (like println)
                    // Wrapper calls (Some, Ok, Err) DO return their arguments!
                    let is_void_call = if let Expression::Call { function, .. } = expr {
                        if let Expression::Identifier { name: fn_name, .. } = &**function {
                            matches!(
                                fn_name.as_str(),
                                "println" | "print" | "eprintln" | "eprint" | "assert" | "panic"
                            )
                        } else {
                            false
                        }
                    } else {
                        false
                    };

                    if !is_void_call && self.expression_uses_identifier_for_return(name, expr) {
                        return true;
                    }
                }
                Statement::If {
                    then_block,
                    else_block,
                    ..
                } => {
                    if self.is_returned(name, then_block) {
                        return true;
                    }
                    if let Some(else_b) = else_block {
                        if self.is_returned(name, else_b) {
                            return true;
                        }
                    }
                }
                // CRITICAL: Handle match expressions where parameter is returned in arms
                Statement::Match { arms, .. } => {
                    for arm in arms {
                        if self.expression_uses_identifier_for_return(name, arm.body) {
                            return true;
                        }
                    }
                }
                _ => {}
            }
        }
        false
    }

    /// Check if an expression uses a parameter in a way that requires ownership for return.
    /// This includes direct use, wrapping in Some/Ok/Err, tuples, etc.
    pub(crate) fn expression_uses_identifier_for_return(&self, name: &str, expr: &Expression) -> bool {
        match expr {
            // Direct identifier use
            Expression::Identifier { name: id, .. } if id == name => true,

            // Wrapped in constructors: Some(param), Ok(param), Err(param), Enum::Variant(param)
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                if let Expression::Identifier { name: fn_name, .. } = &**function {
                    let is_known_wrapper = matches!(fn_name.as_str(), "Some" | "Ok" | "Err");
                    let is_enum_constructor = Self::looks_like_enum_variant_constructor(fn_name);

                    if is_known_wrapper || is_enum_constructor {
                        for (_label, arg) in arguments {
                            if self.expression_uses_identifier(name, arg) {
                                return true;
                            }
                        }
                    }
                }
                false
            }

            // Tuple expression: (a, b, c)
            Expression::Tuple { elements, .. } => {
                for elem in elements {
                    if self.expression_uses_identifier(name, elem) {
                        return true;
                    }
                }
                false
            }

            // CRITICAL FIX: Binary expressions (comparisons, arithmetic) return the RESULT, not the parameter
            // Example: `id == "test"` returns bool, NOT id
            // Example: `id + 1` returns the sum, NOT id
            // The parameter is only being READ, not returned
            Expression::Binary { .. } => false,

            // Unary expressions also return the result, not the operand
            Expression::Unary { .. } => false,

            // Default: reject (conservative - only allow explicit cases above)
            _ => false,
        }
    }

    /// Check if an expression stores a parameter by value.
    /// Matches direct identifier use, wrapping in Some/Ok/Err, enum variant constructors,
    /// tuples, and struct literals containing the parameter.
    pub(crate) fn expression_stores_identifier(&self, name: &str, expr: &Expression) -> bool {
        match expr {
            Expression::Identifier { name: id, .. } => id == name,
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                if let Expression::Identifier { name: fn_name, .. } = &**function {
                    let is_constructor =
                        matches!(fn_name.as_str(), "Some" | "Ok" | "Err") || fn_name.contains("::");
                    if is_constructor {
                        return arguments
                            .iter()
                            .any(|(_label, arg)| self.expression_stores_identifier(name, arg));
                    }
                }
                false
            }
            Expression::Tuple { elements, .. } => elements
                .iter()
                .any(|el| self.expression_stores_identifier(name, el)),
            Expression::StructLiteral { fields, .. } => fields
                .iter()
                .any(|(_, v)| self.expression_stores_identifier(name, v)),
            Expression::Array { elements, .. } => elements
                .iter()
                .any(|el| self.expression_stores_identifier(name, el)),
            _ => false,
        }
    }

    pub(crate) fn param_is_consumed_into_return(
        &self,
        param_name: &str,
        body: &[&'ast Statement<'ast>],
    ) -> bool {
        for stmt in body {
            match stmt {
                Statement::Let {
                    pattern: Pattern::Identifier(var_name),
                    value,
                    ..
                } => {
                    if self.expression_uses_identifier(param_name, value) {
                        if self.is_returned(var_name, body) {
                            return true;
                        }
                    }
                }
                Statement::Assignment { value, .. } => {
                    if self.expression_uses_identifier(param_name, value) {
                        return true;
                    }
                }
                Statement::If {
                    then_block,
                    else_block,
                    ..
                } => {
                    if self.param_is_consumed_into_return(param_name, then_block) {
                        return true;
                    }
                    if let Some(else_b) = else_block {
                        if self.param_is_consumed_into_return(param_name, else_b) {
                            return true;
                        }
                    }
                }
                Statement::Match { arms, .. } => {
                    for arm in arms {
                        if let Expression::Block { statements, .. } = arm.body {
                            if self.param_is_consumed_into_return(param_name, statements) {
                                return true;
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        false
    }

    pub(crate) fn is_stored(&self, name: &str, statements: &[&'ast Statement<'ast>]) -> bool {
        // Check if the parameter is stored in a struct field or collection
        for stmt in statements {
            match stmt {
                Statement::Let {
                    value: Expression::StructLiteral { fields, .. },
                    ..
                } => {
                    for (_field_name, field_expr) in fields {
                        if self.expression_uses_identifier(name, field_expr) {
                            return true;
                        }
                    }
                }
                Statement::Return {
                    value: Some(Expression::StructLiteral { fields, .. }),
                    ..
                } => {
                    // Check if parameter is used in a returned struct literal
                    for (_, field_expr) in fields {
                        if self.expression_uses_identifier(name, field_expr) {
                            return true;
                        }
                    }
                }
                Statement::Expression {
                    expr: Expression::StructLiteral { fields, .. },
                    ..
                } => {
                    // Check if parameter is used in a struct literal expression (implicit return)
                    for (_, field_expr) in fields {
                        if self.expression_uses_identifier(name, field_expr) {
                            return true;
                        }
                    }
                }
                Statement::Assignment {
                    target: Expression::FieldAccess { object, .. },
                    value,
                    ..
                } => {
                    // Check if the parameter is assigned to a struct field, either directly
                    // or wrapped in Some/Enum constructors/tuples.
                    //
                    // Direct: obj.field = param
                    // Wrapped: obj.field = Some(param)
                    // Enum: obj.field = Enum::Variant(param)
                    if matches!(&**object, Expression::Identifier { .. }) {
                        if self.expression_stores_identifier(name, value) {
                            return true;
                        }
                    }
                }
                // Check if parameter is stored via index assignment
                // e.g., self.slots[i] = item
                // e.g., self.slots[i] = Some(ItemStack::new(item, qty))
                Statement::Assignment {
                    target: Expression::Index { .. },
                    value,
                    ..
                } => {
                    if self.expression_stores_identifier(name, value) {
                        return true;
                    }
                }
                Statement::Expression {
                    expr:
                        Expression::MethodCall {
                            object,
                            method,
                            arguments,
                            ..
                        },
                    ..
                } => {
                    let is_storage_method = crate::method_registry::is_storage_method(method);

                    if is_storage_method {
                        // Check for storage method calls on ANY object:
                        // - self.field.push(param)
                        // - self.field.push((param, other))  ← tuple wrapping
                        // - self.field.push(Enum::Variant(param))  ← enum wrapping
                        // - local_var.push(param)
                        let is_on_field_or_var =
                            matches!(&**object, Expression::FieldAccess { .. })
                                || matches!(&**object, Expression::Identifier { .. });

                        if is_on_field_or_var {
                            for (_label, arg) in arguments {
                                if self.expression_stores_identifier(name, arg) {
                                    return true;
                                }
                            }
                        }

                        // TDD FIX: Also check for method calls on LOCAL struct fields: local_var.field.push(param)
                        // e.g., choice.conditions.push(condition) where choice is a local variable
                        if let Expression::FieldAccess {
                            object: field_obj, ..
                        } = &**object
                        {
                            // Check if it's a local variable (not self)
                            if matches!(&**field_obj, Expression::Identifier { name: id, .. } if id != "self")
                            {
                                for (_label, arg) in arguments {
                                    if matches!(arg, Expression::Identifier { name: id, .. } if id == name)
                                    {
                                        return true;
                                    }
                                }
                            }
                        }
                    }

                    // Also check for method calls on local variables: props.push(Property { name, ... })
                    // The parameter might be used in a struct literal passed as an argument
                    for (_label, arg) in arguments {
                        if let Expression::StructLiteral { fields, .. } = arg {
                            for (_field_name, field_expr) in fields {
                                if self.expression_uses_identifier(name, field_expr) {
                                    return true;
                                }
                            }
                        }
                    }

                    // Check for push/insert with a constructor call: vec.push(Node::new(param, ...))
                    // The parameter is being stored if passed to a constructor that stores it
                    if is_storage_method {
                        for (_label, arg) in arguments {
                            if let Expression::Call {
                                arguments: call_args,
                                ..
                            } = arg
                            {
                                for (_call_label, call_arg) in call_args {
                                    if matches!(call_arg, Expression::Identifier { name: id, .. } if id == name)
                                    {
                                        return true;
                                    }
                                }
                            }
                        }
                    }
                }
                // Recursively check if/else bodies for storage operations
                Statement::If {
                    then_block,
                    else_block,
                    ..
                } => {
                    if self.is_stored(name, then_block) {
                        return true;
                    }
                    if let Some(else_stmts) = else_block {
                        if self.is_stored(name, else_stmts) {
                            return true;
                        }
                    }
                }
                // Recursively check loop bodies
                Statement::While { body, .. } | Statement::For { body, .. } => {
                    if self.is_stored(name, body) {
                        return true;
                    }
                }
                // General case: check any statement for enum variant constructors
                // that consume the parameter. Covers patterns like:
                //   let x = Func(EnumType::Variant(param, ...))
                //   let x = Func(format!(..., param), &EnumType::Variant(param, ...))
                _ => {
                    if self.stmt_has_enum_variant_consuming(name, stmt) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Check if a statement contains an enum variant constructor that consumes a parameter.
    /// Recursively scans all expressions within the statement.
    pub(crate) fn stmt_has_enum_variant_consuming(&self, name: &str, stmt: &Statement<'ast>) -> bool {
        match stmt {
            Statement::Let { value, .. } => self.expr_has_enum_variant_consuming(name, value),
            Statement::Expression { expr, .. } => self.expr_has_enum_variant_consuming(name, expr),
            Statement::Return {
                value: Some(expr), ..
            } => self.expr_has_enum_variant_consuming(name, expr),
            Statement::Assignment { value, .. } => {
                self.expr_has_enum_variant_consuming(name, value)
            }
            _ => false,
        }
    }

    /// Recursively check if an expression contains an enum variant constructor
    /// (function call where name contains "::") that has the parameter as a direct argument.
    pub(crate) fn expr_has_enum_variant_consuming(&self, name: &str, expr: &Expression<'ast>) -> bool {
        match expr {
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                let is_enum_variant = if let Expression::Identifier { name: fn_name, .. } = function
                {
                    Self::looks_like_enum_variant_constructor(fn_name)
                } else if let Expression::FieldAccess { field, .. } = function {
                    Self::looks_like_enum_variant_constructor(field)
                } else {
                    false
                };

                if is_enum_variant {
                    for (_label, arg) in arguments {
                        if matches!(arg, Expression::Identifier { name: id, .. } if id == name) {
                            return true;
                        }
                    }
                }

                // Recurse into all arguments
                for (_label, arg) in arguments {
                    if self.expr_has_enum_variant_consuming(name, arg) {
                        return true;
                    }
                }
                // Recurse into function expression
                self.expr_has_enum_variant_consuming(name, function)
            }
            Expression::Unary { operand, .. } => {
                self.expr_has_enum_variant_consuming(name, operand)
            }
            Expression::Block { statements, .. } => {
                for s in statements {
                    if self.stmt_has_enum_variant_consuming(name, s) {
                        return true;
                    }
                }
                false
            }
            Expression::Tuple { elements, .. } => {
                for el in elements {
                    if self.expr_has_enum_variant_consuming(name, el) {
                        return true;
                    }
                }
                false
            }
            _ => false,
        }
    }

    /// Check if a qualified name like "Type::Variant" looks like an enum variant constructor
    /// rather than a static method call. Enum variants use PascalCase after "::"
    /// (e.g., Option::Some, Color::Custom), while methods use snake_case
    /// (e.g., FpsCamera::collides_aabb, Vec3::new).
    pub(crate) fn looks_like_enum_variant_constructor(qualified_name: &str) -> bool {
        if let Some(pos) = qualified_name.rfind("::") {
            let after_colons = &qualified_name[pos + 2..];
            after_colons
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_uppercase())
        } else {
            false
        }
    }

    /// TDD: Check if a parameter is iterated over in a for loop (consumed by iteration)
    /// e.g., `for item in items` (not `for item in &items`)
    /// When you iterate over a Vec without `&`, the Vec is consumed and elements are moved.
    pub(crate) fn is_iterated_over(&self, name: &str, statements: &[&'ast Statement<'ast>]) -> bool {
        for stmt in statements {
            match stmt {
                Statement::For { iterable, body, .. } => {
                    // Check if the iterable is exactly the parameter (direct iteration)
                    if let Expression::Identifier { name: id, .. } = iterable {
                        if id == name {
                            return true;
                        }
                    }

                    // Recursively check nested for loops
                    if self.is_iterated_over(name, body) {
                        return true;
                    }
                }
                Statement::If {
                    then_block,
                    else_block,
                    ..
                } => {
                    if self.is_iterated_over(name, then_block) {
                        return true;
                    }
                    if let Some(else_stmts) = else_block {
                        if self.is_iterated_over(name, else_stmts) {
                            return true;
                        }
                    }
                }
                Statement::While { body, .. } | Statement::Loop { body, .. } => {
                    if self.is_iterated_over(name, body) {
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }

    /// TDD: Check if a parameter calls a method that takes `self` by value (consuming).
    /// When `a.as_float()` is called and `as_float` takes owned `self`, `a` is consumed.
    /// This requires looking up the method's signature in the registry.
    pub(crate) fn calls_consuming_method(
        &self,
        param_name: &str,
        statements: &[&'ast Statement<'ast>],
        registry: &SignatureRegistry,
    ) -> bool {
        for stmt in statements {
            if self.stmt_calls_consuming_method(param_name, stmt, registry) {
                return true;
            }
        }
        false
    }

    pub(crate) fn stmt_calls_consuming_method(
        &self,
        param_name: &str,
        stmt: &Statement<'ast>,
        registry: &SignatureRegistry,
    ) -> bool {
        match stmt {
            Statement::Expression { expr, .. } => {
                self.expr_calls_consuming_method(param_name, expr, registry)
            }
            Statement::Let { value, .. } => {
                self.expr_calls_consuming_method(param_name, value, registry)
            }
            Statement::Return {
                value: Some(expr), ..
            } => self.expr_calls_consuming_method(param_name, expr, registry),
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                if self.expr_calls_consuming_method(param_name, condition, registry) {
                    return true;
                }
                if self.calls_consuming_method(param_name, then_block, registry) {
                    return true;
                }
                if let Some(else_b) = else_block {
                    if self.calls_consuming_method(param_name, else_b, registry) {
                        return true;
                    }
                }
                false
            }
            Statement::While {
                condition, body, ..
            } => {
                self.expr_calls_consuming_method(param_name, condition, registry)
                    || self.calls_consuming_method(param_name, body, registry)
            }
            Statement::Loop { body, .. } => {
                self.calls_consuming_method(param_name, body, registry)
            }
            Statement::For { body, .. } => {
                self.calls_consuming_method(param_name, body, registry)
            }
            Statement::Match { value, arms, .. } => {
                if self.expr_calls_consuming_method(param_name, value, registry) {
                    return true;
                }
                for arm in arms {
                    if self.expr_calls_consuming_method(param_name, arm.body, registry) {
                        return true;
                    }
                }
                false
            }
            _ => false,
        }
    }

    pub(crate) fn expr_calls_consuming_method(
        &self,
        param_name: &str,
        expr: &Expression<'ast>,
        registry: &SignatureRegistry,
    ) -> bool {
        match expr {
            Expression::MethodCall {
                object,
                method,
                arguments,
                ..
            } => {
                // Check if param is the direct receiver of a consuming method
                if self.is_direct_receiver(param_name, object) {
                    if let Some(sig) = registry.get_signature(method) {
                        if sig.has_self_receiver {
                            if let Some(mode) = sig.param_ownership.first() {
                                if matches!(mode, OwnershipMode::Owned) {
                                    return true;
                                }
                            }
                        }
                    }
                }
                // Check if param is passed as an owned argument to the method
                if let Some(sig) = registry.get_signature(method) {
                    let param_offset = if sig.has_self_receiver { 1 } else { 0 };
                    for (i, (_, arg)) in arguments.iter().enumerate() {
                        if matches!(arg, Expression::Identifier { name, .. } if name == param_name)
                        {
                            let sig_idx = i + param_offset;
                            if let Some(mode) = sig.param_ownership.get(sig_idx) {
                                if matches!(mode, OwnershipMode::Owned) {
                                    return true;
                                }
                            }
                        }
                    }
                }
                // Recurse into arguments and object
                if self.expr_calls_consuming_method(param_name, object, registry) {
                    return true;
                }
                for (_, arg) in arguments {
                    if self.expr_calls_consuming_method(param_name, arg, registry) {
                        return true;
                    }
                }
                false
            }
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                // Handle Call(FieldAccess) pattern: param.method(args)
                if let Expression::FieldAccess { object, field, .. } = &**function {
                    if self.is_direct_receiver(param_name, object) {
                        if let Some(sig) = registry.get_signature(field) {
                            if sig.has_self_receiver {
                                if let Some(mode) = sig.param_ownership.first() {
                                    if matches!(mode, OwnershipMode::Owned) {
                                        return true;
                                    }
                                }
                            }
                        }
                    }
                }
                // Extract function name for signature lookup
                let func_name = match &**function {
                    Expression::Identifier { name, .. } => Some(name.as_str()),
                    Expression::FieldAccess { object: obj, field, .. } => {
                        if let Expression::Identifier { name: _, .. } = &**obj {
                            None // Will try qualified name below
                        } else {
                            Some(field.as_str())
                        }
                    }
                    _ => None,
                };
                // Check if param is passed as an owned argument
                let mut names: Vec<&str> = Vec::new();
                if let Some(n) = func_name {
                    names.push(n);
                }
                if let Expression::FieldAccess { object: obj, field, .. } = &**function {
                    if let Expression::Identifier { name, .. } = &**obj {
                        // For qualified calls like Type::method(param)
                        // We can't easily push a formatted string as &str, so just check directly
                        let qualified = format!("{}::{}", name, field);
                        if let Some(sig) = registry.get_signature(&qualified) {
                            let param_offset = if sig.has_self_receiver { 1 } else { 0 };
                            for (i, (_, arg)) in arguments.iter().enumerate() {
                                if matches!(arg, Expression::Identifier { name, .. } if name == param_name) {
                                    let sig_idx = i + param_offset;
                                    if let Some(mode) = sig.param_ownership.get(sig_idx) {
                                        if matches!(mode, OwnershipMode::Owned) {
                                            return true;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                for name in &names {
                    if let Some(sig) = registry.get_signature(name) {
                        let param_offset = if sig.has_self_receiver { 1 } else { 0 };
                        for (i, (_, arg)) in arguments.iter().enumerate() {
                            if matches!(arg, Expression::Identifier { name, .. } if name == param_name) {
                                let sig_idx = i + param_offset;
                                if let Some(mode) = sig.param_ownership.get(sig_idx) {
                                    if matches!(mode, OwnershipMode::Owned) {
                                        return true;
                                    }
                                }
                            }
                        }
                    }
                }
                for (_, arg) in arguments {
                    if self.expr_calls_consuming_method(param_name, arg, registry) {
                        return true;
                    }
                }
                false
            }
            Expression::Block { statements, .. } => {
                self.calls_consuming_method(param_name, statements, registry)
            }
            Expression::Binary { left, right, .. } => {
                self.expr_calls_consuming_method(param_name, left, registry)
                    || self.expr_calls_consuming_method(param_name, right, registry)
            }
            _ => false,
        }
    }

    /// Check if a parameter is the direct receiver of a method call.
    pub(crate) fn is_direct_receiver(&self, param_name: &str, object: &Expression) -> bool {
        match object {
            Expression::Identifier { name, .. } => name == param_name,
            _ => false,
        }
    }

    /// Check if a parameter is passed as a direct (non-&) argument to a function or method call.
    /// When a parameter is passed directly (not via &) to another function, it could be consumed
    /// (the callee may take ownership). Without knowing the callee's signature, we conservatively
    /// assume consumption and keep the parameter Owned.
    ///
    /// Examples that trigger Owned:
    /// - `Quest::new(id, title, description)` — id is a direct argument
    /// - `process(data)` — data is a direct argument
    ///
    /// Examples that do NOT trigger Owned:
    /// - `data.len()` — data is the receiver, not an argument
    /// - `process(&data)` — & wraps the argument, so it's borrowed
    /// - `format!("{}", data)` — macro call, not a function call in the AST
    #[allow(dead_code)]
    pub(crate) fn is_passed_as_argument(&self, name: &str, statements: &[&'ast Statement<'ast>]) -> bool {
        for stmt in statements {
            if self.stmt_passes_as_argument(name, stmt) {
                return true;
            }
        }
        false
    }

    pub(crate) fn stmt_passes_as_argument(&self, name: &str, stmt: &Statement<'ast>) -> bool {
        match stmt {
            Statement::Let { value, .. } => self.expr_passes_as_argument(name, value),
            Statement::Expression { expr, .. } => self.expr_passes_as_argument(name, expr),
            Statement::Return { value: Some(v), .. } => self.expr_passes_as_argument(name, v),
            Statement::Assignment { value, .. } => self.expr_passes_as_argument(name, value),
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.expr_passes_as_argument(name, condition)
                    || self.is_passed_as_argument(name, then_block)
                    || else_block
                        .as_ref()
                        .is_some_and(|b| self.is_passed_as_argument(name, b))
            }
            Statement::While {
                condition, body, ..
            } => {
                self.expr_passes_as_argument(name, condition)
                    || self.is_passed_as_argument(name, body)
            }
            Statement::Loop { body, .. } => self.is_passed_as_argument(name, body),
            Statement::For { body, .. } => self.is_passed_as_argument(name, body),
            Statement::Match { value, arms, .. } => {
                self.expr_passes_as_argument(name, value)
                    || arms
                        .iter()
                        .any(|arm| self.expr_passes_as_argument(name, arm.body))
            }
            _ => false,
        }
    }

    pub(crate) fn expr_passes_as_argument(&self, name: &str, expr: &Expression<'ast>) -> bool {
        match expr {
            // Function call: check if parameter is a bare argument (not wrapped in &)
            Expression::Call { arguments, .. } => {
                // TDD FIX: Don't force Owned for simple pass-through!
                // If a parameter is ONLY passed to another function with no other operations,
                // it might be a pass-through and can stay Borrowed.
                //
                // CONSERVATIVE APPROACH: Still return true (force Owned) because without
                // the callee's signature (which doesn't exist during analysis), we can't
                // know if the callee consumes the value or just borrows it.
                //
                // FUTURE: Multi-pass analysis could solve this:
                // - Pass 1: Conservative inference
                // - Pass 2: Re-infer using SignatureRegistry from Pass 1
                // - Iterate until stable
                for (_label, arg) in arguments {
                    // Direct identifier: `f(param)` → potentially consuming
                    if matches!(arg, Expression::Identifier { name: id, .. } if id == name) {
                        return true;
                    }
                    // Recursively check sub-expressions for nested calls
                    if self.expr_passes_as_argument(name, arg) {
                        return true;
                    }
                }
                false
            }
            // Method call: check arguments (NOT the receiver)
            Expression::MethodCall {
                object, arguments, ..
            } => {
                for (_label, arg) in arguments {
                    // Direct identifier: `obj.method(param)` → consuming
                    if matches!(arg, Expression::Identifier { name: id, .. } if id == name) {
                        return true;
                    }
                    // Recursively check sub-expressions
                    if self.expr_passes_as_argument(name, arg) {
                        return true;
                    }
                }
                // Also check the receiver for nested calls (but NOT as a direct argument)
                self.expr_passes_as_argument(name, object)
            }
            // Block expression: check all statements
            Expression::Block { statements, .. } => self.is_passed_as_argument(name, statements),
            // Binary, unary, index, etc.: recurse into sub-expressions
            Expression::Binary { left, right, .. } => {
                self.expr_passes_as_argument(name, left)
                    || self.expr_passes_as_argument(name, right)
            }
            Expression::Unary { operand, .. } => self.expr_passes_as_argument(name, operand),
            Expression::Index { object, index, .. } => {
                self.expr_passes_as_argument(name, object)
                    || self.expr_passes_as_argument(name, index)
            }
            Expression::FieldAccess { object, .. } => self.expr_passes_as_argument(name, object),
            Expression::Tuple { elements, .. } | Expression::Array { elements, .. } => elements
                .iter()
                .any(|e| self.expr_passes_as_argument(name, e)),
            Expression::Closure { body, .. } => self.expr_passes_as_argument(name, body),
            // TDD FIX: TryOp wraps expressions with `?` (error propagation).
            // e.g., `process(data)?` produces TryOp { expr: Call { args: [data] } }
            // We must recurse into the inner expression to detect argument passing.
            Expression::TryOp { expr, .. } => self.expr_passes_as_argument(name, expr),
            // Note: We do NOT check Expression::Identifier here because bare identifiers
            // outside of Call/MethodCall arguments are not consuming (e.g., `data.len()`)
            _ => false,
        }
    }

    // TDD FIX (Bug #5): New function to check ONLY arithmetic operations, not comparisons
    pub(crate) fn is_used_in_arithmetic_op(&self, name: &str, statements: &[&'ast Statement<'ast>]) -> bool {
        for stmt in statements {
            match stmt {
                Statement::Let { value, .. } => {
                    if self.expr_uses_in_arithmetic_op(name, value) {
                        return true;
                    }
                }
                Statement::Expression { expr, .. } => {
                    if self.expr_uses_in_arithmetic_op(name, expr) {
                        return true;
                    }
                }
                Statement::Return {
                    value: Some(expr), ..
                } => {
                    if self.expr_uses_in_arithmetic_op(name, expr) {
                        return true;
                    }
                }
                Statement::Return { value: None, .. } => {}
                Statement::If {
                    condition,
                    then_block,
                    else_block,
                    ..
                } => {
                    if self.expr_uses_in_arithmetic_op(name, condition) {
                        return true;
                    }
                    if self.is_used_in_arithmetic_op(name, then_block) {
                        return true;
                    }
                    if let Some(else_block) = else_block {
                        if self.is_used_in_arithmetic_op(name, else_block) {
                            return true;
                        }
                    }
                }
                Statement::While {
                    condition, body, ..
                } => {
                    if self.expr_uses_in_arithmetic_op(name, condition) {
                        return true;
                    }
                    if self.is_used_in_arithmetic_op(name, body) {
                        return true;
                    }
                }
                Statement::For { body, .. } => {
                    if self.is_used_in_arithmetic_op(name, body) {
                        return true;
                    }
                }
                Statement::Assignment { value, .. } => {
                    if self.expr_uses_in_arithmetic_op(name, value) {
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }

    pub(crate) fn expr_uses_in_arithmetic_op(&self, name: &str, expr: &Expression) -> bool {
        match expr {
            Expression::Binary {
                op, left, right, ..
            } => {
                use crate::parser::ast::operators::BinaryOp;
                // Only check for arithmetic operators, not comparisons
                let is_arithmetic = matches!(
                    op,
                    BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod
                );

                if is_arithmetic {
                    if self.expr_is_identifier(left, name) || self.expr_is_identifier(right, name) {
                        return true;
                    }
                }
                // Recursively check nested expressions
                self.expr_uses_in_arithmetic_op(name, left)
                    || self.expr_uses_in_arithmetic_op(name, right)
            }
            Expression::Unary { operand, .. } => self.expr_uses_in_arithmetic_op(name, operand),
            Expression::Call { arguments, .. } => arguments
                .iter()
                .any(|(_, arg)| self.expr_uses_in_arithmetic_op(name, arg)),
            Expression::MethodCall {
                object, arguments, ..
            } => {
                self.expr_uses_in_arithmetic_op(name, object)
                    || arguments
                        .iter()
                        .any(|(_, arg)| self.expr_uses_in_arithmetic_op(name, arg))
            }
            Expression::FieldAccess { .. } => false,
            Expression::Index { object, index, .. } => {
                self.expr_uses_in_arithmetic_op(name, object)
                    || self.expr_uses_in_arithmetic_op(name, index)
            }
            Expression::Block { statements, .. } => self.is_used_in_arithmetic_op(name, statements),
            Expression::Tuple { elements, .. } => elements
                .iter()
                .any(|elem| self.expr_uses_in_arithmetic_op(name, elem)),
            Expression::Array { elements, .. } => elements
                .iter()
                .any(|elem| self.expr_uses_in_arithmetic_op(name, elem)),
            Expression::TryOp { expr, .. } => self.expr_uses_in_arithmetic_op(name, expr),
            _ => false,
        }
    }

    pub(crate) fn is_used_in_binary_op(&self, name: &str, statements: &[&'ast Statement<'ast>]) -> bool {
        for stmt in statements {
            match stmt {
                Statement::Let { value, .. } => {
                    if self.expr_uses_in_binary_op(name, value) {
                        return true;
                    }
                }
                Statement::Expression { expr, .. } => {
                    if self.expr_uses_in_binary_op(name, expr) {
                        return true;
                    }
                }
                Statement::Return {
                    value: Some(expr), ..
                } => {
                    if self.expr_uses_in_binary_op(name, expr) {
                        return true;
                    }
                }
                Statement::Return { value: None, .. } => {}
                Statement::If {
                    condition,
                    then_block,
                    else_block,
                    ..
                } => {
                    if self.expr_uses_in_binary_op(name, condition) {
                        return true;
                    }
                    if self.is_used_in_binary_op(name, then_block) {
                        return true;
                    }
                    if let Some(else_b) = else_block {
                        if self.is_used_in_binary_op(name, else_b) {
                            return true;
                        }
                    }
                }
                Statement::Loop { body, .. }
                | Statement::While { body, .. }
                | Statement::For { body, .. } => {
                    if self.is_used_in_binary_op(name, body) {
                        return true;
                    }
                }
                Statement::Assignment { value, .. } => {
                    if self.expr_uses_in_binary_op(name, value) {
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }

    pub(crate) fn expr_uses_in_binary_op(&self, name: &str, expr: &Expression) -> bool {
        match expr {
            Expression::Binary { left, right, .. } => {
                // Check if the parameter is directly used in a binary operation
                // This is for Copy types like Vec2, Vec3 where `a + b` requires owned values
                if self.expr_is_identifier(left, name) || self.expr_is_identifier(right, name) {
                    return true;
                }
                // Recursively check nested expressions
                self.expr_uses_in_binary_op(name, left) || self.expr_uses_in_binary_op(name, right)
            }
            Expression::Unary { operand, .. } => self.expr_uses_in_binary_op(name, operand),
            Expression::Call { arguments, .. } => arguments
                .iter()
                .any(|(_, arg)| self.expr_uses_in_binary_op(name, arg)),
            Expression::MethodCall {
                object, arguments, ..
            } => {
                self.expr_uses_in_binary_op(name, object)
                    || arguments
                        .iter()
                        .any(|(_, arg)| self.expr_uses_in_binary_op(name, arg))
            }
            // CRITICAL FIX: Don't recurse into FieldAccess for binary op detection
            // `self.field + value` doesn't mean `self` is used in a binary op
            // We only care about the DIRECT use of the parameter, like `param + value`
            Expression::FieldAccess { .. } => false,
            Expression::Index { object, index, .. } => {
                self.expr_uses_in_binary_op(name, object)
                    || self.expr_uses_in_binary_op(name, index)
            }
            Expression::Block { statements, .. } => self.is_used_in_binary_op(name, statements),
            // Recurse into tuple elements
            Expression::Tuple { elements, .. } => elements
                .iter()
                .any(|elem| self.expr_uses_in_binary_op(name, elem)),
            // Recurse into array elements
            Expression::Array { elements, .. } => elements
                .iter()
                .any(|elem| self.expr_uses_in_binary_op(name, elem)),
            Expression::TryOp { expr, .. } => self.expr_uses_in_binary_op(name, expr),
            _ => false,
        }
    }

    /// `let v = param` (and chains) so that `if let` / `match` on `v` must still require an owned
    /// parameter when the arm moves out of `Option`/`Result`, etc.
    pub(crate) fn simple_let_alias_ids_for_param(
        &self,
        param_name: &str,
        statements: &[&'ast Statement<'ast>],
    ) -> HashSet<String> {
        let mut set = HashSet::new();
        set.insert(param_name.to_string());
        let mut changed = true;
        while changed {
            changed = false;
            self.simple_let_alias_expand_pass(&mut set, statements, &mut changed);
        }
        set
    }

    pub(crate) fn simple_let_alias_expand_pass(
        &self,
        set: &mut HashSet<String>,
        statements: &[&'ast Statement<'ast>],
        changed: &mut bool,
    ) {
        for stmt in statements {
            match stmt {
                Statement::Let { pattern, value, .. } => {
                    if let Pattern::Identifier(local) = pattern {
                        if let Expression::Identifier { name: src, .. } = &**value {
                            if set.contains(src) && set.insert(local.clone()) {
                                *changed = true;
                            }
                        }
                    }
                }
                Statement::If {
                    then_block,
                    else_block,
                    ..
                } => {
                    self.simple_let_alias_expand_pass(set, then_block, changed);
                    if let Some(else_b) = else_block {
                        self.simple_let_alias_expand_pass(set, else_b, changed);
                    }
                }
                Statement::For { body, .. }
                | Statement::While { body, .. }
                | Statement::Loop { body, .. } => {
                    self.simple_let_alias_expand_pass(set, body, changed);
                }
                Statement::Match { arms, .. } => {
                    for arm in arms {
                        if let Expression::Block { statements, .. } = arm.body {
                            self.simple_let_alias_expand_pass(set, statements, changed);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// Check if a parameter is pattern matched with field extraction
    /// e.g., `match param { Enum::Variant { field: f } => ... }`
    /// If we borrow the parameter, `f` becomes a reference, breaking calls expecting owned values
    pub(crate) fn is_pattern_matched_with_fields(
        &self,
        name: &str,
        statements: &[&'ast Statement<'ast>],
    ) -> bool {
        let aliases = self.simple_let_alias_ids_for_param(name, statements);
        self.match_arm_destructures_enum_subpatterns_in_stmts(&aliases, statements)
    }

    pub(crate) fn match_arm_destructures_enum_subpatterns_in_stmts(
        &self,
        aliases: &HashSet<String>,
        statements: &[&'ast Statement<'ast>],
    ) -> bool {
        for stmt in statements {
            match stmt {
                Statement::Match { value, arms, .. } => {
                    if let Expression::Identifier { name: id, .. } = value {
                        if aliases.contains(id) {
                            for arm in arms {
                                if self.pattern_has_field_bindings(&arm.pattern) {
                                    return true;
                                }
                            }
                        }
                    }
                }
                Statement::If {
                    then_block,
                    else_block,
                    ..
                } => {
                    if self.match_arm_destructures_enum_subpatterns_in_stmts(aliases, then_block) {
                        return true;
                    }
                    if let Some(else_b) = else_block {
                        if self.match_arm_destructures_enum_subpatterns_in_stmts(aliases, else_b) {
                            return true;
                        }
                    }
                }
                Statement::For { body, .. }
                | Statement::While { body, .. }
                | Statement::Loop { body, .. } => {
                    if self.match_arm_destructures_enum_subpatterns_in_stmts(aliases, body) {
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }

    /// Check if a pattern has field bindings (not just wildcards or simple identifiers)
    pub(crate) fn pattern_has_field_bindings(&self, pattern: &Pattern) -> bool {
        use crate::parser::EnumPatternBinding;

        match pattern {
            Pattern::EnumVariant(_, binding) => {
                // Check if the binding extracts fields
                matches!(
                    binding,
                    EnumPatternBinding::Single(_)
                        | EnumPatternBinding::Tuple(_)
                        | EnumPatternBinding::Struct(_, _)
                )
            }
            Pattern::Tuple(patterns) => patterns.iter().any(|p| self.pattern_has_field_bindings(p)),
            Pattern::Or(patterns) => patterns.iter().any(|p| self.pattern_has_field_bindings(p)),
            _ => false,
        }
    }
    /// Check if param type appears in return type (direct, Result<T,E>, or Option<T>).
    /// When fn(T) -> Result<T,E>, we need owned param to produce the return value.
    pub(crate) fn param_type_matches_return(&self, param_type: &Type, return_type: &Type) -> bool {
        match return_type {
            // Direct match: fn(T) -> T
            t if self.types_equal(param_type, t) => true,
            // Result<T, E>: fn(T) -> Result<T, E>
            Type::Result(ok_type, _err_type) => self.types_equal(param_type, ok_type),
            // Option<T>: fn(T) -> Option<T>
            Type::Option(inner) => self.types_equal(param_type, inner),
            _ => false,
        }
    }

    /// Compare two types for equality (custom types, primitives).
    /// Callee parameter type from `SignatureRegistry` must match the caller's declared type
    /// before passthrough ownership is applied. Prevents short names like `contains` / `len`
    /// from unrelated impls forcing Owned on `str` / `string` parameters (E0382).
    pub(super) fn passthrough_types_compatible(&self, sig_ty: &Type, decl_ty: &Type) -> bool {
        if self.types_equal(sig_ty, decl_ty) {
            return true;
        }
        let decl_str = matches!(decl_ty, Type::String);
        let sig_str = matches!(sig_ty, Type::String);
        decl_str && sig_str
    }

    pub(crate) fn types_equal(&self, a: &Type, b: &Type) -> bool {
        match (a, b) {
            (Type::Custom(name_a), Type::Custom(name_b)) => name_a == name_b,
            (Type::String, Type::String) => true,
            (Type::Int, Type::Int) => true,
            (Type::Int32, Type::Int32) => true,
            (Type::Uint, Type::Uint) => true,
            (Type::Float, Type::Float) => true,
            (Type::Bool, Type::Bool) => true,
            (Type::Vec(inner_a), Type::Vec(inner_b)) => self.types_equal(inner_a, inner_b),
            (Type::Option(inner_a), Type::Option(inner_b)) => self.types_equal(inner_a, inner_b),
            (Type::Array(inner_a, _), Type::Array(inner_b, _)) => {
                self.types_equal(inner_a, inner_b)
            }
            (Type::Result(ok_a, err_a), Type::Result(ok_b, err_b)) => {
                self.types_equal(ok_a, ok_b) && self.types_equal(err_a, err_b)
            }
            (Type::Tuple(elems_a), Type::Tuple(elems_b)) => {
                elems_a.len() == elems_b.len()
                    && elems_a
                        .iter()
                        .zip(elems_b.iter())
                        .all(|(a, b)| self.types_equal(a, b))
            }
            (Type::Reference(inner_a), Type::Reference(inner_b)) => {
                self.types_equal(inner_a, inner_b)
            }
            (Type::MutableReference(inner_a), Type::MutableReference(inner_b)) => {
                self.types_equal(inner_a, inner_b)
            }
            (Type::Parameterized(name_a, args_a), Type::Parameterized(name_b, args_b)) => {
                name_a == name_b
                    && args_a.len() == args_b.len()
                    && args_a
                        .iter()
                        .zip(args_b.iter())
                        .all(|(a, b)| self.types_equal(a, b))
            }
            _ => false,
        }
    }
}
