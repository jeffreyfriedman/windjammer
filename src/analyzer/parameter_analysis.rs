//! Parameter ownership inference: orchestration (`infer_parameter_ownership`),
//! borrow-only heuristic (`*-param_only_borrowed`), and return/passthrough type compatibility.
//!
//! Kept separate from usage/call-site walks so the main inference entry point and `types_equal`
//! stay co-located for [`super::passthrough_inference`].

use crate::parser::*;

use super::{Analyzer, OwnershipMode, SignatureRegistry};

impl<'ast> Analyzer<'ast> {
    #[allow(clippy::too_many_arguments)] // Inference needs full function context
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
        // Windjammer `string` ownership is resolved at the end: module-level formals default
        // to owned; impl-method passthrough helpers may infer borrowed `&str`.
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
                    let string_like = Self::is_windjammer_text_param_type(param_type);
                    // Module-level identity returns (e.g. `pub fn greet(name) -> name`) need
                    // owned `String` formals. Impl-method passthrough helpers (e.g.
                    // `extract_extension(path) -> path` called with `self.path`) stay
                    // borrowed so static call sites pass `&str` without cloning.
                    let force_owned = if string_like && func.parent_type.is_some() {
                        self.is_stored(param_name, body)
                            || self.param_is_consumed_into_return(param_name, body)
                    } else {
                        self.is_returned(param_name, body)
                            || self.is_stored(param_name, body)
                            || (!string_like
                                && self.param_is_consumed_into_return(param_name, body))
                    };
                    if force_owned {
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

        // 0d. HashMap lookup keys (get, contains_key, …) are always borrowed (or owned if Copy).
        // Match scrutinees like `match self.nodes.get(id)` must not infer Owned for `id`
        // when passthrough collides on bare `get` from unrelated types.
        if self.is_only_hashmap_lookup_key_param(param_name, body, func) {
            if self.is_copy_type(param_type) {
                return Ok(OwnershipMode::Owned);
            }
            return Ok(OwnershipMode::Borrowed);
        }

        // FFI string wrappers: passthrough to extern keeps Borrowed API even when the
        // callee consumes owned String at the boundary (codegen adds string_to_ffi).
        if Self::is_windjammer_text_param_type(param_type) && func.parent_type.is_none() {
            if let Some(OwnershipMode::Borrowed) = self.infer_passthrough_ownership(
                param_name,
                param_type,
                body,
                registry,
                current_func_name,
                func,
            ) {
                return Ok(OwnershipMode::Borrowed);
            }
        }

        // 1. Check if parameter is mutated (uses registry for method call detection)
        if self.is_mutated(param_name, body, registry, Some(param_type)) {
            return Ok(OwnershipMode::MutBorrowed);
        }

        // 2. Check if parameter is returned (escapes function)
        if self.is_returned(param_name, body)
            && !(Self::is_windjammer_text_param_type(param_type) && func.parent_type.is_some())
        {
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
        let stored = self.is_stored(param_name, body);
        if stored {
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

        // 6. For-loop iteration: if the loop dereferences elements (`*i`), the collection is
        // borrowed (`for i in &items`); otherwise consuming iteration uses owned param.
        if self.is_iterated_over(param_name, body) {
            if self.for_loop_over_param_dereferences_element(param_name, body) {
                return Ok(OwnershipMode::Borrowed);
            }
            if !self.is_copy_type(param_type) {
                return Ok(OwnershipMode::Owned);
            }
        }

        // 6b. TDD: Check if parameter calls a method that takes `self` by value (consuming).
        // When `a.as_float()` is called and `as_float` takes owned `self`, `a` is consumed
        // and must be owned. Without this check, `a` defaults to Borrowed (&a), producing
        // E0507 errors because you can't move out of a shared reference.
        if self.calls_consuming_method(param_name, body, registry) {
            return Ok(OwnershipMode::Owned);
        }

        // 6d. Bare identifier passed to a **function** call uses move semantics (non-Copy)
        // when callees consume the value. Method-call passthrough (e.g. HashSet::contains)
        // is handled by step 7 / default Borrowed — not this rule.
        if !self.is_copy_type(param_type)
            && self.is_passed_by_value_as_function_call_arg(param_name, body)
        {
            if let Some(mode) = self.infer_passthrough_ownership(
                param_name,
                param_type,
                body,
                registry,
                current_func_name,
                func,
            ) {
                // Passthrough to a borrowed callee: wrapper keeps Borrowed so signatures
                // chain (`fn wrapper(items: &Vec<T>) { process(items) }`).
                // Extern FFI callees still surface Borrowed for `string` wrappers via
                // infer_passthrough_ownership's extern rule.
                return Ok(mode);
            }
            // Plain `string` may passthrough to extern on a later convergence pass — do not
            // pin Owned here or FFI wrappers never reach Borrowed (module_qualified autoborrow).
            if !Self::is_windjammer_text_param_type(param_type) {
                return Ok(OwnershipMode::Owned);
            }
        }

        // 6c. TryOp-wrapped non-readonly method calls (e.g. `loader.load()?`) may need
        // &mut self — do not infer Borrowed for the receiver. Scoped to `?` only so
        // unknown methods on typed params still default to borrowed (multi-pass refines).
        if self.has_potentially_mutating_method_call_in_tryop(param_name, body) {
            return Ok(OwnershipMode::MutBorrowed);
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
                OwnershipMode::MutBorrowed => {
                    if self.is_copy_type(param_type) {
                        return Ok(OwnershipMode::Owned);
                    }
                    // `mut param` used only as an argument to a &mut callee keeps an owned
                    // binding; the call site adds `&mut`. Method calls on the param (e.g.
                    // `c.increment()`) need `&mut T` in the signature itself.
                    let only_fn_passthrough = func
                        .parameters
                        .iter()
                        .find(|p| p.name == param_name)
                        .is_some_and(|p| p.is_mutable)
                        && !self.param_has_direct_method_calls(param_name, body);
                    if only_fn_passthrough {
                        return Ok(OwnershipMode::Owned);
                    }
                    return Ok(OwnershipMode::MutBorrowed);
                }
                OwnershipMode::Owned => return Ok(OwnershipMode::Owned),
            }
        }

        if self.is_only_hashmap_lookup_key_param(param_name, body, func) {
            if self.is_copy_type(param_type) {
                return Ok(OwnershipMode::Owned);
            }
            return Ok(OwnershipMode::Borrowed);
        }

        // 8. HashMap lookup keys — pin Borrowed after registry-dependent steps.
        // Large engine metadata can flip passthrough/is_mutated heuristics on later
        // convergence passes; the body fact (only used as HashMap key) is authoritative.
        if self.is_only_hashmap_lookup_key_param(param_name, body, func) {
            if self.is_copy_type(param_type) {
                return Ok(OwnershipMode::Owned);
            }
            return Ok(OwnershipMode::Borrowed);
        }

        // 9. Default ownership: Borrowed (THE WINDJAMMER WAY!)
        //
        // **PHILOSOPHY**: The compiler does the work, not the user.
        // - Default to **Borrowed** for read-only parameters
        // - The checks above handle all consuming cases (mutated, returned,
        //   stored, iterated, used in binary ops, pattern matched)
        // - If none of those apply, the parameter is truly read-only
        // - Read-only non-Copy parameters should be &T in generated Rust
        // - Copy types default to Owned (pass-by-value); see step 9 below
        //
        // This matches the Windjammer philosophy: users write `data: Vec<f32>`
        // and the compiler infers `&Vec<f32>` when data is only read.
        // Call sites naturally pass `&self.data` which matches `&Vec<f32>`.
        //
        // Dogfooding evidence: 6+ E0308 errors in windjammer-game-editor
        // from read-only params generating owned types while call sites pass &T.
        //
        // Module-level `string`: owned when passed to calls or used in comparisons;
        // Phase-2 `&str` for read-only receiver usage (`text.len()`) and struct-literal storage.
        if Self::is_windjammer_text_param_type(param_type) && func.parent_type.is_none() {
            // Function/method calls only — macro args (format!/println!) are Display reads, not moves.
            if self.is_passed_by_value_as_function_call_arg(param_name, body) {
                if let Some(OwnershipMode::Borrowed) = self.infer_passthrough_ownership(
                    param_name,
                    param_type,
                    body,
                    registry,
                    current_func_name,
                    func,
                ) {
                    return Ok(OwnershipMode::Borrowed);
                }
                return Ok(OwnershipMode::Owned);
            }
            if self.is_used_in_binary_op(param_name, body)
                && !self.is_used_in_arithmetic_op(param_name, body)
            {
                return Ok(OwnershipMode::Owned);
            }
            // param_needs_string_ref == false → can use &str → Borrowed
            // param_needs_string_ref == true → needs &String (e.g. Vec::contains) → still Borrowed
            // (consuming patterns like push/insert are caught by is_stored above)
            // Codegen distinguishes &str vs &String via str_ref_optimized_params.
            return Ok(OwnershipMode::Borrowed);
        }
        // Copy types default to Owned unless passthrough to a mutating callee needs &mut.
        if self.is_copy_type(param_type) {
            if let Some(OwnershipMode::MutBorrowed) = self.infer_passthrough_ownership(
                param_name,
                param_type,
                body,
                registry,
                current_func_name,
                func,
            ) {
                return Ok(OwnershipMode::MutBorrowed);
            }
            return Ok(OwnershipMode::Owned);
        }
        Ok(OwnershipMode::Borrowed)
    }

    /// True when `param_name` appears as a bare identifier argument in a **function** call
    /// (not a method call), indicating move semantics when the callee consumes the value.
    pub(crate) fn is_passed_by_value_as_function_call_arg(
        &self,
        param_name: &str,
        body: &[&'ast Statement<'ast>],
    ) -> bool {
        let mut found = false;
        for stmt in body {
            self.stmt_collect_pass_by_value_function_call_arg(param_name, stmt, &mut found);
            if found {
                return true;
            }
        }
        false
    }

    fn stmt_collect_pass_by_value_function_call_arg(
        &self,
        param_name: &str,
        stmt: &Statement,
        found: &mut bool,
    ) {
        if *found {
            return;
        }
        match stmt {
            Statement::Let { value, .. } => {
                self.expr_collect_pass_by_value_function_call_arg(param_name, value, found)
            }
            Statement::Return { value, .. } => {
                if let Some(v) = value {
                    self.expr_collect_pass_by_value_function_call_arg(param_name, v, found)
                }
            }
            Statement::Expression { expr, .. } => {
                self.expr_collect_pass_by_value_function_call_arg(param_name, expr, found)
            }
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.expr_collect_pass_by_value_function_call_arg(param_name, condition, found);
                if *found {
                    return;
                }
                for s in then_block {
                    self.stmt_collect_pass_by_value_function_call_arg(param_name, s, found);
                    if *found {
                        return;
                    }
                }
                if let Some(block) = else_block {
                    for s in block {
                        self.stmt_collect_pass_by_value_function_call_arg(param_name, s, found);
                        if *found {
                            return;
                        }
                    }
                }
            }
            Statement::Match { value, arms, .. } => {
                self.expr_collect_pass_by_value_function_call_arg(param_name, value, found);
                if *found {
                    return;
                }
                for arm in arms {
                    if let Some(guard) = arm.guard {
                        self.expr_collect_pass_by_value_function_call_arg(param_name, guard, found);
                        if *found {
                            return;
                        }
                    }
                    self.expr_collect_pass_by_value_function_call_arg(param_name, arm.body, found);
                    if *found {
                        return;
                    }
                }
            }
            Statement::While { body, .. }
            | Statement::For { body, .. }
            | Statement::Loop { body, .. } => {
                for s in body {
                    self.stmt_collect_pass_by_value_function_call_arg(param_name, s, found);
                    if *found {
                        return;
                    }
                }
            }
            _ => {}
        }
    }

