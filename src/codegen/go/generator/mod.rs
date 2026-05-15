//! Go code generator
//!
//! Generates idiomatic Go source code from the Windjammer AST.
//!
//! Implementation is split across `type_generation.rs`, `function_generation.rs`,
//! `statement_generation.rs`, and `expression_generation.rs` (included below) so this
//! file stays manageable while preserving a single module for visibility rules.

use crate::codegen::backend::{CodegenBackend, CodegenConfig, CodegenOutput, Target};
use crate::parser::{
    BinaryOp, CompoundOp, EnumPatternBinding, Expression, FunctionDecl, Item, Literal, MatchArm,
    Pattern, Program, Statement, Type, UnaryOp,
};
use anyhow::Result;

/// Go code generation backend
pub struct GoBackend;

impl GoBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GoBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl CodegenBackend for GoBackend {
    fn name(&self) -> &str {
        "Go"
    }

    fn target(&self) -> Target {
        Target::Go
    }

    fn generate(&self, program: &Program, _config: &CodegenConfig) -> Result<CodegenOutput> {
        let mut gen = GoGenerator::new();
        let code = gen.generate_program(program);
        Ok(CodegenOutput::new(code, "go".to_string()))
    }

    fn generate_additional_files(
        &self,
        _program: &Program,
        _config: &CodegenConfig,
    ) -> Vec<(String, String)> {
        let go_mod = "module windjammer-generated\n\ngo 1.21\n".to_string();
        vec![("go.mod".to_string(), go_mod)]
    }
}

/// Internal Go code generator
struct GoGenerator {
    indent_level: usize,
    needs_fmt_import: bool,
    needs_math_import: bool,
    declared_structs: Vec<String>,
    declared_vars: Vec<std::collections::HashSet<String>>,
    declared_enums: std::collections::HashMap<String, Vec<String>>,
}

impl GoGenerator {
    fn new() -> Self {
        Self {
            indent_level: 0,
            needs_fmt_import: false,
            needs_math_import: false,
            declared_structs: Vec::new(),
            declared_vars: vec![std::collections::HashSet::new()],
            declared_enums: std::collections::HashMap::new(),
        }
    }

    fn escape_go_keyword(name: &str) -> String {
        match name {
            "break" | "case" | "chan" | "const" | "continue" | "default" | "defer" | "else"
            | "fallthrough" | "for" | "func" | "go" | "goto" | "if" | "import" | "interface"
            | "map" | "package" | "range" | "return" | "select" | "struct" | "switch" | "type"
            | "var" => format!("{}_", name),
            _ => name.to_string(),
        }
    }

    fn op_precedence(op: &BinaryOp) -> i32 {
        match op {
            BinaryOp::Or => 1,
            BinaryOp::And => 2,
            BinaryOp::BitOr => 3,
            BinaryOp::BitXor => 4,
            BinaryOp::BitAnd => 5,
            BinaryOp::Eq | BinaryOp::Ne => 6,
            BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge => 7,
            BinaryOp::Shl | BinaryOp::Shr => 8,
            BinaryOp::Add | BinaryOp::Sub => 9,
            BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod => 10,
        }
    }

    fn push_scope(&mut self) {
        self.declared_vars.push(std::collections::HashSet::new());
    }

    fn pop_scope(&mut self) {
        self.declared_vars.pop();
    }

    fn is_var_declared(&self, name: &str) -> bool {
        self.declared_vars.iter().any(|scope| scope.contains(name))
    }

    fn declare_var(&mut self, name: &str) {
        if let Some(scope) = self.declared_vars.last_mut() {
            scope.insert(name.to_string());
        }
    }

    fn capitalize(s: &str) -> String {
        let mut c = s.chars();
        match c.next() {
            None => String::new(),
            Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        }
    }

    fn indent(&self) -> String {
        "\t".repeat(self.indent_level)
    }
}

include!("type_generation.rs");
include!("function_generation.rs");
include!("statement_generation.rs");
include!("expression_generation.rs");

fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Type;

    #[test]
    fn test_go_backend_creation() {
        let backend = GoBackend::new();
        assert_eq!(backend.name(), "Go");
        assert_eq!(backend.target(), Target::Go);
    }

    #[test]
    fn test_capitalize_first() {
        assert_eq!(capitalize_first("hello"), "Hello");
        assert_eq!(capitalize_first("x"), "X");
        assert_eq!(capitalize_first(""), "");
        assert_eq!(capitalize_first("Main"), "Main");
    }

    #[test]
    fn test_type_mapping() {
        let gen = GoGenerator::new();
        assert_eq!(gen.type_to_go(&Type::Int), "int64");
        assert_eq!(gen.type_to_go(&Type::Float), "float64");
        assert_eq!(gen.type_to_go(&Type::Bool), "bool");
        assert_eq!(gen.type_to_go(&Type::String), "string");
    }

    #[test]
    fn test_empty_program() {
        let program = Program { items: vec![] };
        let mut gen = GoGenerator::new();
        let code = gen.generate_program(&program);
        assert!(code.contains("package main"));
    }
}
