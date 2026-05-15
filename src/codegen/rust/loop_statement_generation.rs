//! Loop, while, break, and continue statement generation.

use crate::parser::*;

use super::CodeGenerator;

impl<'ast> CodeGenerator<'ast> {
    pub(in crate::codegen::rust) fn generate_loop_statement(
        &mut self,
        body: &[&'ast Statement<'ast>],
    ) -> String {
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

    pub(in crate::codegen::rust) fn generate_while_statement(
        &mut self,
        condition: &'ast Expression<'ast>,
        body: &[&'ast Statement<'ast>],
    ) -> String {
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
