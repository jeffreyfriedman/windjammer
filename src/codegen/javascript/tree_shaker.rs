//! Tree shaking (dead code elimination) for JavaScript
//!
//! Removes unused functions, variables, and imports from JavaScript output.

use crate::parser::{Expression, Item, Program, Statement};
use std::collections::HashSet;

/// Tree shaker for JavaScript code
pub struct TreeShaker {
    /// Set of used function names
    used_functions: HashSet<String>,
    /// Set of used variable names
    #[allow(dead_code)]
    used_variables: HashSet<String>,
    /// Entry points (always preserved)
    entry_points: HashSet<String>,
}

impl TreeShaker {
    /// Create a new tree shaker
    pub fn new() -> Self {
        let mut entry_points = HashSet::new();
        entry_points.insert("main".to_string());

        Self {
            used_functions: HashSet::new(),
            used_variables: HashSet::new(),
            entry_points,
        }
    }

    /// Add an entry point (function that should always be preserved)
    pub fn add_entry_point(&mut self, name: String) {
        self.entry_points.insert(name);
    }

    /// Shake the tree - remove unused code from the program
    pub fn shake(&mut self, program: &Program) -> Program {
        // Phase 1: Mark all used items starting from entry points
        self.mark_used(program);

        // Phase 2: Sweep - remove unused items
        let items = program
            .items
            .iter()
            .filter(|item| self.should_keep_item(item))
            .cloned()
            .collect();

        Program { items }
    }

    /// Mark all used items starting from entry points
    fn mark_used(&mut self, program: &Program) {
        // Start with entry points
        for entry in &self.entry_points.clone() {
            self.used_functions.insert(entry.clone());
        }

        // Iteratively find all reachable code
        let mut changed = true;
        while changed {
            changed = false;
            let current_used = self.used_functions.clone();

            for item in &program.items {
                if let Item::Function { decl: func, .. } = item {
                    if current_used.contains(&func.name) {
                        // Mark all functions called from this function
                        for call in self.find_function_calls(&func.body) {
                            if self.used_functions.insert(call) {
                                changed = true;
                            }
                        }
                    }
                }
            }
        }
    }

    /// Find all function calls in a block of statements
    fn find_function_calls(&self, statements: &[Statement]) -> Vec<String> {
        let mut calls = Vec::new();

        for stmt in statements {
            calls.extend(self.find_calls_in_statement(stmt));
        }

        calls
    }

    /// Find function calls in a statement
    fn find_calls_in_statement(&self, stmt: &Statement) -> Vec<String> {
        let mut calls = Vec::new();

        match stmt {
            Statement::Expression { expr, .. } => {
                calls.extend(self.find_calls_in_expression(expr));
            }
            Statement::Let { value, .. } => {
                calls.extend(self.find_calls_in_expression(value));
            }
            Statement::Return {
                value: Some(expr), ..
            } => {
                calls.extend(self.find_calls_in_expression(expr));
            }
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                calls.extend(self.find_calls_in_expression(condition));
                calls.extend(self.find_function_calls(then_block));
                if let Some(else_b) = else_block {
                    calls.extend(self.find_function_calls(else_b));
                }
            }
            Statement::For { iterable, body, .. } => {
                calls.extend(self.find_calls_in_expression(iterable));
                calls.extend(self.find_function_calls(body));
            }
            Statement::While {
                condition, body, ..
            } => {
                calls.extend(self.find_calls_in_expression(condition));
                calls.extend(self.find_function_calls(body));
            }
            Statement::Loop { body, .. } => {
                calls.extend(self.find_function_calls(body));
            }
            _ => {}
        }

        calls
    }

    /// Find function calls in an expression
    fn find_calls_in_expression(&self, expr: &Expression) -> Vec<String> {
        let mut calls = Vec::new();

        match expr {
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                // Check if it's a direct function call
                if let Expression::Identifier { name, .. } = function.as_ref() {
                    calls.push(name.clone());
                }
                calls.extend(self.find_calls_in_expression(function));
                for (_, arg) in arguments {
                    calls.extend(self.find_calls_in_expression(arg));
                }
            }
            Expression::Binary { left, right, .. } => {
                calls.extend(self.find_calls_in_expression(left));
                calls.extend(self.find_calls_in_expression(right));
            }
            Expression::Unary { operand, .. } => {
                calls.extend(self.find_calls_in_expression(operand));
            }
            Expression::MethodCall {
                object, arguments, ..
            } => {
                calls.extend(self.find_calls_in_expression(object));
                for (_, arg) in arguments {
                    calls.extend(self.find_calls_in_expression(arg));
                }
            }
            Expression::FieldAccess { object, .. } => {
                calls.extend(self.find_calls_in_expression(object));
            }
            Expression::Index { object, index, .. } => {
                calls.extend(self.find_calls_in_expression(object));
                calls.extend(self.find_calls_in_expression(index));
            }
            Expression::Block {
                statements: stmts, ..
            } => {
                calls.extend(self.find_function_calls(stmts));
            }
            _ => {}
        }

        calls
    }

    /// Check if an item should be kept
    fn should_keep_item(&self, item: &Item) -> bool {
        match item {
            Item::Function { decl: func, .. } => {
                // Keep if it's used or exported
                self.used_functions.contains(&func.name)
                    || func.decorators.iter().any(|d| d.name == "export")
            }
            Item::Struct { .. } => true, // Keep all structs for now (may be used in types)
            Item::Enum { .. } => true,   // Keep all enums for now
            Item::Const { .. } => true,  // Keep all constants
            Item::Static { .. } => true, // Keep all statics
            Item::Trait { .. } => true,  // Keep all traits
            Item::Impl { .. } => true,   // Keep all impls
            Item::Use { .. } => true,    // Keep all imports (could be smarter here)
            Item::Mod { .. } => true,    // Keep all modules
            Item::BoundAlias { .. } => true, // Keep all bound aliases
        }
    }
}

