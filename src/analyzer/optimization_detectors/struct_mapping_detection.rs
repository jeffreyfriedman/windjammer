//! Struct literal → source mapping opportunities (direct mapping, FromRow, etc.).

use crate::parser::*;
use std::collections::HashMap;

use super::super::{Analyzer, MappingStrategy, StructMappingOptimization};

impl<'ast> Analyzer<'ast> {
    pub(in crate::analyzer) fn detect_struct_mappings(
        &self,
        func: &FunctionDecl,
    ) -> Vec<StructMappingOptimization> {
        let mut optimizations = Vec::new();

        for stmt in &func.body {
            self.analyze_statement_for_struct_mappings(stmt, &mut optimizations);
        }

        optimizations
    }

    /// Helper to analyze statements for struct mapping patterns
    pub(crate) fn analyze_statement_for_struct_mappings(
        &self,
        stmt: &Statement,
        optimizations: &mut Vec<StructMappingOptimization>,
    ) {
        match stmt {
            Statement::Let { value, .. }
            | Statement::Return {
                value: Some(value), ..
            } => {
                self.analyze_expression_for_struct_mappings(value, optimizations);
            }
            Statement::Expression { expr, .. } => {
                self.analyze_expression_for_struct_mappings(expr, optimizations);
            }
            Statement::If {
                then_block,
                else_block,
                ..
            } => {
                for s in then_block {
                    self.analyze_statement_for_struct_mappings(s, optimizations);
                }
                if let Some(else_b) = else_block {
                    for s in else_b {
                        self.analyze_statement_for_struct_mappings(s, optimizations);
                    }
                }
            }
            _ => {}
        }
    }

    /// Analyze an expression for struct mapping opportunities
    pub(crate) fn analyze_expression_for_struct_mappings(
        &self,
        expr: &Expression,
        optimizations: &mut Vec<StructMappingOptimization>,
    ) {
        match expr {
            Expression::StructLiteral { name, fields, .. } => {
                let mut field_mappings = Vec::new();
                let mut source_candidates = HashMap::new();

                for (field_name, field_expr) in fields {
                    let field_source = self.extract_field_source(field_expr);
                    field_mappings
                        .push((field_name.clone(), self.expression_to_string(field_expr)));

                    if let Some(src) = &field_source {
                        *source_candidates.entry(src.clone()).or_insert(0) += 1;
                    }
                }

                let strategy = if let Some((dominant_source, count)) =
                    source_candidates.iter().max_by_key(|(_, c)| *c)
                {
                    if *count == fields.len() {
                        MappingStrategy::DirectMapping
                    } else if dominant_source == "row" || dominant_source.starts_with("row.") {
                        MappingStrategy::FromRow
                    } else {
                        MappingStrategy::TypeConversion
                    }
                } else {
                    MappingStrategy::TypeConversion
                };

                if !source_candidates.is_empty() {
                    let source = source_candidates
                        .keys()
                        .next()
                        .cloned()
                        .unwrap_or_else(|| "unknown".to_string());

                    optimizations.push(StructMappingOptimization {
                        target_struct: name.clone(),
                        source,
                        field_mappings,
                        strategy,
                    });
                }
            }
            Expression::Call { arguments, .. } | Expression::MethodCall { arguments, .. } => {
                for (_, arg) in arguments {
                    self.analyze_expression_for_struct_mappings(arg, optimizations);
                }
            }
            _ => {}
        }
    }

    /// Extract the source variable/expression from a field expression
    pub(crate) fn extract_field_source(&self, expr: &Expression) -> Option<String> {
        match expr {
            Expression::Identifier { name, .. } => Some(name.clone()),
            Expression::FieldAccess { object, .. } => {
                if let Expression::Identifier { name, .. } = &**object {
                    Some(name.clone())
                } else {
                    None
                }
            }
            Expression::MethodCall { object, .. } => {
                if let Expression::Identifier { name, .. } = &**object {
                    Some(name.clone())
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Convert expression to string for field mapping tracking
    #[allow(clippy::only_used_in_recursion)]
    pub(crate) fn expression_to_string(&self, expr: &Expression) -> String {
        match expr {
            Expression::Identifier { name, .. } => name.clone(),
            Expression::FieldAccess { object, field, .. } => {
                format!("{}.{}", self.expression_to_string(object), field)
            }
            Expression::MethodCall { object, method, .. } => {
                format!("{}.{}()", self.expression_to_string(object), method)
            }
            Expression::Literal { value: lit, .. } => format!("{:?}", lit),
            _ => "expr".to_string(),
        }
    }
}
