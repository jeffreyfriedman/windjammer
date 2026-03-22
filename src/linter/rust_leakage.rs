//! Rust Leakage Detection
//!
//! Warns about Rust-specific patterns in Windjammer code.
//! These patterns work but are not idiomatic - the compiler infers them automatically.

use crate::error::SourceLocation;
use crate::linter::{LintCategory, LintDiagnostic, LintLevel, LintCollector};
use crate::parser::ast::core::{
    Expression, FunctionDecl, Item, Parameter, Program, Statement,
};
use crate::parser::ast::operators::UnaryOp;
use crate::parser::ast::OwnershipHint;
use crate::parser::ast::types::Type;
use crate::source_map::Location;

/// Convert AST location to error SourceLocation
fn to_source_location(loc: Option<Location>, default_file: &str) -> SourceLocation {
    loc.map(|l| SourceLocation {
        file: l.file.to_string_lossy().to_string(),
        line: l.line,
        column: l.column,
    })
    .unwrap_or_else(|| SourceLocation::new(default_file, 1, 1))
}

/// Rust Leakage Linter - detects Rust-specific patterns in Windjammer source
pub struct RustLeakageLinter<'ast> {
    collector: LintCollector,
    default_file: String,
    /// When true, we're inside a trait impl - don't warn on &self/&mut self (trait requires it)
    in_trait_impl: bool,
    _phantom: std::marker::PhantomData<&'ast ()>,
}

