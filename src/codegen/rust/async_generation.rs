//! Async and concurrency expression generation
//!
//! Handles generation of:
//! - Await expressions (.await)
//! - ADR-0004 tasks: `spawn`/`async` (closures over cloned captures), `scope` (a joining guard), `select`
//!   over `windjammer_runtime::task` (docs/concurrency/PLAN.md stage 2b)
//! - Range expressions (start..end, start..=end)

use crate::parser::{Expression, Type};

use super::CodeGenerator;

impl<'ast> CodeGenerator<'ast> {
    /// `t.await`: inside an `async fn` (tokio, until stage 2b removes `async fn`) Rust's `.await`; elsewhere it is
    /// sugar for `t.wait()` on an ADR-0004 `Task`.
    pub(in crate::codegen::rust) fn generate_await(&mut self, expr: &Expression<'ast>) -> String {
        let inner = self.generate_expression(expr);
        if self.current_function_is_async {
            format!("{}.await", inner)
        } else {
            format!("{}.wait()", inner)
        }
    }

    /// The names a task operand reads from the enclosing function: each is cloned into the task's `move` closure, so
    /// the task owns what it uses (`Send + 'static`); functions, intrinsics, paths and types are not values to copy.
    fn task_captures(&self, expr: &Expression<'ast>) -> Vec<String> {
        crate::ir::liveness::captures(&[], expr)
            .into_iter()
            .filter(|n| {
                !n.contains("::")
                    && !n.contains('.')
                    && n != "self"
                    && n.chars().next().is_some_and(|c| c.is_lowercase() || c == '_')
                    && crate::intrinsics::Intrinsic::from_call_name(n).is_none()
                    && (self.identifier_is_local_binding(n) || self.signature_registry.get_signature(n).is_none())
            })
            .collect()
    }

    /// `{ let x = x.clone(); ...; <ctor>(move || <operand>) }`
    fn generate_task(&mut self, ctor: &str, expr: &Expression<'ast>) -> String {
        let prelude: String = self.task_captures(expr).iter().map(|n| format!("let {n} = {n}.clone(); ")).collect();
        let body = self.generate_expression(expr);
        format!("{{ {prelude}windjammer_runtime::task::{ctor}(move || {body}) }}")
    }

    /// `async f(x)` / `async { }` (ADR-0004): a task on this thread's local queue.
    pub(in crate::codegen::rust) fn generate_async_call(&mut self, expr: &Expression<'ast>) -> String {
        self.generate_task("local", expr)
    }

    /// `spawn f(x)` / `spawn { }` (ADR-0004): a task on its own thread.
    pub(in crate::codegen::rust) fn generate_spawn_call(&mut self, expr: &Expression<'ast>) -> String {
        self.generate_task("spawn", expr)
    }

    /// `scope { body }` (ADR-0004): a guard whose drop (at the block's end, also on `return`/`?`/`break`) waits for
    /// every task started inside and re-raises the first panic; the block's value is the scope's.
    pub(in crate::codegen::rust) fn generate_scope(&mut self, body: &Expression<'ast>) -> String {
        let inner = self.generate_expression(body);
        format!("{{ let _wj_scope = windjammer_runtime::task::Scope::enter(); {inner} }}")
    }

