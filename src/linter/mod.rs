//! Windjammer Linter - Compile predictably, warn helpfully
//!
//! Philosophy:
//! - ✅ Compile what the user wrote (predictability)
//! - ⚠️ Warn about inefficiencies (education)
//! - 📖 Explain why (understanding)
//!
//! This follows the Rust/Clippy model: code compiles, but warnings guide toward better patterns.

pub mod rust_leakage;

use crate::analyzer::AnalyzedFunction;
use crate::error::SourceLocation;
use crate::parser::{Expression, Statement};
use std::fmt;

/// Lint severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LintLevel {
    Error,   // Must fix (blocks compilation)
    Warning, // Should fix (default for most lints)
    Note,    // Could improve (informational)
    Allow,   // Disabled
}

/// Lint category for organization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LintCategory {
    Performance, // Efficiency issues
    Correctness, // Bugs and mistakes
    Style,       // Idiomatic code
    Complexity,  // Code complexity
}

/// Individual lint diagnostic
#[derive(Debug, Clone)]
pub struct LintDiagnostic {
    pub lint_name: String,
    pub category: LintCategory,
    pub level: LintLevel,
    pub message: String,
    pub location: SourceLocation,
    pub help: Option<String>,
    pub note: Option<String>,
    pub suggestion: Option<String>,
}

impl fmt::Display for LintDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let level_str = match self.level {
            LintLevel::Error => "error",
            LintLevel::Warning => "warning",
            LintLevel::Note => "note",
            LintLevel::Allow => return Ok(()), // Don't display allowed lints
        };

        writeln!(f, "{}: {} [{}]", level_str, self.message, self.lint_name)?;
        writeln!(
            f,
            "  --> {}:{}:{}",
            self.location.file, self.location.line, self.location.column
        )?;

        if let Some(note) = &self.note {
            writeln!(f, "  = note: {}", note)?;
        }

        if let Some(help) = &self.help {
            writeln!(f, "  = help: {}", help)?;
        }

        if let Some(suggestion) = &self.suggestion {
            writeln!(f, "  = suggestion: {}", suggestion)?;
        }

        Ok(())
    }
}

/// Lint collector for a compilation unit
#[derive(Debug, Default)]
pub struct LintCollector {
    diagnostics: Vec<LintDiagnostic>,
}

impl LintCollector {
    pub fn new() -> Self {
        Self {
            diagnostics: Vec::new(),
        }
    }

    pub fn add(&mut self, diagnostic: LintDiagnostic) {
        self.diagnostics.push(diagnostic);
    }

    pub fn diagnostics(&self) -> &[LintDiagnostic] {
        &self.diagnostics
    }

    /// Consume the collector and return all diagnostics
    pub fn into_diagnostics(self) -> Vec<LintDiagnostic> {
        self.diagnostics
    }

    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(|d| d.level == LintLevel::Error)
    }

    pub fn has_warnings(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.level == LintLevel::Warning)
    }
}

/// Linter for Windjammer code
pub struct Linter<'ast> {
    collector: LintCollector,
    _phantom: std::marker::PhantomData<&'ast ()>,
}

impl<'ast> Default for Linter<'ast> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'ast> Linter<'ast> {
    pub fn new() -> Self {
        Self {
            collector: LintCollector::new(),
            _phantom: std::marker::PhantomData,
        }
    }

    /// Run all lints on an analyzed function
    pub fn lint_function(&mut self, analyzed: &AnalyzedFunction<'ast>) {
        self.lint_owned_but_not_returned(analyzed);
        // NOTE: explicit-to-string lint removed - compiler already normalizes string literals!
        // Future lints:
        // - unnecessary-clone
        // - owned-string-param
        // - needless-borrow
    }

