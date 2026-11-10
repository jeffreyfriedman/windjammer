//! Const and static statement code generation.

use crate::parser::*;

use super::{expression_helpers, CodeGenerator};

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

    /// A module-level `const`/`static` declaration line (top level or inside an inline `mod`).
    pub(in crate::codegen::rust) fn generate_const_or_static_item(&mut self, item: &Item<'ast>) -> String {
        match item {
            Item::Const { name, is_pub, type_, value, .. } => {
                let pub_prefix = if *is_pub { "pub " } else { "" };
                // string constants are `&'static str`, not `String`
                let rust_type = if matches!(type_, Type::String) && matches!(value, Expression::Literal { value: Literal::String(_), .. }) {
                    "&'static str".to_string()
                } else {
                    self.type_to_rust(type_)
                };
                format!("{pub_prefix}const {name}: {rust_type} = {};\n", self.generate_expression_immut(value))
            }
            Item::Static { name, mutable, type_, value, .. } => {
                let ty = self.type_to_rust(type_);
                let v = self.generate_expression_immut(value);
                if *mutable {
                    format!("static mut {name}: {ty} = {v};\n")
                } else {
                    // a compile-time evaluable static is promoted to `const`
                    let keyword = if expression_helpers::is_const_evaluable(value) { "const" } else { "static" };
                    format!("{keyword} {name}: {ty} = {v};\n")
                }
            }
            _ => String::new(),
        }
    }
}