    /// `select { arms }` (ADR-0004): sources evaluated once in order, `task::select` returns the first ready one
    /// (or the `else` arm), then that arm binds its pattern and runs.
    pub(in crate::codegen::rust) fn generate_select(&mut self, arms: &[crate::parser::SelectArm<'ast>]) -> String {
        use crate::parser::SelectKind as K;
        let mut out = String::from("{ ");
        let mut sources = Vec::new();
        let mut cases = Vec::new();
        let mut else_body = None;
        for arm in arms {
            let Some(src) = arm.source else {
                else_body = Some(self.generate_expression(arm.body));
                continue;
            };
            let i = sources.len();
            let var = format!("__wj_sel{i}");
            let src_code = self.generate_expression(src);
            let take = match arm.kind {
                K::Timeout => {
                    out.push_str(&format!("let {var} = windjammer_runtime::task::Deadline::after({src_code}); "));
                    sources.push(format!("&{var}"));
                    String::new()
                }
                K::Recv => {
                    out.push_str(&format!("let {var} = &({src_code}); "));
                    sources.push(var.clone());
                    format!("let {} = {var}.recv(); ", self.pattern_to_rust(&arm.pattern))
                }
                _ => {
                    out.push_str(&format!("let {var} = &({src_code}); "));
                    sources.push(var.clone());
                    format!("let {} = {var}.wait(); ", self.pattern_to_rust(&arm.pattern))
                }
            };
            let body = self.generate_expression(arm.body);
            cases.push(format!("{i} => {{ {take}{body} }}"));
        }
        let n = sources.len();
        if n == 0 {
            return format!("{{ {} }}", else_body.unwrap_or_else(|| "()".to_string()));
        }
        let list: Vec<String> = sources
            .iter()
            .enumerate()
            .map(|(i, s)| if i == 0 { format!("{s} as &dyn windjammer_runtime::task::Selectable") } else { s.clone() })
            .collect();
        out.push_str(&format!("match windjammer_runtime::task::select(&[{}], {}) {{ ", list.join(", "), else_body.is_some()));
        for c in cases {
            out.push_str(&c);
            out.push_str(", ");
        }
        if let Some(e) = else_body {
            out.push_str(&format!("{n} => {{ {e} }}, "));
        }
        out.push_str("_ => unreachable!(\"select\"), } }");
        out
    }

    /// Generate code for range expression (start..end or start..=end)
    /// TDD FIX: Range type unification for 0..vec.len()
    pub(in crate::codegen::rust) fn generate_range(
        &mut self,
        start: &Expression<'ast>,
        end: &Expression<'ast>,
        inclusive: bool,
    ) -> String {
        // If end returns usize (registry: len/capacity/…), cast start to usize.
        let end_is_usize = match end {
            Expression::MethodCall { object, method, .. } => {
                crate::codegen::rust::stdlib_method_traits::method_returns_usize_qualified(
                    method,
                    self.infer_expression_type(object)
                        .as_ref()
                        .and_then(Self::type_to_name)
                        .as_deref(),
                    &self.signature_registry,
                )
            }
            _ => false,
        } || self.expression_produces_usize(end);

        // P3.280: peer-drive int literal suffixes from the other bound's type
        // (`(cx - 16)..(cx + 16)` with cx:i32 → `16_i32`, not default `_i64`).
        // Prefer specific widths over WJ-default `int` when one bound is a typed
        // const/local (`0..LOGICAL_SIZE` with LOGICAL_SIZE:i32). Pure-literal
        // ranges (`0..2`) default to i32 (Rust unsuffixed).
        let prev_range_int = self.assignment_int_target_type.clone();
        if self.assignment_int_target_type.is_none() {
            let start_ty = self.infer_expression_type(start);
            let end_ty = self.infer_expression_type(end);
            let bound_ty = match (start_ty, end_ty) {
                (Some(a), Some(b)) if a != b => {
                    if matches!(a, Type::Int)
                        && Self::assignment_target_needs_int_codegen_context(&b)
                        && !matches!(b, Type::Int)
                    {
                        Some(b)
                    } else if matches!(b, Type::Int)
                        && Self::assignment_target_needs_int_codegen_context(&a)
                        && !matches!(a, Type::Int)
                    {
                        Some(a)
                    } else if Self::assignment_target_needs_int_codegen_context(&a) {
                        Some(a)
                    } else if Self::assignment_target_needs_int_codegen_context(&b) {
                        Some(b)
                    } else {
                        None
                    }
                }
                (Some(t), _) | (_, Some(t))
                    if Self::assignment_target_needs_int_codegen_context(&t)
                        && Self::int_type_from_assignment_target(&t).is_some() =>
                {
                    Some(t)
                }
                _ => None,
            };
            let both_literal_ints = matches!(
                start,
                Expression::Literal {
                    value: crate::parser::Literal::Int(_),
                    ..
                }
            ) && matches!(
                end,
                Expression::Literal {
                    value: crate::parser::Literal::Int(_),
                    ..
                }
            );
            if both_literal_ints {
                self.assignment_int_target_type = Some(Type::Int32);
            } else if let Some(t) = bound_ty {
                // The literal must take the bound's *emitted* width. WJ `uint` is emitted as
                // `u64` (types.rs), whereas the generic literal-suffix mapping for `Type::Uint`
                // is u32, so name the width explicitly.
                self.assignment_int_target_type = Some(if matches!(t, Type::Uint) {
                    Type::Custom("u64".to_string())
                } else {
                    t
                });
            }
        }

        let mut start_str = self.generate_expression(start);

        if end_is_usize {
            if start_str.ends_with("_i32") {
                // Replace _i32 with _usize for literals
                start_str = start_str.replace("_i32", "_usize");
            } else if matches!(
                start,
                Expression::Identifier { .. } | Expression::Binary { .. }
            ) && !start_str.contains(" as usize")
                // peer-driven `1_usize` / usize-typed `(i + 1)` must not
                // become `1_usize as usize` / `(i + 1_usize as usize)`.
                && !start_str.contains("_usize")
                && !self.expression_produces_usize(start)
            {
                // Cast the whole bound — `as` binds tighter than `+`, so
                // `(i + 1 as usize)` is wrong; use `(i + 1) as usize`.
                if matches!(start, Expression::Binary { .. }) {
                    start_str = format!("({}) as usize", start_str);
                } else {
                    start_str = format!("{} as usize", start_str);
                }
            }
        }

        let end_str = self.generate_expression(end);
        self.assignment_int_target_type = prev_range_int;
        if inclusive {
            format!("{}..={}", start_str, end_str)
        } else {
            format!("{}..{}", start_str, end_str)
        }
    }
}
