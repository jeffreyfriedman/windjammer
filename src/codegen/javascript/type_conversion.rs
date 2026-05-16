//! JavaScript type and operator mapping helpers

use crate::parser::{BinaryOp, Type, UnaryOp};

/// TDD FIX: Escape JavaScript reserved keywords
pub(crate) fn escape_js_keyword(name: &str) -> String {
    match name {
        "break" | "case" | "catch" | "class" | "const" | "continue" | "debugger" | "default"
        | "delete" | "do" | "else" | "export" | "extends" | "finally" | "for" | "function"
        | "if" | "import" | "in" | "instanceof" | "let" | "new" | "return" | "super" | "switch"
        | "this" | "throw" | "try" | "typeof" | "var" | "void" | "while" | "with" | "yield"
        | "async" | "await" | "enum" | "implements" | "interface" | "package" | "private"
        | "protected" | "public" | "static" => {
            format!("{}_", name) // Append underscore to avoid keyword conflict
        }
        _ => name.to_string(),
    }
}

pub(crate) fn type_to_jsdoc(ty: &Type) -> String {
    match ty {
        Type::Int | Type::Int32 | Type::Uint | Type::Float => "number".to_string(),
        Type::String => "string".to_string(),
        Type::Bool => "boolean".to_string(),
        Type::Custom(name) => name.clone(),
        Type::Generic(name) => name.clone(),
        Type::Vec(inner) => format!("Array<{}>", type_to_jsdoc(inner)),
        Type::Option(inner) => format!("{}|null", type_to_jsdoc(inner)),
        _ => "any".to_string(),
    }
}

pub(crate) fn binary_op_to_js(op: &BinaryOp) -> String {
    match op {
        BinaryOp::Add => "+",
        BinaryOp::Sub => "-",
        BinaryOp::Mul => "*",
        BinaryOp::Div => "/",
        BinaryOp::Mod => "%",
        BinaryOp::Eq => "===",
        BinaryOp::Ne => "!==",
        BinaryOp::Lt => "<",
        BinaryOp::Le => "<=",
        BinaryOp::Gt => ">",
        BinaryOp::Ge => ">=",
        BinaryOp::And => "&&",
        BinaryOp::Or => "||",
        BinaryOp::BitAnd => "&",
        BinaryOp::BitOr => "|",
        BinaryOp::BitXor => "^",
        BinaryOp::Shl => "<<",
        BinaryOp::Shr => ">>",
    }
    .to_string()
}

pub(crate) fn unary_op_to_js(op: &UnaryOp) -> String {
    match op {
        UnaryOp::Not => "!".to_string(),
        UnaryOp::Neg => "-".to_string(),
        UnaryOp::Ref => "".to_string(), // & doesn't apply in JS (auto-reference)
        UnaryOp::MutRef => "".to_string(), // &mut doesn't apply in JS (auto-reference)
        UnaryOp::Deref => "".to_string(), // * doesn't apply in JS
    }
}