impl Default for TreeShaker {
    fn default() -> Self {
        Self::new()
    }
}

/// Shake the tree - remove unused code
pub fn shake_tree(program: &Program) -> Program {
    TreeShaker::new().shake(program)
}

/// Analyze code usage and return statistics
pub struct UsageAnalysis {
    pub total_functions: usize,
    pub used_functions: usize,
    pub unused_functions: Vec<String>,
}

/// Analyze usage statistics
pub fn analyze_usage(program: &Program) -> UsageAnalysis {
    let mut shaker = TreeShaker::new();
    shaker.mark_used(program);

    let mut total_functions = 0;
    let mut unused_functions = Vec::new();

    for item in &program.items {
        if let Item::Function { decl: func, .. } = item {
            total_functions += 1;
            if !shaker.used_functions.contains(&func.name) {
                unused_functions.push(func.name.clone());
            }
        }
    }

    UsageAnalysis {
        total_functions,
        used_functions: shaker.used_functions.len(),
        unused_functions,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::FunctionDecl;

    #[test]
    fn test_tree_shaker_basic() {
        let program = Program {
            items: vec![
                Item::Function {
                    decl: FunctionDecl {
                        name: "main".to_string(),
                        is_pub: false,
                        is_extern: false,
                        type_params: vec![],
                        where_clause: vec![],
                        decorators: vec![],
                        is_async: false,
                        is_extern: false,
                        parameters: vec![],
                        return_type: None,
                        body: vec![Statement::Expression {
                            expr: Expression::Call {
                                function: Box::new(Expression::Identifier {
                                    name: "used".to_string(),
                                    location: None,
                                }),
                                arguments: vec![],
                                location: None,
                            },
                            location: None,
                        }],
                        parent_type: None,
                    },
                    location: None,
                },
                Item::Function {
                    decl: FunctionDecl {
                        name: "used".to_string(),
                        is_pub: false,
                        is_extern: false,
                        type_params: vec![],
                        where_clause: vec![],
                        decorators: vec![],
                        is_async: false,
                        is_extern: false,
                        parameters: vec![],
                        return_type: None,
                        body: vec![],
                        parent_type: None,
                    },
                    location: None,
                },
                Item::Function {
                    decl: FunctionDecl {
                        name: "unused".to_string(),
                        is_pub: false,
                        is_extern: false,
                        type_params: vec![],
                        where_clause: vec![],
                        decorators: vec![],
                        is_async: false,
                        is_extern: false,
                        parameters: vec![],
                        return_type: None,
                        body: vec![],
                        parent_type: None,
                    },
                    location: None,
                },
            ],
        };

        let mut shaker = TreeShaker::new();
        let shaken = shaker.shake(&program);

        // Should keep main and used, remove unused
        assert_eq!(shaken.items.len(), 2);
    }

    #[test]
    fn test_analyze_usage() {
        let program = Program {
            items: vec![
                Item::Function {
                    decl: FunctionDecl {
                        name: "main".to_string(),
                        is_pub: false,
                        is_extern: false,
                        type_params: vec![],
                        where_clause: vec![],
                        decorators: vec![],
                        is_async: false,
                        is_extern: false,
                        parameters: vec![],
                        return_type: None,
                        body: vec![],
                        parent_type: None,
                    },
                    location: None,
                },
                Item::Function {
                    decl: FunctionDecl {
                        name: "unused".to_string(),
                        is_pub: false,
                        is_extern: false,
                        type_params: vec![],
                        where_clause: vec![],
                        decorators: vec![],
                        is_async: false,
                        is_extern: false,
                        parameters: vec![],
                        return_type: None,
                        body: vec![],
                        parent_type: None,
                    },
                    location: None,
                },
            ],
        };

        let analysis = analyze_usage(&program);
        assert_eq!(analysis.total_functions, 2);
        assert_eq!(analysis.unused_functions.len(), 1);
        assert_eq!(analysis.unused_functions[0], "unused");
    }
}
