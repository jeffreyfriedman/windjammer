//! Const and static statement code generation.

use crate::parser::*;

use super::CodeGenerator;

impl<'ast> CodeGenerator<'ast> {
    pub(in crate::codegen::rust) fn generate_const_statement(
        &mut self,
        name: &str,
        type_: &Type,
        value: &'ast Expression<'ast>,
    ) -> String {
        let mut output = self.indent();

        let rust_type = if matches!(type_, Type::String)
            && matches!(
                value,
                Expression::Literal {
                    value: Literal::String(_),
                    ..
                }
            ) {
            "&'static str".to_string()
        } else {
            self.type_to_rust(type_)
        };

        output.push_str(&format!(
            "const {}: {} = {};\n",
            name,
            rust_type,
            self.generate_expression(value)
        ));
        output
    }

    pub(in crate::codegen::rust) fn generate_static_statement(
        &mut self,
        name: &str,
        mutable: bool,
        type_: &Type,
        value: &'ast Expression<'ast>,
    ) -> String {
        let mut output = self.indent();
        if mutable {
            output.push_str(&format!(
                "static mut {}: {} = {};\n",
                name,
                self.type_to_rust(type_),
                self.generate_expression(value)
            ));
        } else {
            output.push_str(&format!(
                "static {}: {} = {};\n",
                name,
                self.type_to_rust(type_),
                self.generate_expression(value)
            ));
        }
        output
    }
}
