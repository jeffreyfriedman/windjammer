// Automatic clone insertion for Windjammer ergonomics
//
// Philosophy: Users should NEVER need to write .clone() manually.
// The compiler should automatically insert clones when:
// 1. A value is moved AND used again later
// 2. A value is passed to a function that takes ownership AND used again
// 3. A value is stored in a collection AND used again
//
// This module tracks variable usage and determines where clones are needed.

use crate::parser::*;
use std::collections::HashMap;

/// Tracks where automatic clones should be inserted
#[derive(Debug, Clone)]
pub struct AutoCloneAnalysis {
    /// Variables that need to be cloned at specific usage sites
    /// Key: (variable_name, statement_index)
    /// Value: reason for clone
    pub clone_sites: HashMap<(String, usize), CloneReason>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CloneReason {
    /// Value is moved here but used again later
    MovedButUsedLater,
    /// Value is passed to function that takes ownership
    PassedToOwningFunction,
    /// Value is stored in collection
    StoredInCollection,
    /// Value is returned but also used in function
    ReturnedButUsedAgain,
}

impl Default for AutoCloneAnalysis {
    fn default() -> Self {
        Self::new()
    }
}

impl AutoCloneAnalysis {
    pub fn new() -> Self {
        AutoCloneAnalysis {
            clone_sites: HashMap::new(),
        }
    }

    /// Analyze a function to determine where clones should be inserted
    pub fn analyze_function(func: &FunctionDecl) -> Self {
        let mut analysis = AutoCloneAnalysis::new();

        // Track all variable usages
        let usage_map = Self::build_usage_map(&func.body);

        // For each variable, determine if it needs clones
        for (var_name, usages) in &usage_map {
            analysis.analyze_variable_usages(var_name, usages);
        }

        analysis
    }

    /// Build a map of all variable usages in the function
    fn build_usage_map(statements: &[Statement]) -> HashMap<String, Vec<Usage>> {
        let mut map = HashMap::new();

        for (idx, stmt) in statements.iter().enumerate() {
            Self::collect_usages_from_statement(stmt, idx, &mut map);
        }

        map
    }

    /// Collect all usages of variables from a statement
    fn collect_usages_from_statement(
        stmt: &Statement,
        idx: usize,
        map: &mut HashMap<String, Vec<Usage>>,
    ) {
        match stmt {
            Statement::Let { pattern, value, .. } => {
                // Collect usages from the value expression
                Self::collect_usages_from_expression(value, idx, UsageKind::Read, map);

                // Mark the variable as defined
                if let Pattern::Identifier(name) = pattern {
                    map.entry(name.clone()).or_default().push(Usage {
                        statement_idx: idx,
                        kind: UsageKind::Definition,
                        is_move: false,
                    });
                }
            }
            Statement::Assignment { target, value, .. } => {
                Self::collect_usages_from_expression(target, idx, UsageKind::Write, map);
                Self::collect_usages_from_expression(value, idx, UsageKind::Read, map);
            }
            Statement::Return {
                value: Some(expr), ..
            } => {
                Self::collect_usages_from_expression(expr, idx, UsageKind::Move, map);
            }
            Statement::Expression { expr, .. } => {
                Self::collect_usages_from_expression(expr, idx, UsageKind::Read, map);
            }
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                Self::collect_usages_from_expression(condition, idx, UsageKind::Read, map);
                for stmt in then_block {
                    Self::collect_usages_from_statement(stmt, idx, map);
                }
                if let Some(else_b) = else_block {
                    for stmt in else_b {
                        Self::collect_usages_from_statement(stmt, idx, map);
                    }
                }
            }
            Statement::While {
                condition, body, ..
            } => {
                Self::collect_usages_from_expression(condition, idx, UsageKind::Read, map);
                for stmt in body {
                    Self::collect_usages_from_statement(stmt, idx, map);
                }
            }
            Statement::For {
                pattern: _,
                iterable,
                body,
                ..
            } => {
                Self::collect_usages_from_expression(iterable, idx, UsageKind::Read, map);
                for stmt in body {
                    Self::collect_usages_from_statement(stmt, idx, map);
                }
            }
            Statement::Loop { body, .. } => {
                for stmt in body {
                    Self::collect_usages_from_statement(stmt, idx, map);
                }
            }
            Statement::Match { value, arms, .. } => {
                Self::collect_usages_from_expression(value, idx, UsageKind::Read, map);
                for arm in arms {
                    // MatchArm.body is an Expression, not Vec<Statement>
                    Self::collect_usages_from_expression(&arm.body, idx, UsageKind::Read, map);
                }
            }
            _ => {}
        }
    }

