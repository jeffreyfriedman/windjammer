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
                        || (!string_like && self.param_is_consumed_into_return(param_name, body))
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

        // 0d. HashMap lookup keys (get, contains_key, …) are always borrowed.
        // Match scrutinees like `match self.nodes.get(id)` must not infer Owned for `id`
        // when passthrough collides on bare `get` from unrelated types.
        if Self::is_windjammer_text_param_type(param_type)
            && self.is_only_hashmap_lookup_key_param(param_name, body, func)
        {
            return Ok(OwnershipMode::Borrowed);
        }

        // 1. Check if parameter is mutated (uses registry for method call detection)
        if self.is_mutated(param_name, body, registry, Some(param_type)) {
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
                OwnershipMode::MutBorrowed => return Ok(OwnershipMode::MutBorrowed),
                OwnershipMode::Owned => {
                    return Ok(OwnershipMode::Owned);
                }
            }
        }

        if Self::is_windjammer_text_param_type(param_type)
            && self.is_only_hashmap_lookup_key_param(param_name, body, func)
        {
            return Ok(OwnershipMode::Borrowed);
        }

        // 8. HashMap lookup keys — pin Borrowed after registry-dependent steps.
        // Large engine metadata can flip passthrough/is_mutated heuristics on later
        // convergence passes; the body fact (only used as HashMap key) is authoritative.
        if Self::is_windjammer_text_param_type(param_type)
            && self.is_only_hashmap_lookup_key_param(param_name, body, func)
        {
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
