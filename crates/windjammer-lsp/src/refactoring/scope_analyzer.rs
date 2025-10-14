//! Scope analysis for refactoring operations
//!
//! This module analyzes variable scopes to determine:
//! - Which variables are read from outer scope (→ parameters)
//! - Which variables are written and used after (→ return values)
//! - Which variables are purely local (→ stay inside)

use std::collections::{HashMap, HashSet};
use windjammer::parser::{Expression, Statement};

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
                name,
                mutable,
                value,
                ..
            } => {
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
            }

            Statement::Expression(expr) => {
                self.analyze_expression(expr);
            }

            Statement::Assignment { target, value } => {
                // Record write to target
                if let Expression::Identifier(name) = target {
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

            Statement::Return(expr) => {
                if let Some(expr) = expr {
                    self.analyze_expression(expr);
                }
            }

            Statement::If {
                condition,
                then_block,
                else_block,
            } => {
                self.analyze_expression(condition);
                self.analyze_statements(then_block);
                if let Some(else_stmts) = else_block {
                    self.analyze_statements(else_stmts);
                }
            }

            Statement::While { condition, body } => {
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
            Expression::Identifier(name) => {
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

            Expression::Ternary {
                condition,
                true_expr,
                false_expr,
            } => {
                self.analyze_expression(condition);
                self.analyze_expression(true_expr);
                self.analyze_expression(false_expr);
            }

            _ => {
                // Literals, etc. don't have variable usage
            }
        }
    }

    /// Collect variable definitions in a statement
    fn collect_definitions_in_statement(stmt: &Statement, scope: &mut HashSet<String>) {
        match stmt {
            Statement::Let { name, .. } => {
                scope.insert(name.clone());
            }
            Statement::For { variable, .. } => {
                scope.insert(variable.clone());
            }
            _ => {}
        }
    }

    /// Collect variable usages in a statement
    fn collect_usages_in_statement(stmt: &Statement, usages: &mut HashSet<String>) {
        match stmt {
            Statement::Expression(expr) => {
                Self::collect_usages_in_expression(expr, usages);
            }
            Statement::Return(Some(expr)) => {
                Self::collect_usages_in_expression(expr, usages);
            }
            Statement::Assignment { value, .. } => {
                Self::collect_usages_in_expression(value, usages);
            }
            Statement::If {
                condition,
                then_block,
                else_block,
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
            Expression::Identifier(name) => {
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
            name: "x".to_string(),
            mutable: false,
            type_: None,
            value: Expression::Literal(windjammer::parser::Literal::Int(10)),
        }];

        // Selection: let y = x + 5;
        let selected = vec![Statement::Let {
            name: "y".to_string(),
            mutable: false,
            type_: None,
            value: Expression::Binary {
                left: Box::new(Expression::Identifier("x".to_string())),
                op: BinaryOp::Add,
                right: Box::new(Expression::Literal(windjammer::parser::Literal::Int(5))),
            },
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
            name: "result".to_string(),
            mutable: false,
            type_: None,
            value: Expression::Literal(windjammer::parser::Literal::Int(42)),
        }];

        // After: println(result);
        let after = vec![Statement::Expression(Expression::Call {
            function: Box::new(Expression::Identifier("println".to_string())),
            arguments: vec![(None, Expression::Identifier("result".to_string()))],
        })];

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
            name: "x".to_string(),
            mutable: false,
            type_: None,
            value: Expression::Literal(windjammer::parser::Literal::Int(10)),
        }];

        // Selection: let y = x * 2;
        let selected = vec![Statement::Let {
            name: "y".to_string(),
            mutable: false,
            type_: None,
            value: Expression::Binary {
                left: Box::new(Expression::Identifier("x".to_string())),
                op: BinaryOp::Mul,
                right: Box::new(Expression::Literal(windjammer::parser::Literal::Int(2))),
            },
        }];

        // After: use(y);
        let after = vec![Statement::Expression(Expression::Call {
            function: Box::new(Expression::Identifier("use".to_string())),
            arguments: vec![(None, Expression::Identifier("y".to_string()))],
        })];

        let analysis = analyzer.analyze(&before, &selected, &after);

        // x is parameter, y is return value
        assert_eq!(analysis.parameters.len(), 1);
        assert_eq!(analysis.parameters[0].name, "x");
        assert_eq!(analysis.return_values.len(), 1);
        assert_eq!(analysis.return_values[0].name, "y");
    }
}
