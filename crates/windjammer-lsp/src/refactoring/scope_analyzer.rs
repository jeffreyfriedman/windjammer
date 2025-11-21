#![allow(dead_code)] // Refactoring implementation - some parts planned for future versions
//! Scope analysis for refactoring operations
//!
//! This module analyzes variable scopes to determine:
//! - Which variables are read from outer scope (→ parameters)
//! - Which variables are written and used after (→ return values)
//! - Which variables are purely local (→ stay inside)

use std::collections::{HashMap, HashSet};
use windjammer::parser::{Expression, Pattern, Statement};

/// Result of analyzing a code selection for extraction
#[derive(Debug, Clone)]
pub struct ScopeAnalysis {
    /// Variables read from outer scope (will become parameters)
    pub parameters: Vec<Variable>,

    /// Variables written in selection and used after (will be returned)
    pub return_values: Vec<Variable>,

    /// Variables that are purely local to the selection
    pub local_variables: Vec<Variable>,

    /// Variables that are captured from outer scope but not as parameters
    /// (e.g., from closures - may need special handling)
    pub captured: Vec<Variable>,

    /// Whether the selection contains early returns
    pub has_early_return: bool,

    /// Whether the selection contains break/continue
    pub has_control_flow: bool,
}

/// Information about a variable in scope
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Variable {
    /// Variable name
    pub name: String,

    /// Inferred type (if known)
    pub type_name: Option<String>,

    /// Is the variable mutable?
    pub is_mutable: bool,

    /// Line where variable is defined
    pub defined_at: Option<usize>,
}

/// Analyzes the scope of a code selection
pub struct ScopeAnalyzer {
    /// Variables defined before the selection
    outer_scope: HashSet<String>,

    /// Variables defined within the selection
    inner_scope: HashSet<String>,

    /// Variables read within the selection
    reads: HashMap<String, Variable>,

    /// Variables written within the selection  
    writes: HashMap<String, Variable>,

    /// Variables used after the selection
    used_after: HashSet<String>,
}

impl ScopeAnalyzer {
    /// Create a new scope analyzer
    pub fn new() -> Self {
        Self {
            outer_scope: HashSet::new(),
            inner_scope: HashSet::new(),
            reads: HashMap::new(),
            writes: HashMap::new(),
            used_after: HashSet::new(),
        }
    }

    /// Analyze a selection of statements
    pub fn analyze(
        &mut self,
        before_statements: &[Statement],
        selected_statements: &[Statement],
        after_statements: &[Statement],
    ) -> ScopeAnalysis {
        // Phase 1: Build outer scope from statements before selection
        self.collect_outer_scope(before_statements);

        // Phase 2: Analyze the selection
        self.analyze_statements(selected_statements);

        // Phase 3: Find variables used after selection
        self.collect_used_after(after_statements);

        // Phase 4: Categorize variables
        self.categorize_variables()
    }

    /// Collect variables defined before the selection
    fn collect_outer_scope(&mut self, statements: &[Statement]) {
        for stmt in statements {
            Self::collect_definitions_in_statement(stmt, &mut self.outer_scope);
        }
    }

    /// Collect variables used after the selection
    fn collect_used_after(&mut self, statements: &[Statement]) {
        for stmt in statements {
            Self::collect_usages_in_statement(stmt, &mut self.used_after);
        }
    }

    /// Analyze the selected statements
    fn analyze_statements(&mut self, statements: &[Statement]) {
        for stmt in statements {
            self.analyze_statement(stmt);
        }
    }

    /// Analyze a single statement
    fn analyze_statement(&mut self, stmt: &Statement) {
        match stmt {
            Statement::Let {
                pattern,
                mutable,
                value,
                ..
            } => {
                // Only handle simple identifier patterns
                if let Pattern::Identifier(name) = pattern {
                    // Record definition
                    self.inner_scope.insert(name.clone());

                    // Analyze the value expression
                    self.analyze_expression(value);

                    // Record write
                    self.writes.insert(
                        name.clone(),
                        Variable {
                            name: name.clone(),
                            type_name: None, // TODO: Type inference
                            is_mutable: *mutable,
                            defined_at: None,
                        },
                    );
                } else {
                    // For non-identifier patterns (tuple, wildcard), just analyze the value
                    self.analyze_expression(value);
                }
            }

            Statement::Expression { expr, location: _ } => {
                self.analyze_expression(expr);
            }

            Statement::Assignment {
                target,
                value,
                location: _,
            } => {
                // Record write to target
                if let Expression::Identifier { name, location: _ } = target {
                    self.writes.insert(
                        name.clone(),
                        Variable {
                            name: name.clone(),
                            type_name: None,
                            is_mutable: true,
                            defined_at: None,
                        },
                    );
                }

                // Analyze value
                self.analyze_expression(value);
            }

            Statement::Return {
                value: Some(expr),
                location: _,
            } => {
                self.analyze_expression(expr);
            }
            Statement::Return {
                value: None,
                location: _,
            } => {}

            Statement::If {
                condition,
                then_block,
                else_block,
                location: _,
            } => {
                self.analyze_expression(condition);
                self.analyze_statements(then_block);
                if let Some(else_stmts) = else_block {
                    self.analyze_statements(else_stmts);
                }
            }

            Statement::While {
                condition,
                body,
                location: _,
            } => {
                self.analyze_expression(condition);
                self.analyze_statements(body);
            }

            Statement::For { .. } => {
                // TODO: Implement for loop analysis
            }

            _ => {
                // TODO: Handle other statement types
            }
        }
    }

