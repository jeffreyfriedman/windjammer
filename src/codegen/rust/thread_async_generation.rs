//! Thread and async block statement generation (`std::thread::spawn`, `tokio::spawn`).

use crate::parser::*;

use super::CodeGenerator;

impl<'ast> CodeGenerator<'ast> {
    pub(in crate::codegen::rust) fn generate_thread_statement(
        &mut self,
        body: &[&'ast Statement<'ast>],
    ) -> String {
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

    pub(in crate::codegen::rust) fn generate_async_statement(
        &mut self,
        body: &[&'ast Statement<'ast>],
    ) -> String {
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
}
