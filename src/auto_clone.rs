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
    /// Variables that are bound to string literals (don't need .clone())
    /// These are Copy types (references) so .clone() is a no-op
    pub string_literal_vars: std::collections::HashSet<String>,
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
            string_literal_vars: std::collections::HashSet::new(),
        }
    }

    /// Analyze a function to determine where clones should be inserted
    pub fn analyze_function(func: &FunctionDecl) -> Self {
        let mut analysis = AutoCloneAnalysis::new();

        // Track variables bound to string literals (don't need .clone())
        analysis.find_string_literal_vars(&func.body);

        // Track all variable usages
        let mut usage_map = Self::build_usage_map(&func.body);

        // Register function parameters as definitions at statement_idx 0.
        // Without this, parameters are skipped by analyze_variable_usages
        // because they have no Definition usage, causing auto-clone to miss
        // parameters used multiple times (E0382).
        for param in &func.parameters {
            if param.name == "self" {
                continue;
            }
            let usages = usage_map.entry(param.name.clone()).or_default();
            let has_def = usages.iter().any(|u| u.kind == UsageKind::Definition);
            if !has_def {
                usages.insert(
                    0,
                    Usage {
                        kind: UsageKind::Definition,
                        statement_idx: 0,
                        is_move: false,
                        in_loop: false,
                    },
                );
            }
        }

        // For each variable, determine if it needs clones
        for (var_name, usages) in &usage_map {
            analysis.analyze_variable_usages(var_name, usages);
        }

        // Partial-move detection: if a field path like "s.item" is moved,
        // and the root variable "s" has later uses, the field access must
        // be cloned to avoid a partial move error (E0382).
        analysis.detect_partial_moves(&usage_map);

        analysis
    }

    /// Build a map of all variable usages in the function
    fn build_usage_map<'ast>(statements: &[&'ast Statement<'ast>]) -> HashMap<String, Vec<Usage>> {
        let mut map = HashMap::new();

        for (idx, stmt) in statements.iter().enumerate() {
            Self::collect_usages_from_statement(stmt, idx, false, &mut map);
        }

        map
    }

    /// Collect all usages of variables from a statement
    fn collect_usages_from_statement(
        stmt: &Statement,
        idx: usize,
        in_loop: bool,
        map: &mut HashMap<String, Vec<Usage>>,
    ) {
        match stmt {
            Statement::Let { pattern, value, .. } => {
                Self::collect_usages_from_expression(value, idx, UsageKind::Read, in_loop, map);

                if let Pattern::Identifier(name) = pattern {
                    map.entry(name.clone()).or_default().push(Usage {
                        statement_idx: idx,
                        kind: UsageKind::Definition,
                        is_move: false,
                        in_loop,
                    });
                }
            }
            Statement::Assignment { target, value, .. } => {
                Self::collect_usages_from_expression(target, idx, UsageKind::Write, in_loop, map);
                Self::collect_usages_from_expression(value, idx, UsageKind::Read, in_loop, map);
            }
            Statement::Return {
                value: Some(expr), ..
            } => {
                Self::collect_usages_from_expression(expr, idx, UsageKind::Move, in_loop, map);
            }
            Statement::Expression { expr, .. } => {
                Self::collect_usages_from_expression(expr, idx, UsageKind::Read, in_loop, map);
            }
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                Self::collect_usages_from_expression(condition, idx, UsageKind::Read, in_loop, map);
                for (body_i, stmt) in then_block.iter().enumerate() {
                    Self::collect_usages_from_statement(stmt, body_i, in_loop, map);
                }
                if let Some(else_b) = else_block {
                    for (body_i, stmt) in else_b.iter().enumerate() {
                        Self::collect_usages_from_statement(stmt, body_i, in_loop, map);
                    }
                }
            }
            Statement::While {
                condition, body, ..
            } => {
                Self::collect_usages_from_expression(condition, idx, UsageKind::Read, in_loop, map);
                for (body_i, stmt) in body.iter().enumerate() {
                    Self::collect_usages_from_statement(stmt, body_i, true, map);
                }
            }
            Statement::For {
                pattern: _,
                iterable,
                body,
                ..
            } => {
                Self::collect_usages_from_expression(iterable, idx, UsageKind::Read, in_loop, map);
                for (body_i, stmt) in body.iter().enumerate() {
                    Self::collect_usages_from_statement(stmt, body_i, true, map);
                }
            }
            Statement::Loop { body, .. } => {
                for (body_i, stmt) in body.iter().enumerate() {
                    Self::collect_usages_from_statement(stmt, body_i, true, map);
                }
            }
            Statement::Match { value, arms, .. } => {
                Self::collect_usages_from_expression(value, idx, UsageKind::Read, in_loop, map);
                for arm in arms {
                    Self::collect_usages_from_expression(
                        arm.body,
                        idx,
                        UsageKind::Read,
                        in_loop,
                        map,
                    );
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
                    let index_str = match index {
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
        in_loop: bool,
        map: &mut HashMap<String, Vec<Usage>>,
    ) {
        match expr {
            Expression::Identifier { name, .. } => {
                map.entry(name.clone()).or_default().push(Usage {
                    statement_idx: idx,
                    kind,
                    is_move: kind == UsageKind::Move,
                    in_loop,
                });
            }
            Expression::FieldAccess { object, .. } => {
                if let Some(path) = Self::extract_expression_path(expr) {
                    map.entry(path).or_default().push(Usage {
                        statement_idx: idx,
                        kind,
                        is_move: kind == UsageKind::Move,
                        in_loop,
                    });
                }
                Self::collect_usages_from_expression(object, idx, UsageKind::Read, in_loop, map);
            }
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                Self::collect_usages_from_expression(function, idx, UsageKind::Read, in_loop, map);
                for (_label, arg_expr) in arguments {
                    Self::collect_usages_from_expression(
                        arg_expr,
                        idx,
                        UsageKind::Move,
                        in_loop,
                        map,
                    );
                }
            }
            Expression::MethodCall {
                object, arguments, ..
            } => {
                if let Some(path) = Self::extract_expression_path(expr) {
                    map.entry(path).or_default().push(Usage {
                        statement_idx: idx,
                        kind,
                        is_move: kind == UsageKind::Move,
                        in_loop,
                    });
                }
                Self::collect_usages_from_expression(object, idx, UsageKind::Read, in_loop, map);
                for (_label, arg_expr) in arguments {
                    Self::collect_usages_from_expression(
                        arg_expr,
                        idx,
                        UsageKind::Move,
                        in_loop,
                        map,
                    );
                }
            }
            Expression::Binary { left, right, .. } => {
                Self::collect_usages_from_expression(left, idx, UsageKind::Read, in_loop, map);
                Self::collect_usages_from_expression(right, idx, UsageKind::Read, in_loop, map);
            }
            Expression::Unary { operand, .. } => {
                Self::collect_usages_from_expression(operand, idx, UsageKind::Read, in_loop, map);
            }
            Expression::Index { object, index, .. } => {
                if let Some(path) = Self::extract_expression_path(expr) {
                    map.entry(path).or_default().push(Usage {
                        statement_idx: idx,
                        kind,
                        is_move: kind == UsageKind::Move,
                        in_loop,
                    });
                }
                Self::collect_usages_from_expression(object, idx, UsageKind::Read, in_loop, map);
                Self::collect_usages_from_expression(index, idx, UsageKind::Read, in_loop, map);
            }
            Expression::Tuple { elements, .. } => {
                for elem in elements {
                    Self::collect_usages_from_expression(elem, idx, UsageKind::Read, in_loop, map);
                }
            }
            Expression::Array { elements, .. } => {
                for elem in elements {
                    Self::collect_usages_from_expression(elem, idx, UsageKind::Move, in_loop, map);
                }
            }
            Expression::StructLiteral { fields, .. } => {
                for (_, field_expr) in fields {
                    Self::collect_usages_from_expression(
                        field_expr,
                        idx,
                        UsageKind::Move,
                        in_loop,
                        map,
                    );
                }
            }
            Expression::Block { statements, .. } => {
                for stmt in statements {
                    Self::collect_usages_from_statement(stmt, idx, in_loop, map);
                }
            }
            Expression::Cast { expr, .. } => {
                Self::collect_usages_from_expression(expr, idx, UsageKind::Read, in_loop, map);
            }
            Expression::Range { start, end, .. } => {
                Self::collect_usages_from_expression(start, idx, UsageKind::Read, in_loop, map);
                Self::collect_usages_from_expression(end, idx, UsageKind::Read, in_loop, map);
            }
            Expression::TryOp { expr, .. } => {
                Self::collect_usages_from_expression(expr, idx, UsageKind::Read, in_loop, map);
            }
            Expression::Await { expr, .. } => {
                Self::collect_usages_from_expression(expr, idx, UsageKind::Read, in_loop, map);
            }
            Expression::ChannelSend { channel, value, .. } => {
                Self::collect_usages_from_expression(channel, idx, UsageKind::Read, in_loop, map);
                Self::collect_usages_from_expression(value, idx, UsageKind::Move, in_loop, map);
            }
            Expression::ChannelRecv { channel, .. } => {
                Self::collect_usages_from_expression(channel, idx, UsageKind::Read, in_loop, map);
            }
            Expression::MacroInvocation { args, .. } => {
                for arg in args {
                    Self::collect_usages_from_expression(arg, idx, UsageKind::Read, in_loop, map);
                }
            }
            Expression::MapLiteral { pairs, .. } => {
                for (key, value) in pairs {
                    Self::collect_usages_from_expression(key, idx, UsageKind::Move, in_loop, map);
                    Self::collect_usages_from_expression(value, idx, UsageKind::Move, in_loop, map);
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

        // For each move, check if it needs cloning:
        // 1. There are later usages after this move
        // 2. Multiple moves in the same statement
        // 3. The move is inside a loop (loop may execute again, consuming the value twice)
        let total_uses: Vec<&Usage> = usages
            .iter()
            .filter(|u| u.kind != UsageKind::Definition)
            .collect();

        for move_usage in &moves {
            let has_later_use = total_uses
                .iter()
                .any(|u| u.statement_idx > move_usage.statement_idx);

            let same_stmt_moves = moves
                .iter()
                .filter(|m| m.statement_idx == move_usage.statement_idx)
                .count();

            // Moves inside loops always need clone -- the loop body executes
            // multiple times, consuming the value on each iteration.
            let needs_clone = has_later_use || same_stmt_moves > 1 || move_usage.in_loop;

            if needs_clone {
                self.clone_sites.insert(
                    (var_name.to_string(), move_usage.statement_idx),
                    CloneReason::MovedButUsedLater,
                );
            }
        }
    }

    /// Detect partial moves: field accesses like `s.item` where `s` is used later.
    /// When `s.item` is moved (e.g., passed to a function taking ownership) and `s`
    /// itself is used afterwards, `s.item` must be cloned to avoid E0382.
    fn detect_partial_moves(&mut self, usage_map: &HashMap<String, Vec<Usage>>) {
        let field_paths: Vec<String> = usage_map
            .keys()
            .filter(|k| k.contains('.') && !k.contains('('))
            .cloned()
            .collect();

        for path in &field_paths {
            let Some(dot_pos) = path.find('.') else {
                continue;
            };
            let root = &path[..dot_pos];

            let Some(root_usages) = usage_map.get(root) else {
                continue;
            };

            let Some(field_usages) = usage_map.get(path.as_str()) else {
                continue;
            };

            let field_moves: Vec<&Usage> = field_usages
                .iter()
                .filter(|u| u.is_move && u.kind != UsageKind::Definition)
                .collect();

            for field_move in &field_moves {
                let root_used_later = root_usages.iter().any(|u| {
                    u.kind != UsageKind::Definition
                        && u.statement_idx >= field_move.statement_idx
                        && u.is_move
                });

                if root_used_later {
                    self.clone_sites.insert(
                        (path.clone(), field_move.statement_idx),
                        CloneReason::MovedButUsedLater,
                    );
                }
            }
        }
    }

    /// Check if a variable needs to be cloned at a specific statement
    pub fn needs_clone(&self, var_name: &str, statement_idx: usize) -> Option<&CloneReason> {
        // Don't clone string literal variables (they're just &str references)
        if self.string_literal_vars.contains(var_name) {
            return None;
        }
        self.clone_sites.get(&(var_name.to_string(), statement_idx))
    }

    /// Find variables that are bound to string literals
    /// These don't need .clone() because they're just &str references
    fn find_string_literal_vars<'ast>(&mut self, statements: &[&'ast Statement<'ast>]) {
        for stmt in statements {
            match stmt {
                Statement::Let {
                    pattern: Pattern::Identifier(var_name),
                    value,
                    ..
                }
                    // Check if value is a string literal or a match/if that returns string literals
                    if Self::expr_returns_string_literal(value) => {
                        self.string_literal_vars.insert(var_name.clone());
                    }
                Statement::Let { .. } => {
                    // Non-identifier patterns (tuple, wildcard, etc.)
                }
                Statement::If {
                    then_block,
                    else_block,
                    ..
                } => {
                    self.find_string_literal_vars(then_block);
                    if let Some(else_b) = else_block {
                        self.find_string_literal_vars(else_b);
                    }
                }
                Statement::While { body, .. }
                | Statement::For { body, .. }
                | Statement::Loop { body, .. } => {
                    self.find_string_literal_vars(body);
                }
                Statement::Match { .. } => {
                    // Match arms are expressions, handled in expr_returns_string_literal
                }
                _ => {}
            }
        }
    }

    /// Check if an expression returns a string literal
    /// This includes direct literals, match expressions with all string literal arms, etc.
    fn expr_returns_string_literal(expr: &Expression) -> bool {
        match expr {
            Expression::Literal {
                value: crate::parser::Literal::String(_),
                ..
            } => true,
            Expression::Block { statements, .. } => {
                // Check if the block ends with a match statement that returns string literals
                if let Some(Statement::Match { arms, .. }) = statements.last() {
                    arms.iter()
                        .all(|arm| Self::expr_returns_string_literal(arm.body))
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Usage {
    statement_idx: usize,
    kind: UsageKind,
    is_move: bool,
    in_loop: bool,
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
    use crate::test_utils::{test_alloc_expr, test_alloc_stmt};

    #[test]
    fn test_simple_move_and_reuse() {
        // let x = vec![1, 2, 3]
        // takes_ownership(x)  // <- Should insert .clone() here
        // println!("{}", x.len())

        let func = FunctionDecl {
            name: "test".to_string(),
            is_pub: false,
            is_extern: false,
            parameters: vec![],
            return_type: None,
            return_decorators: Vec::new(),
            type_params: vec![],
            where_clause: vec![],
            decorators: vec![],
            is_async: false,
            parent_type: None,
            impl_trait: None,
            doc_comment: None,
            body: vec![
                test_alloc_stmt(Statement::Let {
                    pattern: Pattern::Identifier("x".to_string()),
                    mutable: false,
                    type_: None,
                    value: test_alloc_expr(Expression::Array {
                        elements: vec![
                            test_alloc_expr(Expression::Literal {
                                value: Literal::Int(1),
                                location: None,
                            }),
                            test_alloc_expr(Expression::Literal {
                                value: Literal::Int(2),
                                location: None,
                            }),
                            test_alloc_expr(Expression::Literal {
                                value: Literal::Int(3),
                                location: None,
                            }),
                        ],
                        location: None,
                    }),
                    else_block: None,
                    location: None,
                }),
                test_alloc_stmt(Statement::Expression {
                    expr: test_alloc_expr(Expression::Call {
                        function: test_alloc_expr(Expression::Identifier {
                            name: "takes_ownership".to_string(),
                            location: None,
                        }),
                        arguments: vec![(
                            None,
                            test_alloc_expr(Expression::Identifier {
                                name: "x".to_string(),
                                location: None,
                            }),
                        )],
                        location: None,
                    }),
                    location: None,
                }),
                test_alloc_stmt(Statement::Expression {
                    expr: test_alloc_expr(Expression::MethodCall {
                        object: test_alloc_expr(Expression::Identifier {
                            name: "x".to_string(),
                            location: None,
                        }),
                        method: "len".to_string(),
                        arguments: vec![],
                        type_args: None,
                        location: None,
                    }),
                    location: None,
                }),
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
            is_pub: false,
            is_extern: false,
            parameters: vec![],
            return_type: None,
            return_decorators: Vec::new(),
            type_params: vec![],
            where_clause: vec![],
            decorators: vec![],
            is_async: false,
            parent_type: None,
            impl_trait: None,
            doc_comment: None,
            body: vec![
                test_alloc_stmt(Statement::Let {
                    pattern: Pattern::Identifier("x".to_string()),
                    mutable: false,
                    type_: None,
                    value: test_alloc_expr(Expression::Array {
                        elements: vec![],
                        location: None,
                    }),
                    else_block: None,
                    location: None,
                }),
                test_alloc_stmt(Statement::Expression {
                    expr: test_alloc_expr(Expression::Call {
                        function: test_alloc_expr(Expression::Identifier {
                            name: "takes_ownership".to_string(),
                            location: None,
                        }),
                        arguments: vec![(
                            None,
                            test_alloc_expr(Expression::Identifier {
                                name: "x".to_string(),
                                location: None,
                            }),
                        )],
                        location: None,
                    }),
                    location: None,
                }),
            ],
        };

        let analysis = AutoCloneAnalysis::analyze_function(&func);

        // Should NOT detect any clones needed
        assert!(analysis.needs_clone("x", 1).is_none());
    }
}
