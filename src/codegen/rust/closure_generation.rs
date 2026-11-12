//! Closure expression generation
//!
//! Handles generation of:
//! - Closure expressions (|params| body)
//! - Smart `move` inference based on capture analysis
//!
//! THE WINDJAMMER WAY: The compiler infers `move` automatically.
//! - Closures that capture outer variables → `move` (ownership transfer)
//! - Closures that only use their own params → no `move` (pure functions)
//! - Closures that capture `self` → no `move` (borrow from method receiver)
//! - Compiler-generated wrappers (__params) → always `move`

use crate::parser::Expression;

use super::CodeGenerator;

impl<'ast> CodeGenerator<'ast> {
    pub(in crate::codegen::rust) fn closure_param_binding_name(param: &str) -> String {
        if let Some(colon_idx) = param.find(": ") {
            param[..colon_idx].to_string()
        } else {
            param.to_string()
        }
    }

    fn format_closure_param_to_rust(&self, param: &str) -> String {
        if let Some(colon_idx) = param.find(": ") {
            let name = &param[..colon_idx];
            let type_str = param[colon_idx + 2..].trim();
            let ty = crate::codegen::rust::types::parse_wj_type_name(type_str);
            let rust_ty = self.type_to_rust(&ty);
            if self.closure_predicate_typed_params {
                // the adapter's declaration gives the depth (`fn(&T)` over `Iterator<&T>`: two); one when it is unknown
                let depth = self.idiom.adapter_closure_param_depth.get(name).copied().unwrap_or(1).max(1);
                format!("{}: {}{}", name, "&".repeat(depth), rust_ty)
            } else {
                format!("{}: {}", name, rust_ty)
            }
        } else {
            param.to_string()
        }
    }

