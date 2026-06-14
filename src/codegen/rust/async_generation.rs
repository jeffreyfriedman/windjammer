//! Async and concurrency expression generation
//!
//! Handles generation of:
//! - Await expressions (.await)
//! - Channel operations (send/recv)
//! - Range expressions (start..end, start..=end)

use crate::parser::Expression;

use super::CodeGenerator;

impl<'ast> CodeGenerator<'ast> {
    pub(in crate::codegen::rust) fn generate_await(&mut self, expr: &Expression<'ast>) -> String {
        format!("{}.await", self.generate_expression(expr))
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
        // If end is .len() (returns usize), cast start to usize to avoid type mismatch
        let end_is_len = matches!(
            end,
            Expression::MethodCall { method, .. }
                if matches!(method.as_str(), "len" | "capacity" | "count")
        );

        let mut start_str = self.generate_expression(start);

        // If end is .len() and start has _i32 suffix, replace with _usize or add cast
        if end_is_len {
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

        let end_str = self.generate_expression(end);
        if inclusive {
            format!("{}..={}", start_str, end_str)
        } else {
            format!("{}..{}", start_str, end_str)
        }
    }
}
