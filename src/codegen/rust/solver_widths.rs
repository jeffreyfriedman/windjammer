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
    /// NUM-USIZE-TO-INT: print the conversions the solver placed on this expression — an index source
    /// of another width (`(v.len() as i64)`) or a `usize` value stored into a wider slot.
    pub(in crate::codegen::rust) fn apply_solver_conversion(&self, expr: &Expression<'ast>, emitted: String) -> String {
        let Some(f) = self.current_ir_function.as_ref() else { return emitted };
        let target = match expr {
            Expression::MethodCall { arguments, .. } if arguments.is_empty() => crate::ir::numeric_defaults::literal_key(expr)
                .and_then(|k| f.index_source_widths.get(&k))
                .filter(|w| **w != BaseType::Usize),
            _ => None,
        }
        .or_else(|| crate::ir::numeric_facts::conversion_key(expr).and_then(|k| f.usize_conversions.get(&k)));
        match target.and_then(rust_int_name) {
            Some(t) if !emitted.ends_with(&format!(" as {t})")) => {
                // `as` binds tighter than any binary operator: `(i + 1) as i64`, not `i + 1 as i64`.
                if matches!(expr, Expression::Binary { .. } | Expression::Unary { .. }) {
                    format!("(({emitted}) as {t})")
                } else {
                    format!("({emitted} as {t})")
                }
            }
            _ => emitted,
        }
    }

    /// Integer width of a block tail: a tail is unified with its block's (or the function's) value, so
    /// when the solver fixed its width there is nothing to cast; otherwise the legacy W5 casts to the
    /// function's `int`/`i32` return width apply.
    pub(in crate::codegen::rust) fn cast_tail_to_return_int_width(&self, expr_str: &mut String, expr: &Expression<'ast>) {
        if self.solver_operand_width(expr).is_some() {
            return;
        }
        let target = match &self.current_function_return_type {
            Some(Type::Int) => Some("int"),
            Some(Type::Custom(name)) if name == "i64" || name == "int" => Some("int"),
            Some(Type::Int32) => Some("i32"),
            Some(Type::Custom(name)) if name == "i32" => Some("i32"),
            _ => None,
        };
        self.maybe_cast_usize_to_int_target(expr_str, expr, target);
        self.maybe_cast_to_function_return_int_width(expr_str, expr);
        crate::codegen::rust::type_classification_utilities::strip_redundant_same_width_int_cast(expr_str);
    }

    /// Width the solver fixed for an integer literal (NUM-LITERAL-WIDTH, any width).
    pub(in crate::codegen::rust) fn solver_literal_width(&self, lit: &Expression<'ast>) -> Option<&BaseType> {
        let f = self.current_ir_function.as_ref()?;
        f.literal_widths.get(&crate::ir::numeric_defaults::literal_key(lit)?)
    }

    /// Width the solver fixed for a singly bound local (NUM-LOCAL-WIDTH).
    pub(in crate::codegen::rust) fn solver_local_width(&self, name: &str) -> Option<&BaseType> {
        self.current_ir_function.as_ref()?.solver_local_widths.get(name)
    }

    /// ADR-0028: both operands of a binary operator have one solver width, so there is nothing to
    /// decide and no cast to place (the W3 peer/promotion deciders are skipped).
    pub(in crate::codegen::rust) fn solver_agrees(&self, left: &Expression<'ast>, right: &Expression<'ast>) -> bool {
        self.solver_operand_width(left).is_some_and(|l| self.solver_operand_width(right) == Some(l))
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