    /// Generate code for closure expression |params| body
    pub(in crate::codegen::rust) fn generate_closure(
        &mut self,
        parameters: &[String],
        body: &Expression<'ast>,
    ) -> String {
        let borrows_captures = std::mem::take(&mut self.idiom.next_closure_borrows_captures);
        let params = parameters
            .iter()
            .map(|p| self.format_closure_param_to_rust(p))
            .collect::<Vec<_>>()
            .join(", ");

        let is_compiler_generated = parameters.iter().any(|p| p.starts_with("__"));
        let captures_self = self.expression_references_self(body);
        let captured = crate::ir::liveness::captures(parameters, body);
        // Escape fact (CLOS-MOVE-CAPTURE): a closure stays in its function only in a known `Fn`/`FnMut`
        // slot of its callee. An unknown callee (a dependency's `impl Fn() + 'static` callback
        // registry) or a `FnOnce` slot (`thread::spawn`) may keep it: it escapes and keeps `move`.
        let non_escaping_slot = self
            .call_arg_expected_type
            .as_ref()
            .is_some_and(crate::codegen::rust::stdlib_method_traits::formal_is_multi_invoke_closure_trait);
        let captures_outer = !captured.is_empty();

        // For user-written closures, set flag and track params to suppress transformations
        let prev_in_user_closure = self.in_user_written_closure;
        let prev_multi_invoke = self.closure_multi_invoke_captures;
        let mut prev_closure_params = None;
        if !is_compiler_generated {
            self.in_user_written_closure = true;
            // Signature-driven: `serve(|req| …)` / `Fn`/`FnMut` may invoke many times;
            // `spawn(move || …)` is FnOnce and may move captures once.
            // Missing enclosing formal: fail open to multi-invoke (clone) — safer than a
            // move into `Fn`. FnOnce sites (`thread::spawn`) register FnOnce/fn-ptr.
            self.closure_multi_invoke_captures = match self.call_arg_expected_type.as_ref() {
                Some(ty) => {
                    crate::codegen::rust::stdlib_method_traits::formal_is_multi_invoke_closure_trait(
                        ty,
                    )
                }
                None => captures_outer,
            };
            prev_closure_params = Some(std::mem::take(&mut self.user_closure_params));
            for param in parameters {
                self.user_closure_params
                    .insert(Self::closure_param_binding_name(param));
            }
        }

        // `|| base * 2` with `base: int`: the literal takes the width of its typed peer. Numeric
        // inference does not key closure-body expressions reliably, so give the body the type of
        // its own operands as the ambient integer slot.
        let prev_int_target = self.assignment_int_target_type.clone();
        // The operand that is not a literal is the typed peer. When it has no type of its own (an untyped
        // parameter) but numeric inference bound the literal (immediately-invoked `(|v| v * 2)(x)`, the
        // lowering of `x |> (|v| v * 2)`), that width wins; otherwise the body's own type is the slot.
        if !is_compiler_generated {
            if let Expression::Binary { left, right, .. } = body {
                let is_lit = |e: &Expression| matches!(e, Expression::Literal { .. });
                let peer = [&**left, &**right].into_iter().find(|e| !is_lit(e));
                let lit_inferred = [&**left, &**right].into_iter().filter(|e| is_lit(e)).any(|e| {
                    self.numeric_inference.as_ref().is_some_and(|ni| ni.get_int_type(e) != crate::type_inference::IntType::Unknown)
                });
                let slot = match peer.and_then(|p| self.infer_expression_type(p)) {
                    Some(ty) => Some(ty),
                    None if lit_inferred => None,
                    None => self.infer_expression_type(body),
                };
                if let Some(ty) = slot.filter(|t| Self::int_type_from_assignment_target(t).is_some()) {
                    self.assignment_int_target_type = Some(ty);
                }
            }
        }
        // A closure body is its own value context: its tail is the closure's result even when the
        // closure sits inside a unit-typed block (`hs.push(thread::spawn(move || { ..; s }))`).
        let was_void = std::mem::replace(&mut self.in_void_block, false);
        // a block whose tail is an `if`/`else` or `match` yields that value: its arms are not statements (`y.cmp(x)`, not `y.cmp(x);`)
        let tail_branches = matches!(body, Expression::Block { statements, .. }
            if matches!(statements.last(), Some(crate::parser::Statement::If { else_block: Some(_), .. } | crate::parser::Statement::Match { .. })));
        let was_expr = self.in_expression_context;
        self.in_expression_context = was_expr || tail_branches;
        // The adapter's by-reference parameter typing (`filter(|x: int| ..)` is `|x: &i64|`) belongs to this closure's own
        // parameters only: a closure nested in the body keeps its annotations as written (`|y: int|` is `|y: i64|`).
        let prev_predicate_typed = std::mem::replace(&mut self.closure_predicate_typed_params, false);
        let mut body_str = self.generate_expression(body);
        self.closure_predicate_typed_params = prev_predicate_typed;
        // a closure returns an owned value: a non-Copy field read through a reference parameter is cloned (`|p| p.1`)
        if let Expression::FieldAccess { object: Expression::Identifier { name, .. }, .. } = body {
            let through_ref = matches!(self.local_var_types.get(name), Some(crate::parser::Type::Reference(_)));
            if through_ref && !body_str.ends_with(".clone()") && self.infer_expression_type(body).is_some_and(|t| !self.is_type_copy(&t)) {
                body_str.push_str(".clone()");
            }
        }
        self.in_expression_context = was_expr;
        self.in_void_block = was_void;
        self.assignment_int_target_type = prev_int_target;

        if !is_compiler_generated {
            self.in_user_written_closure = prev_in_user_closure;
            self.closure_multi_invoke_captures = prev_multi_invoke;
            if let Some(prev_params) = prev_closure_params {
                self.user_closure_params = prev_params;
            }
        }

        let needs_move = if captures_self {
            false
        } else if is_compiler_generated {
            true
        } else if borrows_captures {
            false
        } else {
            // CLOS-MOVE-CAPTURE from liveness: in a non-escaping slot, a captured non-Copy binding
            // read after this statement stays with its owner, so the closure borrows (`move` is E0382).
            captures_outer
                && (!non_escaping_slot
                    || !captured
                        .iter()
                        .any(|n| self.binding_known_non_copy(n) && self.read_after_current_statement(n)))
        };

        if needs_move {
            format!("move |{}| {}", params, body_str)
        } else {
            format!("|{}| {}", params, body_str)
        }
    }
}
