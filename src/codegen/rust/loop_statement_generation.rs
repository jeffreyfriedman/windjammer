//! Loop, while, break, and continue statement generation.

use crate::parser::*;

use super::{operators, CodeGenerator};

impl<'ast> CodeGenerator<'ast> {
    pub(in crate::codegen::rust) fn generate_loop_statement(
        &mut self,
        body: &[&'ast Statement<'ast>],
    ) -> String {
        let mut output = self.indent();
        output.push_str("loop {\n");

        self.indent_level += 1;
        self.loop_body_depth += 1;
        let saved_idx = self.current_statement_idx;
        let saved_local_idx = self.current_block_local_idx;
        let old_void_block = self.in_void_block;
        self.in_void_block = true;
        for (i, stmt) in body.iter().enumerate() {
            self.current_statement_idx = self.auto_clone_counter;
            self.current_block_local_idx = i;
            self.auto_clone_counter += 1;
            output.push_str(&self.generate_statement(stmt));
        }
        self.current_statement_idx = saved_idx;
        self.current_block_local_idx = saved_local_idx;
        self.in_void_block = old_void_block;
        self.loop_body_depth = self.loop_body_depth.saturating_sub(1);
        self.indent_level -= 1;

        output.push_str(&self.indent());
        output.push_str("}\n");
        output
    }

    pub(in crate::codegen::rust) fn generate_while_statement(
        &mut self,
        condition: &'ast Expression<'ast>,
        body: &[&'ast Statement<'ast>],
    ) -> String {
        self.mark_usize_variables_in_condition(condition);
        self.promote_ambiguous_int_loop_counter_in_while_condition(condition);

        let mut output = self.indent();
        output.push_str("while ");

        let prev_while_int = self.assignment_int_target_type.clone();
        if let Expression::Binary { left, right, .. } = condition {
            for id_expr in [left, right] {
                if let Expression::Identifier { name, .. } = id_expr {
                    let is_i32_local = matches!(
                        self.local_var_types.get(name.as_str()),
                        Some(Type::Int32)
                    ) || matches!(
                        self.local_var_types.get(name.as_str()),
                        Some(Type::Custom(n)) if n == "i32"
                    );
                    if is_i32_local {
                        self.assignment_int_target_type = Some(Type::Int32);
                        break;
                    }
                    let is_u32_local = matches!(
                        self.local_var_types.get(name.as_str()),
                        Some(Type::Uint)
                    ) || matches!(
                        self.local_var_types.get(name.as_str()),
                        Some(Type::Custom(n)) if n == "u32"
                    );
                    if is_u32_local {
                        self.assignment_int_target_type = Some(Type::Uint);
                        break;
                    }
                }
            }
        }

        let mut condition_str = self.generate_expression(condition);
        self.assignment_int_target_type = prev_while_int;
        if let Expression::Binary { left, op, right, .. } = condition {
                if matches!(
                    op,
                    BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge
                ) {
                    let bound_is_len = matches!(
                        right,
                        Expression::MethodCall { method, .. }
                            if method == "len" || method == "capacity"
                    ) || self.expression_produces_usize(right);
                    if bound_is_len || condition_str.contains(".len()") {
                        if let Expression::Identifier { name, .. } = left {
                            let param_is_wj_int = self.current_function_params.iter().any(|p| {
                                p.name == *name && matches!(&p.type_, Type::Int)
                            });
                            let local_is_i64 = self.local_var_types.get(name).is_some_and(|t| {
                                matches!(t, Type::Int)
                                    || matches!(t, Type::Custom(n) if n == "int" || n == "i64")
                            });
                            if !param_is_wj_int
                                && !local_is_i64
                                && !condition_str.contains(" as usize")
                            {
                                self.usize_variables.remove(name);
                                let left_str = self.generate_expression(left);
                                let right_str = self.generate_expression(right);
                                let op_str = operators::binary_op_to_rust(op);
                                condition_str =
                                    format!("({left_str} as usize) {op_str} {right_str}");
                            }
                        }
                    }
                }
            }
        output.push_str(&condition_str);
        output.push_str(" {\n");

        self.indent_level += 1;
        self.loop_body_depth += 1;
        let saved_body = self.current_function_body.clone();
        let saved_idx = self.current_statement_idx;
        let saved_local_idx = self.current_block_local_idx;
        let old_void_block = self.in_void_block;
        self.in_void_block = true;
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
        self.in_void_block = old_void_block;
        self.loop_body_depth = self.loop_body_depth.saturating_sub(1);
        self.indent_level -= 1;

        output.push_str(&self.indent());
        output.push_str("}\n");
        output
    }

    pub(in crate::codegen::rust) fn generate_break_statement(&mut self) -> String {
        let mut output = self.indent();
        output.push_str("break;\n");
        output
    }

    pub(in crate::codegen::rust) fn generate_continue_statement(&mut self) -> String {
        let mut output = self.indent();
        output.push_str("continue;\n");
        output
    }
}