    /// Lint: owned-but-not-returned
    ///
    /// Warns when a function takes an owned parameter, mutates it, but never returns it.
    /// This is wasteful - the parameter is moved in, modified, then dropped.
    ///
    /// Better: Use &mut T to borrow mutably.
    fn lint_owned_but_not_returned(&mut self, analyzed: &AnalyzedFunction<'ast>) {
        use crate::analyzer::OwnershipMode;

        for param in &analyzed.decl.parameters {
            // Check if parameter is owned
            let ownership = analyzed
                .inferred_ownership
                .get(&param.name)
                .unwrap_or(&OwnershipMode::Owned);

            if !matches!(ownership, OwnershipMode::Owned) {
                continue; // Only check owned parameters
            }

            // Check if parameter is mutated
            let is_mutated = analyzed.mutated_parameters.contains(&param.name);
            if !is_mutated {
                continue; // Only check mutated parameters
            }

            // Check if parameter is returned
            let is_returned = Self::parameter_is_returned(&param.name, &analyzed.decl.body);
            if is_returned {
                continue; // Parameter is returned, so owned makes sense
            }

            // Check if parameter type is Copy (Copy types should be passed by value)
            // For Copy types, owned is correct even if mutated
            // TODO: Implement is_copy_type check properly using struct registry
            // For now, skip primitives (i32, f32, bool, etc.)
            use crate::parser::Type;
            if matches!(
                param.type_,
                Type::Int | Type::Int32 | Type::Uint | Type::Float | Type::Bool
            ) {
                continue;
            }

            // LINT: Parameter is owned, mutated, but not returned - suggest &mut
            // NOTE: FunctionDecl doesn't have a location field, use first line of file
            let location = SourceLocation {
                file: format!("{}.wj", analyzed.decl.name), // Approximate
                line: 1,
                column: 1,
            };

            self.collector.add(LintDiagnostic {
                lint_name: "owned-but-not-returned".to_string(),
                category: LintCategory::Performance,
                level: LintLevel::Warning,
                message: format!("Parameter `{}` is mutated but not returned", param.name),
                location,
                help: Some(format!(
                    "Consider using `&mut {}` for efficiency",
                    match &param.type_ {
                        crate::parser::Type::Custom(name) => name.clone(),
                        _ => format!("{:?}", param.type_),
                    }
                )),
                note: Some(
                    "Owned parameters that aren't returned waste a move operation".to_string(),
                ),
                suggestion: Some(format!(
                    "Change `{}: {}` to `{}: &mut {}`",
                    param.name,
                    match &param.type_ {
                        crate::parser::Type::Custom(name) => name.clone(),
                        crate::parser::Type::String => "string".to_string(),
                        _ => format!("{:?}", param.type_),
                    },
                    param.name,
                    match &param.type_ {
                        crate::parser::Type::Custom(name) => name.clone(),
                        crate::parser::Type::String => "string".to_string(),
                        _ => format!("{:?}", param.type_),
                    }
                )),
            });
        }
    }

    /// Check if a parameter is returned from the function
    fn parameter_is_returned(param_name: &str, body: &[&Statement]) -> bool {
        for stmt in body {
            match stmt {
                Statement::Return {
                    value: Some(expr), ..
                } => {
                    if Self::expression_contains_identifier(expr, param_name) {
                        return true;
                    }
                }
                Statement::Expression { expr, .. } => {
                    // Check if last expression returns the parameter
                    if Self::expression_contains_identifier(expr, param_name) {
                        return true;
                    }
                }
                _ => {}
            }
        }

        false
    }

    /// Check if an expression contains an identifier
    fn expression_contains_identifier(expr: &Expression, name: &str) -> bool {
        match expr {
            Expression::Identifier { name: id_name, .. } => id_name == name,
            Expression::FieldAccess { object, .. } => {
                Self::expression_contains_identifier(object, name)
            }
            // Add more cases as needed
            _ => false,
        }
    }

    // NOTE: explicit-to-string lint removed
    // The compiler already normalizes "text".to_string() → "text" at parse/codegen time
    // No need for a lint - the feature works perfectly!

    /// Get all diagnostics collected
    pub fn into_diagnostics(self) -> Vec<LintDiagnostic> {
        self.collector.diagnostics
    }

    /// Get diagnostics by level
    pub fn diagnostics_by_level(&self, level: LintLevel) -> Vec<&LintDiagnostic> {
        self.collector
            .diagnostics
            .iter()
            .filter(|d| d.level == level)
            .collect()
    }
}