impl<'ast> RustLeakageLinter<'ast> {
    pub fn new(default_file: impl Into<String>) -> Self {
        Self {
            collector: LintCollector::new(),
            default_file: default_file.into(),
            in_trait_impl: false,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Run all Rust leakage checks on a program
    pub fn lint_program(&mut self, program: &Program<'ast>) {
        for item in &program.items {
            self.check_item(item);
        }
    }

    fn check_item(&mut self, item: &Item<'ast>) {
        match item {
            Item::Function { decl, location } => {
                self.in_trait_impl = false;
                self.check_function(decl, location.clone());
            }
            Item::Impl { block, location } => {
                let prev = self.in_trait_impl;
                self.in_trait_impl = block.trait_name.is_some();
                for func in &block.functions {
                    self.check_function(func, location.clone());
                }
                self.in_trait_impl = prev;
            }
            Item::Mod { items, .. } => {
                for sub_item in items {
                    self.check_item(sub_item);
                }
            }
            _ => {}
        }
    }

    fn check_function(&mut self, func: &FunctionDecl<'ast>, fallback_loc: Option<Location>) {
        let file = fallback_loc
            .as_ref()
            .map(|l| l.file.to_string_lossy().to_string())
            .unwrap_or_else(|| self.default_file.clone());

        // W0001: Explicit ownership annotations - skip for extern fn and trait impls
        if !func.is_extern && !self.in_trait_impl {
            for param in &func.parameters {
                self.check_explicit_ownership(param, &file);
                self.check_param_type(param, &file);
            }
        }

        // Check body for unwrap, iter, explicit borrows
        for stmt in &func.body {
            self.check_statement(stmt, &file);
        }
    }

    /// W0001: Explicit ownership annotations (&self, &mut self, &T in params)
    fn check_explicit_ownership(&mut self, param: &Parameter<'ast>, file: &str) {
        let (hint, param_name) = match &param.ownership {
            OwnershipHint::Ref => ("&", param.name.as_str()),
            OwnershipHint::Mut => ("&mut", param.name.as_str()),
            _ => return,
        };

        // For self parameter, suggest "self"
        let suggestion = if param_name == "self" {
            "use inferred ownership: `self`".to_string()
        } else {
            format!("use inferred ownership: `{}`", param_name)
        };

        let note = if param_name == "self" {
            if hint == "&mut" {
                "the compiler will add `&mut` based on usage".to_string()
            } else {
                "Windjammer infers ownership automatically".to_string()
            }
        } else {
            "Windjammer infers borrowing automatically".to_string()
        };

        // Parameter doesn't have its own location - use a default
        let location = SourceLocation::new(file, 1, 1);

        self.collector.add(LintDiagnostic {
            lint_name: "W0001".to_string(),
            category: LintCategory::Style,
            level: LintLevel::Note,
            message: "explicit ownership annotation".to_string(),
            location,
            help: Some(suggestion.clone()),
            note: Some(note),
            suggestion: Some(suggestion),
        });
    }

    fn check_statement(&mut self, stmt: &Statement<'ast>, file: &str) {
        match stmt {
            Statement::Let { value, else_block, .. } => {
                self.check_expression(value, file);
                if let Some(block) = else_block {
                    for s in block {
                        self.check_statement(s, file);
                    }
                }
            }
            Statement::Expression { expr, .. } => self.check_expression(expr, file),
            Statement::Assignment { target, value, .. } => {
                self.check_expression(target, file);
                self.check_expression(value, file);
            }
            Statement::Return { value: Some(expr), .. } => self.check_expression(expr, file),
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.check_expression(condition, file);
                for s in then_block {
                    self.check_statement(s, file);
                }
                if let Some(block) = else_block {
                    for s in block {
                        self.check_statement(s, file);
                    }
                }
            }
            Statement::Match { value, arms, .. } => {
                self.check_expression(value, file);
                for arm in arms {
                    self.check_expression(arm.body, file);
                }
            }
            Statement::For { iterable, body, .. } => {
                self.check_expression(iterable, file);
                for s in body {
                    self.check_statement(s, file);
                }
            }
            Statement::While { condition, body, .. } => {
                self.check_expression(condition, file);
                for s in body {
                    self.check_statement(s, file);
                }
            }
            Statement::Loop { body, .. } | Statement::Thread { body, .. } | Statement::Async { body, .. } => {
                for s in body {
                    self.check_statement(s, file);
                }
            }
            Statement::Defer { statement, .. } => self.check_statement(statement, file),
            _ => {}
        }
    }

    fn check_expression(&mut self, expr: &Expression<'ast>, file: &str) {
        match expr {
            Expression::MethodCall {
                method,
                object,
                arguments,
                location,
                ..
            } => {
                // W0002: .unwrap() and .expect()
                if method == "unwrap" || method == "expect" {
                    let loc = to_source_location(location.clone(), &self.default_file);
                    self.collector.add(LintDiagnostic {
                        lint_name: "W0002".to_string(),
                        category: LintCategory::Correctness,
                        level: LintLevel::Warning,
                        message: format!("explicit .{}() call", method),
                        location: loc,
                        help: Some("use explicit error handling: `if let Some(...)` or `match`".to_string()),
                        note: Some(".unwrap() is a Rust-specific panic convention".to_string()),
                        suggestion: Some("prefer explicit error handling in Windjammer".to_string()),
                    });
                }

                // W0003: .iter() and .iter_mut()
                if method == "iter" || method == "iter_mut" {
                    let loc = to_source_location(location.clone(), &self.default_file);
                    self.collector.add(LintDiagnostic {
                        lint_name: "W0003".to_string(),
                        category: LintCategory::Style,
                        level: LintLevel::Note,
                        message: "explicit .iter() call".to_string(),
                        location: loc,
                        help: Some("use direct iteration: `for x in collection`".to_string()),
                        note: Some("Windjammer supports direct iteration".to_string()),
                        suggestion: Some("remove .iter() - use the collection directly".to_string()),
                    });
                }

                self.check_expression(object, file);
                for (_, arg) in arguments {
                    self.check_expression(arg, file);
                }
            }
            Expression::Call {
                function,
                arguments,
                location: _,
                ..
            } => {
                // W0004: Explicit borrowing in function call args (&expr)
                for (_, arg) in arguments {
                    self.check_call_argument(arg, file);
                }
                self.check_expression(function, file);
            }
            Expression::Unary {
                op: _,
                operand,
                location: _,
                ..
            } => {
                // W0004: &expr or &mut expr used as standalone (e.g. in assignment)
                // We check this in check_call_argument for Call args
                // For Unary in other contexts, we could warn - but &x in let y = &x might be intentional
                // The design says to check "explicit borrow in function call" - so we focus on Call args
                self.check_expression(operand, file);
            }
            Expression::Binary { left, right, .. } => {
                self.check_expression(left, file);
                self.check_expression(right, file);
            }
            Expression::FieldAccess { object, .. } => self.check_expression(object, file),
            Expression::Index { object, index, .. } => {
                self.check_expression(object, file);
                self.check_expression(index, file);
            }
            Expression::StructLiteral { fields, .. } => {
                for (_, value) in fields {
                    self.check_expression(value, file);
                }
            }
            Expression::Array { elements, .. } => {
                for elem in elements {
                    self.check_expression(elem, file);
                }
            }
            Expression::Tuple { elements, .. } => {
                for elem in elements {
                    self.check_expression(elem, file);
                }
            }
            Expression::Block { statements, .. } => {
                for stmt in statements {
                    self.check_statement(stmt, file);
                }
            }
            Expression::Closure { body, .. } => self.check_expression(body, file),
            Expression::Cast { expr, .. } => self.check_expression(expr, file),
            Expression::Range { start, end, .. } => {
                self.check_expression(start, file);
                self.check_expression(end, file);
            }
            Expression::MapLiteral { pairs, .. } => {
                for (k, v) in pairs {
                    self.check_expression(k, file);
                    self.check_expression(v, file);
                }
            }
            Expression::TryOp { expr, .. } => self.check_expression(expr, file),
            Expression::Await { expr, .. } => self.check_expression(expr, file),
            Expression::ChannelSend { channel, value, .. } => {
                self.check_expression(channel, file);
                self.check_expression(value, file);
            }
            Expression::ChannelRecv { channel, .. } => self.check_expression(channel, file),
            Expression::MacroInvocation { args, .. } => {
                for arg in args {
                    self.check_expression(arg, file);
                }
            }
            Expression::Literal { .. } | Expression::Identifier { .. } => {}
        }
    }

    /// W0004: Check if a function call argument is an explicit borrow (&expr)
    fn check_call_argument(&mut self, arg: &Expression<'ast>, file: &str) {
        if let Expression::Unary {
            op: UnaryOp::Ref | UnaryOp::MutRef,
            operand: _,
            location,
            ..
        } = arg
        {
            let loc = to_source_location(location.clone(), &self.default_file);
            self.collector.add(LintDiagnostic {
                lint_name: "W0004".to_string(),
                category: LintCategory::Style,
                level: LintLevel::Note,
                message: "explicit borrow in function call".to_string(),
                location: loc,
                help: Some("remove explicit borrow - pass value directly".to_string()),
                note: Some("Windjammer infers borrowing automatically".to_string()),
                suggestion: Some("remove `&` or `&mut`".to_string()),
            });
        } else {
            self.check_expression(arg, file);
        }
    }

    /// Check for explicit reference types in parameters (e.g. &str, &Camera)
    fn check_param_type(&mut self, param: &Parameter<'ast>, file: &str) {
        match &param.type_ {
            Type::Reference(_) | Type::MutableReference(_) => {
                let location = SourceLocation::new(file, 1, 1);
                self.collector.add(LintDiagnostic {
                    lint_name: "W0001".to_string(),
                    category: LintCategory::Style,
                    level: LintLevel::Note,
                    message: "explicit reference type in parameter".to_string(),
                    location,
                    help: Some(format!("use inferred type: remove `&` from `{}`", param.name)),
                    note: Some("Windjammer infers borrowing automatically".to_string()),
                    suggestion: Some(format!("use `{}` without reference type", param.name)),
                });
            }
            _ => {}
        }
    }

    pub fn into_diagnostics(self) -> Vec<LintDiagnostic> {
        self.collector.into_diagnostics()
    }

    pub fn diagnostics(&self) -> &[LintDiagnostic] {
        self.collector.diagnostics()
    }
}