    /// Analyze an expression for variable usage
    fn analyze_expression(&mut self, expr: &Expression) {
        match expr {
            Expression::Identifier { name, location: _ } => {
                // Is this a read from outer scope?
                if !self.inner_scope.contains(name) {
                    self.reads.insert(
                        name.clone(),
                        Variable {
                            name: name.clone(),
                            type_name: None,
                            is_mutable: false,
                            defined_at: None,
                        },
                    );
                }
            }

            Expression::Binary { left, right, .. } => {
                self.analyze_expression(left);
                self.analyze_expression(right);
            }

            Expression::Unary { operand, .. } => {
                self.analyze_expression(operand);
            }

            Expression::Call {
                function,
                arguments,
                location: _,
            } => {
                self.analyze_expression(function);
                for (_, arg) in arguments {
                    self.analyze_expression(arg);
                }
            }

            Expression::MethodCall {
                object, arguments, ..
            } => {
                self.analyze_expression(object);
                for (_, arg) in arguments {
                    self.analyze_expression(arg);
                }
            }

            Expression::FieldAccess { object, .. } => {
                self.analyze_expression(object);
            }

            // Ternary operator removed from Windjammer - use if/else expressions instead
            _ => {
                // Literals, etc. don't have variable usage
            }
        }
    }

    /// Collect variable definitions in a statement
    fn collect_definitions_in_statement(stmt: &Statement, scope: &mut HashSet<String>) {
        match stmt {
            Statement::Let {
                pattern: windjammer::parser::Pattern::Identifier(name),
                ..
            } => {
                scope.insert(name.clone());
            }
            Statement::Let { .. } => {
                // Non-identifier patterns (tuple, wildcard) - skip for now
            }
            Statement::For {
                pattern: windjammer::parser::Pattern::Identifier(var),
                ..
            } => {
                // For simple identifier patterns, add to scope
                scope.insert(var.clone());
            }
            Statement::For { .. } => {
                // For tuple patterns, we'd need to extract all identifiers
                // For now, skip complex patterns
            }
            _ => {}
        }
    }

    /// Collect variable usages in a statement
    fn collect_usages_in_statement(stmt: &Statement, usages: &mut HashSet<String>) {
        match stmt {
            Statement::Expression { expr, location: _ } => {
                Self::collect_usages_in_expression(expr, usages);
            }
            Statement::Return {
                value: Some(expr),
                location: _,
            } => {
                Self::collect_usages_in_expression(expr, usages);
            }
            Statement::Assignment { value, .. } => {
                Self::collect_usages_in_expression(value, usages);
            }
            Statement::If {
                condition,
                then_block,
                else_block,
                location: _,
            } => {
                Self::collect_usages_in_expression(condition, usages);
                for stmt in then_block {
                    Self::collect_usages_in_statement(stmt, usages);
                }
                if let Some(else_stmts) = else_block {
                    for stmt in else_stmts {
                        Self::collect_usages_in_statement(stmt, usages);
                    }
                }
            }
            _ => {}
        }
    }

    /// Collect variable usages in an expression
    fn collect_usages_in_expression(expr: &Expression, usages: &mut HashSet<String>) {
        match expr {
            Expression::Identifier { name, location: _ } => {
                usages.insert(name.clone());
            }
            Expression::Binary { left, right, .. } => {
                Self::collect_usages_in_expression(left, usages);
                Self::collect_usages_in_expression(right, usages);
            }
            Expression::Unary { operand, .. } => {
                Self::collect_usages_in_expression(operand, usages);
            }
            Expression::Call {
                function,
                arguments,
                location: _,
            } => {
                Self::collect_usages_in_expression(function, usages);
                for (_, arg) in arguments {
                    Self::collect_usages_in_expression(arg, usages);
                }
            }
            _ => {}
        }
    }