    fn expr_collect_pass_by_value_function_call_arg(
        &self,
        param_name: &str,
        expr: &Expression,
        found: &mut bool,
    ) {
        if *found {
            return;
        }
        match expr {
            Expression::Call { arguments, .. } => {
                for (_, arg) in arguments {
                    if matches!(arg, Expression::Identifier { name, .. } if name == param_name) {
                        *found = true;
                        return;
                    }
                    self.expr_collect_pass_by_value_function_call_arg(param_name, arg, found);
                }
            }
            Expression::Block { statements, .. } => {
                for s in statements {
                    self.stmt_collect_pass_by_value_function_call_arg(param_name, s, found);
                    if *found {
                        return;
                    }
                }
            }
            Expression::Binary { left, right, .. } => {
                self.expr_collect_pass_by_value_function_call_arg(param_name, left, found);
                self.expr_collect_pass_by_value_function_call_arg(param_name, right, found);
            }
            Expression::Unary { operand, .. } => {
                self.expr_collect_pass_by_value_function_call_arg(param_name, operand, found);
            }
            Expression::FieldAccess { object, .. } => {
                self.expr_collect_pass_by_value_function_call_arg(param_name, object, found);
            }
            Expression::Index { object, index, .. } => {
                self.expr_collect_pass_by_value_function_call_arg(param_name, object, found);
                self.expr_collect_pass_by_value_function_call_arg(param_name, index, found);
            }
            Expression::TryOp { expr, .. } => {
                self.expr_collect_pass_by_value_function_call_arg(param_name, expr, found);
            }
            Expression::MethodCall {
                object, arguments, ..
            } => {
                self.expr_collect_pass_by_value_function_call_arg(param_name, object, found);
                for (_, arg) in arguments {
                    self.expr_collect_pass_by_value_function_call_arg(param_name, arg, found);
                }
            }
            Expression::MacroInvocation { name, args, .. } => {
                let borrows_only = matches!(
                    name.as_str(),
                    "format"
                        | "println"
                        | "print"
                        | "eprintln"
                        | "eprint"
                        | "write"
                        | "writeln"
                        | "panic"
                        | "debug"
                        | "info"
                        | "warn"
                        | "error"
                        | "trace"
                        | "log"
                );
                if !borrows_only {
                    for arg in args {
                        if matches!(arg, Expression::Identifier { name, .. } if name == param_name)
                        {
                            *found = true;
                            return;
                        }
                        self.expr_collect_pass_by_value_function_call_arg(param_name, arg, found);
                    }
                }
            }
            _ => {}
        }
    }

