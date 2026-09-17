//! Async and concurrency expression generation
//!
//! Handles generation of:
//! - Await expressions (.await)
//! - Channel operations (send/recv)
//! - Range expressions (start..end, start..=end)

use crate::parser::{Expression, Type};

use super::CodeGenerator;

impl<'ast> CodeGenerator<'ast> {
    pub(in crate::codegen::rust) fn generate_await(&mut self, expr: &Expression<'ast>) -> String {
        format!("{}.await", self.generate_expression(expr))
    }

    /// Generate caller-controlled async call: `async fetch_user(1)` → `async move { fetch_user(1) }`
    pub(in crate::codegen::rust) fn generate_async_call(
        &mut self,
        expr: &Expression<'ast>,
    ) -> String {
        format!("async move {{ {} }}", self.generate_expression(expr))
    }

    /// Generate caller-controlled spawn call: `spawn fetch_user(1)` → `tokio::spawn(async move { ... })`
    pub(in crate::codegen::rust) fn generate_spawn_call(
        &mut self,
        expr: &Expression<'ast>,
    ) -> String {
        format!(
            "tokio::spawn(async move {{ {} }})",
            self.generate_expression(expr)
        )
    }

    /// Generate code for channel send expression (channel.send(value))
    pub(in crate::codegen::rust) fn generate_channel_send(
        &mut self,
        channel: &Expression<'ast>,
        value: &Expression<'ast>,
    ) -> String {
        let ch_str = self.generate_expression(channel);
        let val_str = self.generate_expression(value);
        format!("{}.send({})", ch_str, val_str)
    }

    /// Generate code for channel receive expression (channel.recv())
    pub(in crate::codegen::rust) fn generate_channel_recv(
        &mut self,
        channel: &Expression<'ast>,
    ) -> String {
        let ch_str = self.generate_expression(channel);
        format!("{}.recv()", ch_str)
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
            Expression::MethodCall {
                object, method, ..
            } => crate::codegen::rust::stdlib_method_traits::method_returns_usize_qualified(
                method,
                self.infer_expression_type(object)
                    .as_ref()
                    .and_then(Self::type_to_name)
                    .as_deref(),
                &self.signature_registry,
            ),
            _ => false,
        } || self.expression_produces_usize(end);
        let i32_scan_len_range =
            self.function_returns_i32_for_loop_scan() && end_is_usize;

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
                self.assignment_int_target_type = if matches!(t, Type::Int) {
                    Some(Type::Int32)
                } else {
                    Some(t)
                };
            }
        }

        let mut start_str = self.generate_expression(start);

        // P3.337: `-> i32` + `for i in 0..vec.len()` keeps i32 counter; cast len end, not start.
        if i32_scan_len_range {
            if start_str.ends_with("_usize") {
                start_str = start_str.replace("_usize", "_i32");
            }
        } else if end_is_usize {
            if start_str.ends_with("_i32") {
                // Replace _i32 with _usize for literals
                start_str = start_str.replace("_i32", "_usize");
            } else if matches!(
                start,
                Expression::Identifier { .. } | Expression::Binary { .. }
            ) && !start_str.contains("as usize")
            {
                // Add cast for identifiers or expressions without existing cast
                if matches!(start, Expression::Binary { .. }) {
                    start_str = format!("({} as usize)", start_str);
                } else {
                    start_str = format!("{} as usize", start_str);
                }
            }
        }

        let mut end_str = self.generate_expression(end);
        if i32_scan_len_range && !end_str.contains(" as i32") {
            end_str = format!("({} as i32)", end_str);
        }
        self.assignment_int_target_type = prev_range_int;
        if inclusive {
            format!("{}..={}", start_str, end_str)
        } else {
            format!("{}..{}", start_str, end_str)
        }
    }
}