    /// Categorize variables into parameters, return values, and local
    fn categorize_variables(&self) -> ScopeAnalysis {
        let mut parameters = Vec::new();
        let mut return_values = Vec::new();
        let mut local_variables = Vec::new();

        // Variables read from outer scope → parameters
        for (name, var) in &self.reads {
            if self.outer_scope.contains(name) {
                parameters.push(var.clone());
            }
        }

        // Variables written in selection and used after → return values
        for (name, var) in &self.writes {
            if self.used_after.contains(name) {
                return_values.push(var.clone());
            } else if !self.outer_scope.contains(name) {
                // Written but not used after, and not from outer scope → local
                local_variables.push(var.clone());
            }
        }

        ScopeAnalysis {
            parameters,
            return_values,
            local_variables,
            captured: Vec::new(),    // TODO: Implement closure capture detection
            has_early_return: false, // TODO: Detect early returns
            has_control_flow: false, // TODO: Detect break/continue
        }
    }
}

impl Default for ScopeAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windjammer::parser::{BinaryOp, Expression, Statement};

    #[test]
    fn test_simple_parameter_detection() {
        let mut analyzer = ScopeAnalyzer::new();

        // Before: let x = 10;
        let before = vec![Statement::Let {
            pattern: windjammer::parser::Pattern::Identifier("x".to_string()),
            mutable: false,
            type_: None,
            value: Expression::Literal {
                value: windjammer::parser::Literal::Int(10),
                location: None,
            },
            location: None,
        }];

        // Selection: let y = x + 5;
        let selected = vec![Statement::Let {
            pattern: windjammer::parser::Pattern::Identifier("y".to_string()),
            mutable: false,
            type_: None,
            value: Expression::Binary {
                left: Box::new(Expression::Identifier {
                    name: "x".to_string(),
                    location: None,
                }),
                op: BinaryOp::Add,
                right: Box::new(Expression::Literal {
                    value: windjammer::parser::Literal::Int(5),
                    location: None,
                }),
                location: None,
            },
            location: None,
        }];

        // After: (empty)
        let after = vec![];

        let analysis = analyzer.analyze(&before, &selected, &after);

        // x should be detected as a parameter
        assert_eq!(analysis.parameters.len(), 1);
        assert_eq!(analysis.parameters[0].name, "x");

        // y is local (not used after)
        assert_eq!(analysis.return_values.len(), 0);
    }

    #[test]
    fn test_return_value_detection() {
        let mut analyzer = ScopeAnalyzer::new();

        let before = vec![];

        // Selection: let result = 42;
        let selected = vec![Statement::Let {
            pattern: windjammer::parser::Pattern::Identifier("result".to_string()),
            mutable: false,
            type_: None,
            value: Expression::Literal {
                value: windjammer::parser::Literal::Int(42),
                location: None,
            },
            location: None,
        }];

        // After: println(result);
        let after = vec![Statement::Expression {
            expr: Expression::Call {
                function: Box::new(Expression::Identifier {
                    name: "println".to_string(),
                    location: None,
                }),
                arguments: vec![(
                    None,
                    Expression::Identifier {
                        name: "result".to_string(),
                        location: None,
                    },
                )],
                location: None,
            },
            location: None,
        }];

        let analysis = analyzer.analyze(&before, &selected, &after);

        // result should be detected as a return value
        assert_eq!(analysis.return_values.len(), 1);
        assert_eq!(analysis.return_values[0].name, "result");
    }

    #[test]
    fn test_parameter_and_return() {
        let mut analyzer = ScopeAnalyzer::new();

        // Before: let x = 10;
        let before = vec![Statement::Let {
            pattern: windjammer::parser::Pattern::Identifier("x".to_string()),
            mutable: false,
            type_: None,
            value: Expression::Literal {
                value: windjammer::parser::Literal::Int(10),
                location: None,
            },
            location: None,
        }];

        // Selection: let y = x * 2;
        let selected = vec![Statement::Let {
            pattern: windjammer::parser::Pattern::Identifier("y".to_string()),
            mutable: false,
            type_: None,
            value: Expression::Binary {
                left: Box::new(Expression::Identifier {
                    name: "x".to_string(),
                    location: None,
                }),
                op: BinaryOp::Mul,
                right: Box::new(Expression::Literal {
                    value: windjammer::parser::Literal::Int(2),
                    location: None,
                }),
                location: None,
            },
            location: None,
        }];

        // After: use(y);
        let after = vec![Statement::Expression {
            expr: Expression::Call {
                function: Box::new(Expression::Identifier {
                    name: "use".to_string(),
                    location: None,
                }),
                arguments: vec![(
                    None,
                    Expression::Identifier {
                        name: "y".to_string(),
                        location: None,
                    },
                )],
                location: None,
            },
            location: None,
        }];

        let analysis = analyzer.analyze(&before, &selected, &after);

        // x is parameter, y is return value
        assert_eq!(analysis.parameters.len(), 1);
        assert_eq!(analysis.parameters[0].name, "x");
        assert_eq!(analysis.return_values.len(), 1);
        assert_eq!(analysis.return_values[0].name, "y");
    }
}
