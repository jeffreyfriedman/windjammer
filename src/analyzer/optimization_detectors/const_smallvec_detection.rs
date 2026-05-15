//! Const-evaluability checks, const/static stub, and SmallVec literal sizing.

use crate::parser::*;

use super::super::{
    AnalyzedFunction, Analyzer, ConstStaticOptimization, SmallVecOptimization,
};

impl<'ast> Analyzer<'ast> {
    pub(in crate::analyzer) fn detect_const_static_opportunities(
        &self,
        _func: &AnalyzedFunction,
    ) -> Vec<ConstStaticOptimization> {
        Vec::new()
    }

    #[allow(clippy::only_used_in_recursion)]
    pub(in crate::analyzer) fn is_const_evaluable(&self, expr: &Expression) -> bool {
        match expr {
            Expression::Literal { .. } => true,
            Expression::Binary { left, right, .. } => {
                self.is_const_evaluable(left) && self.is_const_evaluable(right)
            }
            Expression::Unary { operand, .. } => self.is_const_evaluable(operand),
            Expression::StructLiteral { fields, .. } => {
                fields.iter().all(|(_, expr)| self.is_const_evaluable(expr))
            }
            Expression::Identifier { .. } => false,
            Expression::Call { .. } => false,
            Expression::FieldAccess { .. } => false,
            Expression::MethodCall { .. } => false,
            _ => false,
        }
    }

    pub(in crate::analyzer) fn detect_smallvec_opportunities(
        &self,
        func: &FunctionDecl,
    ) -> Vec<SmallVecOptimization> {
        let mut optimizations = Vec::new();

        for stmt in &func.body {
            self.detect_smallvec_in_statement(stmt, &mut optimizations);
        }

        optimizations
    }

    pub(crate) fn detect_smallvec_in_statement(
        &self,
        stmt: &Statement,
        optimizations: &mut Vec<SmallVecOptimization>,
    ) {
        if let Statement::Let {
            pattern: Pattern::Identifier(name),
            value,
            ..
        } = stmt
        {
            if let Some(size) = self.estimate_vec_literal_size(value) {
                if size <= 8 {
                    let stack_size = size.next_power_of_two().max(4);
                    optimizations.push(SmallVecOptimization {
                        variable: name.clone(),
                        estimated_max_size: size,
                        stack_size,
                    });
                }
            }
        }
    }

    pub(crate) fn estimate_vec_literal_size(&self, expr: &Expression) -> Option<usize> {
        match expr {
            Expression::MacroInvocation {
                name,
                args,
                delimiter,
                ..
            } if name == "vec" && *delimiter == MacroDelimiter::Brackets => Some(args.len()),

            Expression::MethodCall {
                object,
                method,
                arguments,
                ..
            } if method == "new" && arguments.is_empty() => {
                if let Expression::Identifier { name, .. } = object {
                    if name == "Vec" {
                        return Some(0);
                    }
                }
                None
            }

            Expression::Call {
                function,
                arguments,
                ..
            } => {
                if let Expression::FieldAccess { object, field, .. } = function {
                    if let Expression::Identifier { name, .. } = object {
                        if name == "Vec" && field == "with_capacity" {
                            if let Some((_, arg)) = arguments.first() {
                                return self.extract_literal_int(arg);
                            }
                        }
                    }
                }
                None
            }

            Expression::MethodCall { object, method, .. } if method == "collect" => {
                if let Expression::Range { start, end, .. } = object {
                    let start_val = self.extract_literal_int(start).unwrap_or(0);
                    let end_val = self.extract_literal_int(end)?;
                    return Some(end_val - start_val);
                }
                None
            }

            _ => None,
        }
    }

    pub(crate) fn extract_literal_int(&self, expr: &Expression) -> Option<usize> {
        match expr {
            Expression::Literal {
                value: Literal::Int(n),
                ..
            } if *n >= 0 => Some(*n as usize),
            _ => None,
        }
    }
}