    /// True when the parameter is the receiver of a method call (e.g. `grid.set()`), not
    /// merely passed as an argument to a free function.
    fn param_has_direct_method_calls(
        &self,
        param_name: &str,
        body: &[&'ast Statement<'ast>],
    ) -> bool {
        let mut calls = Vec::new();
        self.collect_method_calls_on_param(param_name, body, &mut calls);
        !calls.is_empty()
    }

    /// TDD: Check if parameter is ONLY used as &param or &mut param (never consumed directly).
    /// Example: a + &b + &c - b and c are only used as &b, &c → true for b and c.
    /// Used to avoid param_type_matches_return incorrectly inferring Owned for string params
    /// in concatenation: fn(a, b, c) -> a + &b + &c.
    pub(crate) fn is_only_used_as_borrow(
        &self,
        param_name: &str,
        body: &[&'ast Statement<'ast>],
    ) -> bool {
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
            Expression::Binary {
                left, right, op, ..
            } => {
                use crate::parser::BinaryOp;
                // String concatenation `lhs + rhs`: codegen emits `lhs + &rhs` — RHS is borrow-only.
                if matches!(op, BinaryOp::Add) {
                    if self.expr_is_identifier(right, param_name) {
                        return self.expr_param_only_borrowed(param_name, left, false);
                    }
                    if self.expr_is_identifier(left, param_name) {
                        return false;
                    }
                }
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
        // Windjammer `string` formals passthrough to extern `String` / owned string callees.
        if Self::is_windjammer_text_param_type(decl_ty)
            && Self::is_windjammer_text_param_type(sig_ty)
        {
            return true;
        }
        // Callee signature uses `&T` / `&mut T` while the caller declares owned `T`.
        match sig_ty {
            Type::Reference(inner) | Type::MutableReference(inner) => {
                return self.types_equal(inner, decl_ty);
            }
            _ => {}
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
