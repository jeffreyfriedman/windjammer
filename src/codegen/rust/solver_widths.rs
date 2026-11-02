//! Integer widths the numeric solver fixed (ADR-0028), read by the Rust backend.
//!
//! The solver publishes per-function facts on `IrFunction` (`literal_widths`,
//! `solver_local_widths`, `index_source_widths`, keyed by source position or local name). This
//! module is the backend's only reader: it answers "which width did the solver give this
//! operand", and the legacy width deciders run only where it answers `None`.

use crate::codegen::rust::CodeGenerator;
use crate::ir::safety_type::BaseType;
use crate::parser::{Expression, Literal, Type, UnaryOp};

/// Rust spelling of an integer `BaseType` (`None` for non-integers).
pub(crate) fn rust_int_name(base: &BaseType) -> Option<&'static str> {
    Some(match base {
        BaseType::I8 => "i8",
        BaseType::I16 => "i16",
        BaseType::I32 => "i32",
        BaseType::I64 => "i64",
        BaseType::I128 => "i128",
        BaseType::Isize => "isize",
        BaseType::U8 => "u8",
        BaseType::U16 => "u16",
        BaseType::U32 => "u32",
        BaseType::U64 => "u64",
        BaseType::U128 => "u128",
        BaseType::Usize => "usize",
        _ => return None,
    })
}

/// The parser type the legacy codegen state uses for an integer width (`int` for i64).
pub(crate) fn int_width_as_type(base: &BaseType) -> Option<Type> {
    Some(match base {
        BaseType::I64 => Type::Int,
        BaseType::I32 => Type::Int32,
        other => Type::Custom(rust_int_name(other)?.to_string()),
    })
}

impl<'ast> CodeGenerator<'ast> {
    /// Width the solver fixed for an integer literal (NUM-LITERAL-WIDTH, any width).
    pub(in crate::codegen::rust) fn solver_literal_width(&self, lit: &Expression<'ast>) -> Option<&BaseType> {
        let f = self.current_ir_function.as_ref()?;
        f.literal_widths.get(&crate::ir::numeric_defaults::literal_key(lit)?)
    }

    /// Width the solver fixed for a singly bound local (NUM-LOCAL-WIDTH).
    pub(in crate::codegen::rust) fn solver_local_width(&self, name: &str) -> Option<&BaseType> {
        self.current_ir_function.as_ref()?.solver_local_widths.get(name)
    }

    /// Width of an integer operand as the solver fixed it, when every part of it has a fact:
    /// literals, published locals, `len()`-like index sources that stay `usize`, casts, and
    /// arithmetic over operands of one width. `None` hands the decision to the legacy path.
    pub(in crate::codegen::rust) fn solver_operand_width(&self, expr: &Expression<'ast>) -> Option<BaseType> {
        match expr {
            Expression::Literal { value: Literal::Int(_), .. } => self.solver_literal_width(expr).cloned(),
            Expression::Identifier { name, .. } => self.solver_local_width(name).cloned(),
            Expression::MethodCall { .. } => {
                let f = self.current_ir_function.as_ref()?;
                f.index_source_widths.get(&crate::ir::numeric_defaults::literal_key(expr)?).cloned()
            }
            Expression::Unary { op: UnaryOp::Neg, operand, .. } => self.solver_operand_width(operand),
            Expression::Binary { left, op, right, .. } => {
                use crate::parser::BinaryOp::*;
                if !matches!(op, Add | Sub | Mul | Div | Mod | BitAnd | BitOr | BitXor) {
                    return None;
                }
                let l = self.solver_operand_width(left)?;
                (self.solver_operand_width(right)? == l).then_some(l)
            }
            _ => None,
        }
    }
}