    /// Extract a path string from an expression (e.g., "config.paths", "obj.method()", "items[0]")
    fn extract_expression_path(expr: &Expression) -> Option<String> {
        match expr {
            Expression::Identifier { name, .. } => Some(name.clone()),
            Expression::FieldAccess { object, field, .. } => {
                // Recursively build the path: object.field
                Self::extract_expression_path(object)
                    .map(|base_path| format!("{}.{}", base_path, field))
            }
            Expression::MethodCall { object, method, .. } => {
                // Build path for method calls: object.method()
                Self::extract_expression_path(object)
                    .map(|base_path| format!("{}.{}()", base_path, method))
            }
            Expression::Index { object, index, .. } => {
                // Build path for index expressions: object[index]
                // For simplicity, we use [*] as a placeholder since the actual index
                // might vary (e.g., items[0], items[i])
                if let Some(base_path) = Self::extract_expression_path(object) {
                    // Try to get a more specific index if it's a literal
                    let index_str = match index.as_ref() {
                        Expression::Literal {
                            value: crate::parser::Literal::Int(n),
                            ..
                        } => n.to_string(),
                        Expression::Identifier { name, .. } => name.clone(),
                        _ => "*".to_string(), // Generic placeholder
                    };
                    Some(format!("{}[{}]", base_path, index_str))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Collect usages from an expression
    fn collect_usages_from_expression(
        expr: &Expression,
        idx: usize,
        kind: UsageKind,
        map: &mut HashMap<String, Vec<Usage>>,
    ) {
        match expr {
            Expression::Identifier { name, .. } => {
                map.entry(name.clone()).or_default().push(Usage {
                    statement_idx: idx,
                    kind,
                    is_move: kind == UsageKind::Move,
                });
            }
            Expression::FieldAccess { object, .. } => {
                // Track the full field access path (e.g., "config.paths")
                if let Some(path) = Self::extract_expression_path(expr) {
                    map.entry(path).or_default().push(Usage {
                        statement_idx: idx,
                        kind,
                        is_move: kind == UsageKind::Move,
                    });
                }
                // Also track the base object as a read
                Self::collect_usages_from_expression(object, idx, UsageKind::Read, map);
            }
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                // Function calls may move arguments
                Self::collect_usages_from_expression(function, idx, UsageKind::Read, map);
                for (_label, arg_expr) in arguments {
                    // Assume arguments are moved (conservative)
                    // TODO: Check function signature to determine actual ownership
                    Self::collect_usages_from_expression(arg_expr, idx, UsageKind::Move, map);
                }
            }
            Expression::MethodCall {
                object, arguments, ..
            } => {
                // Track the full method call path (e.g., "source.get_items()")
                if let Some(path) = Self::extract_expression_path(expr) {
                    map.entry(path).or_default().push(Usage {
                        statement_idx: idx,
                        kind,
                        is_move: kind == UsageKind::Move,
                    });
                }
                // Also track the base object as a read
                Self::collect_usages_from_expression(object, idx, UsageKind::Read, map);
                for (_label, arg_expr) in arguments {
                    Self::collect_usages_from_expression(arg_expr, idx, UsageKind::Move, map);
                }
            }
            Expression::Binary { left, right, .. } => {
                Self::collect_usages_from_expression(left, idx, UsageKind::Read, map);
                Self::collect_usages_from_expression(right, idx, UsageKind::Read, map);
            }
            Expression::Unary { operand, .. } => {
                Self::collect_usages_from_expression(operand, idx, UsageKind::Read, map);
            }
            Expression::Index { object, index, .. } => {
                // Track the full index expression path (e.g., "items[0]", "arr[i]")
                if let Some(path) = Self::extract_expression_path(expr) {
                    map.entry(path).or_default().push(Usage {
                        statement_idx: idx,
                        kind,
                        is_move: kind == UsageKind::Move,
                    });
                }
                // Also track the base object and index as reads
                Self::collect_usages_from_expression(object, idx, UsageKind::Read, map);
                Self::collect_usages_from_expression(index, idx, UsageKind::Read, map);
            }
            Expression::Tuple { elements, .. } => {
                for elem in elements {
                    Self::collect_usages_from_expression(elem, idx, UsageKind::Read, map);
                }
            }
            Expression::Array { elements, .. } => {
                for elem in elements {
                    // Array elements are moved
                    Self::collect_usages_from_expression(elem, idx, UsageKind::Move, map);
                }
            }
            Expression::StructLiteral { fields, .. } => {
                for (_, field_expr) in fields {
                    // Struct fields are moved
                    Self::collect_usages_from_expression(field_expr, idx, UsageKind::Move, map);
                }
            }
            Expression::Block { statements, .. } => {
                for stmt in statements {
                    Self::collect_usages_from_statement(stmt, idx, map);
                }
            }
            Expression::Cast { expr, .. } => {
                Self::collect_usages_from_expression(expr, idx, UsageKind::Read, map);
            }
            Expression::Range { start, end, .. } => {
                Self::collect_usages_from_expression(start, idx, UsageKind::Read, map);
                Self::collect_usages_from_expression(end, idx, UsageKind::Read, map);
            }
            Expression::TryOp { expr, .. } => {
                Self::collect_usages_from_expression(expr, idx, UsageKind::Read, map);
            }
            Expression::Await { expr, .. } => {
                Self::collect_usages_from_expression(expr, idx, UsageKind::Read, map);
            }
            Expression::ChannelSend { channel, value, .. } => {
                Self::collect_usages_from_expression(channel, idx, UsageKind::Read, map);
                Self::collect_usages_from_expression(value, idx, UsageKind::Move, map);
            }
            Expression::ChannelRecv { channel, .. } => {
                Self::collect_usages_from_expression(channel, idx, UsageKind::Read, map);
            }
            Expression::MacroInvocation { args, .. } => {
                for arg in args {
                    Self::collect_usages_from_expression(arg, idx, UsageKind::Read, map);
                }
            }
            Expression::MapLiteral { pairs, .. } => {
                for (key, value) in pairs {
                    Self::collect_usages_from_expression(key, idx, UsageKind::Move, map);
                    Self::collect_usages_from_expression(value, idx, UsageKind::Move, map);
                }
            }
            _ => {}
        }
    }

    /// Analyze usages of a single variable to determine where clones are needed
    fn analyze_variable_usages(&mut self, var_name: &str, usages: &[Usage]) {
        // Find the definition
        let definition_idx = usages
            .iter()
            .find(|u| u.kind == UsageKind::Definition)
            .map(|u| u.statement_idx);

        // Field accesses (e.g., "config.paths"), method calls (e.g., "source.get_items()"),
        // and index expressions (e.g., "items[0]") don't have definitions.
        // They're valid if they contain a dot, parentheses, or square brackets.
        let is_complex_expr =
            var_name.contains('.') || var_name.contains('(') || var_name.contains('[');

        if definition_idx.is_none() && !is_complex_expr {
            // Variable not defined in this scope (parameter, etc.) and not a complex expression
            return;
        }

        // Find all moves
        let moves: Vec<&Usage> = usages
            .iter()
            .filter(|u| u.is_move && u.kind != UsageKind::Definition)
            .collect();

        if moves.is_empty() {
            // No moves, no clones needed
            return;
        }

        // For each move, check if there are later usages
        for move_usage in &moves {
            let later_usages: Vec<&Usage> = usages
                .iter()
                .filter(|u| {
                    u.statement_idx > move_usage.statement_idx && u.kind != UsageKind::Definition
                })
                .collect();

            if !later_usages.is_empty() {
                // This move needs a clone because the variable is used later
                self.clone_sites.insert(
                    (var_name.to_string(), move_usage.statement_idx),
                    CloneReason::MovedButUsedLater,
                );
            }
        }
    }

    /// Check if a variable needs to be cloned at a specific statement
    pub fn needs_clone(&self, var_name: &str, statement_idx: usize) -> Option<&CloneReason> {
        self.clone_sites.get(&(var_name.to_string(), statement_idx))
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Usage {
    statement_idx: usize,
    kind: UsageKind,
    is_move: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum UsageKind {
    Definition,
    Read,
    Write,
    Move,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_move_and_reuse() {
        // let x = vec![1, 2, 3]
        // takes_ownership(x)  // <- Should insert .clone() here
        // println!("{}", x.len())

        let func = FunctionDecl {
            name: "test".to_string(),
            parameters: vec![],
            return_type: None,
            type_params: vec![],
            where_clause: vec![],
            decorators: vec![],
            is_async: false,
            parent_type: None,
            body: vec![
                Statement::Let {
                    pattern: Pattern::Identifier("x".to_string()),
                    mutable: false,
                    type_: None,
                    value: Expression::Array {
                        elements: vec![
                            Expression::Literal {
                                value: Literal::Int(1),
                                location: None,
                            },
                            Expression::Literal {
                                value: Literal::Int(2),
                                location: None,
                            },
                            Expression::Literal {
                                value: Literal::Int(3),
                                location: None,
                            },
                        ],
                        location: None,
                    },
                    location: None,
                },
                Statement::Expression {
                    expr: Expression::Call {
                        function: Box::new(Expression::Identifier {
                            name: "takes_ownership".to_string(),
                            location: None,
                        }),
                        arguments: vec![(
                            None,
                            Expression::Identifier {
                                name: "x".to_string(),
                                location: None,
                            },
                        )],
                        location: None,
                    },
                    location: None,
                },
                Statement::Expression {
                    expr: Expression::MethodCall {
                        object: Box::new(Expression::Identifier {
                            name: "x".to_string(),
                            location: None,
                        }),
                        method: "len".to_string(),
                        arguments: vec![],
                        type_args: None,
                        location: None,
                    },
                    location: None,
                },
            ],
        };

        let analysis = AutoCloneAnalysis::analyze_function(&func);

        // Should detect that x needs to be cloned at statement 1 (the function call)
        assert!(analysis.needs_clone("x", 1).is_some());
        assert_eq!(
            analysis.needs_clone("x", 1),
            Some(&CloneReason::MovedButUsedLater)
        );
    }

    #[test]
    fn test_no_clone_needed_single_use() {
        // let x = vec![1, 2, 3]
        // takes_ownership(x)  // <- No clone needed, x not used again

        let func = FunctionDecl {
            name: "test".to_string(),
            parameters: vec![],
            return_type: None,
            type_params: vec![],
            where_clause: vec![],
            decorators: vec![],
            is_async: false,
            parent_type: None,
            body: vec![
                Statement::Let {
                    pattern: Pattern::Identifier("x".to_string()),
                    mutable: false,
                    type_: None,
                    value: Expression::Array {
                        elements: vec![],
                        location: None,
                    },
                    location: None,
                },
                Statement::Expression {
                    expr: Expression::Call {
                        function: Box::new(Expression::Identifier {
                            name: "takes_ownership".to_string(),
                            location: None,
                        }),
                        arguments: vec![(
                            None,
                            Expression::Identifier {
                                name: "x".to_string(),
                                location: None,
                            },
                        )],
                        location: None,
                    },
                    location: None,
                },
            ],
        };

        let analysis = AutoCloneAnalysis::analyze_function(&func);

        // Should NOT detect any clones needed
        assert!(analysis.needs_clone("x", 1).is_none());
    }
}
