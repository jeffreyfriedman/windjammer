/// Mutability Error Checking
///
/// Detects when variables are mutated without `mut` keyword
/// and provides helpful error messages with suggestions.
use crate::parser::*;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct MutabilityError {
    pub variable: String,
    pub error_type: MutabilityErrorType,
    pub location: SourceLocation,
    pub suggestion: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MutabilityErrorType {
    Reassignment,
    CompoundAssignment,
    FieldMutation,
    MutatingMethodCall,
}

impl MutabilityError {
    pub fn format_error(&self) -> String {
        let error_msg = match self.error_type {
            MutabilityErrorType::Reassignment => {
                format!(
                    "cannot assign twice to immutable variable `{}`",
                    self.variable
                )
            }
            MutabilityErrorType::CompoundAssignment => {
                format!(
                    "cannot use compound assignment on immutable variable `{}`",
                    self.variable
                )
            }
            MutabilityErrorType::FieldMutation => {
                format!(
                    "cannot mutate field of immutable binding `{}`",
                    self.variable
                )
            }
            MutabilityErrorType::MutatingMethodCall => {
                format!(
                    "cannot borrow `{}` as mutable, as it is not declared as mutable",
                    self.variable
                )
            }
        };

        // Handle default location if none provided
        let (file_display, line, column) = if let Some(loc) = &self.location {
            (loc.file.display().to_string(), loc.line, loc.column)
        } else {
            ("unknown".to_string(), 0, 0)
        };

        format!(
            "error: {}\n  --> {}:{}:{}\n   |\nhelp: {}",
            error_msg, file_display, line, column, self.suggestion
        )
    }
}

pub struct MutabilityChecker {
    /// Variables declared in current scope and whether they're mutable
    declared_variables: HashMap<String, bool>,
    /// Errors found
    errors: Vec<MutabilityError>,
    /// Current source file (for future error reporting enhancements)
    #[allow(dead_code)]
    current_file: std::path::PathBuf,
}

impl MutabilityChecker {
    pub fn new(file: std::path::PathBuf) -> Self {
        MutabilityChecker {
            declared_variables: HashMap::new(),
            errors: Vec::new(),
            current_file: file,
        }
    }

    pub fn check_function(&mut self, func: &FunctionDecl) -> Vec<MutabilityError> {
        self.declared_variables.clear();
        self.errors.clear();

        // NOTE: We do NOT track parameters here!
        // Parameter ownership (including &mut inference) is handled by the Analyzer.
        // The mutability checker only checks LOCAL VARIABLES declared with `let`.
        // This prevents false positives where a parameter like `fn foo(x: T)` gets
        // inferred as `fn foo(x: &mut T)` by the analyzer, but the mutability checker
        // complains before that inference happens.

        // Check function body
        self.check_statements(&func.body);

        self.errors.clone()
    }

    fn check_statements<'ast>(&mut self, statements: &[&'ast Statement<'ast>]) {
        for stmt in statements {
            self.check_statement(stmt);
        }
    }

    fn check_statement(&mut self, stmt: &Statement) {
        match stmt {
            Statement::Let {
                pattern,
                mutable,
                value,
                else_block,
                ..
            } => {
                // Track variable declaration
                if let Pattern::Identifier(name) = pattern {
                    self.declared_variables.insert(name.clone(), *mutable);
                }

                // Check the value expression
                self.check_expression(value);

                // Check else block if present
                if let Some(else_stmts) = else_block {
                    self.check_statements(else_stmts);
                }
            }
            Statement::Assignment {
                target,
                value,
                compound_op,
                location,
            } => {
                // TDD: Auto-mutability inference
                // THE WINDJAMMER WAY: Compiler infers `mut` when field mutations detected
                if let Some(var_name) = self.get_variable_name(target) {
                    if let Some(&is_mutable) = self.declared_variables.get(&var_name) {
                        if !is_mutable {
                            // Check if this is a field mutation (e.g., point.x = 10)
                            if self.is_field_access(target) {
                                // AUTO-MUTABILITY: Automatically mark as mutable!
                                // No error - the compiler infers `mut` for us
                                self.declared_variables.insert(var_name.clone(), true);
                            } else {
                                // Direct reassignment or compound assignment still errors
                                let error_type = if compound_op.is_some() {
                                    MutabilityErrorType::CompoundAssignment
                                } else {
                                    MutabilityErrorType::Reassignment
                                };

                                self.errors.push(MutabilityError {
                                    variable: var_name.clone(),
                                    error_type,
                                    location: location.clone(),
                                    suggestion: format!(
                                        "make this binding mutable: `mut {}`",
                                        var_name
                                    ),
                                });
                            }
                        }
                    }
                }

                self.check_expression(value);
            }
            Statement::Expression { expr, .. } => {
                self.check_expression(expr);
            }
            Statement::Return {
                value: Some(expr), ..
            } => {
                self.check_expression(expr);
            }
            Statement::Return { value: None, .. } => {}
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.check_expression(condition);
                self.check_statements(then_block);
                if let Some(else_stmts) = else_block {
                    self.check_statements(else_stmts);
                }
            }
            Statement::While {
                condition, body, ..
            } => {
                self.check_expression(condition);
                self.check_statements(body);
            }
            Statement::For { iterable, body, .. } => {
                self.check_expression(iterable);
                self.check_statements(body);
            }
            Statement::Loop { body, .. } => {
                self.check_statements(body);
            }
            Statement::Match { value, arms, .. } => {
                self.check_expression(value);
                for arm in arms {
                    if let Some(guard) = &arm.guard {
                        self.check_expression(guard);
                    }
                    self.check_expression(arm.body);
                }
            }
            _ => {}
        }
    }

    fn check_expression(&mut self, expr: &Expression) {
        match expr {
            Expression::MethodCall {
                object,
                method,
                arguments,
                location,
                ..
            } => {
                // Check if this is a mutating method call on an immutable variable
                if self.is_mutating_method(method) {
                    if let Some(var_name) = self.get_variable_name(object) {
                        if let Some(&is_mutable) = self.declared_variables.get(&var_name) {
                            if !is_mutable {
                                self.errors.push(MutabilityError {
                                    variable: var_name.clone(),
                                    error_type: MutabilityErrorType::MutatingMethodCall,
                                    location: location.clone(),
                                    suggestion: format!(
                                        "make this binding mutable: `mut {}`",
                                        var_name
                                    ),
                                });
                            }
                        }
                    }
                }

                self.check_expression(object);
                for (_, arg) in arguments {
                    self.check_expression(arg);
                }
            }
            Expression::Binary { left, right, .. } => {
                self.check_expression(left);
                self.check_expression(right);
            }
            Expression::Unary { operand, .. } => {
                self.check_expression(operand);
            }
            Expression::Call { arguments, .. } => {
                for (_, arg) in arguments {
                    self.check_expression(arg);
                }
            }
            Expression::Index { object, index, .. } => {
                self.check_expression(object);
                self.check_expression(index);
            }
            Expression::FieldAccess { object, .. } => {
                self.check_expression(object);
            }
            Expression::Block { statements, .. } => {
                self.check_statements(statements);
            }
            _ => {}
        }
    }

    fn get_variable_name(&self, expr: &Expression) -> Option<String> {
        match expr {
            Expression::Identifier { name, .. } => Some(name.clone()),
            Expression::FieldAccess { object, .. } => self.get_variable_name(object),
            _ => None,
        }
    }

    fn is_field_access(&self, expr: &Expression) -> bool {
        matches!(expr, Expression::FieldAccess { .. })
    }

    fn is_mutating_method(&self, method: &str) -> bool {
        // Common mutating methods
        matches!(
            method,
            "push"
                | "pop"
                | "insert"
                | "remove"
                | "clear"
                | "append"
                | "extend"
                | "push_front"
                | "push_back"
                | "pop_front"
                | "pop_back"
                | "retain"
                | "dedup"
                | "sort"
                | "reverse"
                | "swap"
        )
    }
}
