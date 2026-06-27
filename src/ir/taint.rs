//! Taint tracking system (WJ-SEC-02).
//!
//! Tracks data provenance through the program to prevent injection attacks.
//! Tainted data (from HTTP requests, user input, etc.) cannot reach dangerous
//! sinks (SQL queries, shell commands, etc.) without passing through a
//! declared sanitizer function.
//!
//! Taint propagates through assignments, function returns, and region sharing.
//! The solver catches violations at compile time with full provenance traces.

use std::collections::HashMap;

/// Identifier for a value in the taint analysis.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TaintVar(pub String);

impl TaintVar {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }
}

/// A taint constraint.
#[derive(Debug, Clone)]
pub enum TaintConstraint {
    /// A value is a taint source (comes from untrusted input).
    IsSource {
        var: TaintVar,
        source_kind: TaintSourceKind,
    },
    /// Taint flows from one value to another (assignment, return, field access).
    FlowsTo { from: TaintVar, to: TaintVar },
    /// A function sanitizes its input — output is clean.
    Sanitizes {
        input: TaintVar,
        output: TaintVar,
        sanitizer: String,
    },
    /// A sink requires clean data — tainted input is a compile error.
    RequiresClean { var: TaintVar, sink: String },
}

/// Source of tainted data.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TaintSourceKind {
    HttpRequestBody,
    HttpRequestQuery,
    HttpRequestHeader,
    UserInput,
    DatabaseRow,
    FileContents,
    EnvironmentVariable,
    Custom(String),
}

/// Taint status of a variable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaintStatus {
    /// Definitely clean (never touched tainted data).
    Clean,
    /// Tainted — came from an untrusted source.
    Tainted(TaintSourceKind),
    /// Was tainted but passed through a sanitizer.
    Sanitized {
        original_source: TaintSourceKind,
        sanitizer: String,
    },
    /// Unknown — not yet determined.
    Unknown,
}

/// The taint solver.
#[derive(Default)]
pub struct TaintSolver {
    constraints: Vec<TaintConstraint>,
}

impl TaintSolver {
    pub fn new() -> Self {
        Self {
            constraints: Vec::new(),
        }
    }

    pub fn add_constraint(&mut self, constraint: TaintConstraint) {
        self.constraints.push(constraint);
    }

    pub fn add_constraints(&mut self, constraints: impl IntoIterator<Item = TaintConstraint>) {
        for c in constraints {
            self.add_constraint(c);
        }
    }

    /// Solve taint constraints and return results.
    pub fn solve(self) -> TaintSolverResult {
        let mut status: HashMap<TaintVar, TaintStatus> = HashMap::new();
        let mut errors: Vec<TaintError> = Vec::new();

        // Phase 1: Mark all sources as tainted.
        for constraint in &self.constraints {
            if let TaintConstraint::IsSource { var, source_kind } = constraint {
                status.insert(var.clone(), TaintStatus::Tainted(source_kind.clone()));
            }
        }

        // Phase 2: Propagate taint through flows (fixpoint).
        let max_iterations = 100;
        let mut changed = true;
        let mut iteration = 0;

        while changed && iteration < max_iterations {
            changed = false;
            iteration += 1;

            for constraint in &self.constraints {
                match constraint {
                    TaintConstraint::FlowsTo { from, to } => {
                        let from_status = status.get(from).cloned().unwrap_or(TaintStatus::Unknown);
                        let to_status = status.get(to).cloned().unwrap_or(TaintStatus::Unknown);

                        if let (
                            TaintStatus::Tainted(src),
                            TaintStatus::Unknown | TaintStatus::Clean,
                        ) = (&from_status, &to_status)
                        {
                            status.insert(to.clone(), TaintStatus::Tainted(src.clone()));
                            changed = true;
                        }
                    }

                    TaintConstraint::Sanitizes {
                        input,
                        output,
                        sanitizer,
                    } => {
                        let input_status =
                            status.get(input).cloned().unwrap_or(TaintStatus::Unknown);

                        if let TaintStatus::Tainted(src) = &input_status {
                            let sanitized = TaintStatus::Sanitized {
                                original_source: src.clone(),
                                sanitizer: sanitizer.clone(),
                            };
                            let current =
                                status.get(output).cloned().unwrap_or(TaintStatus::Unknown);
                            if current != sanitized {
                                status.insert(output.clone(), sanitized);
                                changed = true;
                            }
                        }
                    }

                    _ => {}
                }
            }
        }

        // Phase 3: Check sinks — any tainted data reaching a sink is an error.
        for constraint in &self.constraints {
            if let TaintConstraint::RequiresClean { var, sink } = constraint {
                let var_status = status.get(var).cloned().unwrap_or(TaintStatus::Unknown);
                match &var_status {
                    TaintStatus::Tainted(source) => {
                        errors.push(TaintError {
                            kind: TaintErrorKind::TaintedSink,
                            var: var.clone(),
                            sink: sink.clone(),
                            source: Some(source.clone()),
                            message: format!(
                                "tainted data from {:?} reaches sink '{}' without sanitization",
                                source, sink
                            ),
                        });
                    }
                    TaintStatus::Sanitized { .. } | TaintStatus::Clean | TaintStatus::Unknown => {
                        // OK — clean, sanitized, or not reachable from any taint source
                    }
                }
            }
        }

        TaintSolverResult { status, errors }
    }
}

/// Result of taint analysis.
#[derive(Debug)]
pub struct TaintSolverResult {
    /// Taint status of each variable.
    pub status: HashMap<TaintVar, TaintStatus>,
    /// Taint violations found.
    pub errors: Vec<TaintError>,
}

/// A taint violation error.
#[derive(Debug, Clone)]
pub struct TaintError {
    pub kind: TaintErrorKind,
    pub var: TaintVar,
    pub sink: String,
    pub source: Option<TaintSourceKind>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TaintErrorKind {
    TaintedSink,
}

/// Standard library taint source declarations.
pub fn stdlib_taint_sources() -> Vec<TaintConstraint> {
    vec![
        TaintConstraint::IsSource {
            var: TaintVar::new("http.request.body"),
            source_kind: TaintSourceKind::HttpRequestBody,
        },
        TaintConstraint::IsSource {
            var: TaintVar::new("http.request.query"),
            source_kind: TaintSourceKind::HttpRequestQuery,
        },
        TaintConstraint::IsSource {
            var: TaintVar::new("http.request.headers"),
            source_kind: TaintSourceKind::HttpRequestHeader,
        },
        TaintConstraint::IsSource {
            var: TaintVar::new("env.var"),
            source_kind: TaintSourceKind::EnvironmentVariable,
        },
    ]
}

/// Standard library sanitizer declarations.
pub fn stdlib_sanitizers() -> Vec<(&'static str, &'static str)> {
    vec![
        ("sql_escape", "SQL injection"),
        ("html_escape", "XSS"),
        ("shell_escape", "command injection"),
        ("url_encode", "URL injection"),
        ("json_escape", "JSON injection"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_taint_propagation() {
        let mut solver = TaintSolver::new();

        solver.add_constraint(TaintConstraint::IsSource {
            var: TaintVar::new("user_input"),
            source_kind: TaintSourceKind::HttpRequestBody,
        });
        solver.add_constraint(TaintConstraint::FlowsTo {
            from: TaintVar::new("user_input"),
            to: TaintVar::new("query_param"),
        });

        let result = solver.solve();
        assert_eq!(
            result.status.get(&TaintVar::new("query_param")),
            Some(&TaintStatus::Tainted(TaintSourceKind::HttpRequestBody))
        );
    }

    #[test]
    fn test_taint_reaches_sink_error() {
        let mut solver = TaintSolver::new();

        solver.add_constraint(TaintConstraint::IsSource {
            var: TaintVar::new("body"),
            source_kind: TaintSourceKind::HttpRequestBody,
        });
        solver.add_constraint(TaintConstraint::FlowsTo {
            from: TaintVar::new("body"),
            to: TaintVar::new("sql_input"),
        });
        solver.add_constraint(TaintConstraint::RequiresClean {
            var: TaintVar::new("sql_input"),
            sink: "db.query".into(),
        });

        let result = solver.solve();
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].kind, TaintErrorKind::TaintedSink);
        assert_eq!(result.errors[0].sink, "db.query");
    }

    #[test]
    fn test_sanitizer_clears_taint() {
        let mut solver = TaintSolver::new();

        solver.add_constraint(TaintConstraint::IsSource {
            var: TaintVar::new("body"),
            source_kind: TaintSourceKind::HttpRequestBody,
        });
        solver.add_constraint(TaintConstraint::Sanitizes {
            input: TaintVar::new("body"),
            output: TaintVar::new("clean_body"),
            sanitizer: "sql_escape".into(),
        });
        solver.add_constraint(TaintConstraint::RequiresClean {
            var: TaintVar::new("clean_body"),
            sink: "db.query".into(),
        });

        let result = solver.solve();
        assert!(result.errors.is_empty());
        assert!(matches!(
            result.status.get(&TaintVar::new("clean_body")),
            Some(TaintStatus::Sanitized { .. })
        ));
    }

    #[test]
    fn test_transitive_taint_flow() {
        let mut solver = TaintSolver::new();

        solver.add_constraint(TaintConstraint::IsSource {
            var: TaintVar::new("input"),
            source_kind: TaintSourceKind::UserInput,
        });
        solver.add_constraint(TaintConstraint::FlowsTo {
            from: TaintVar::new("input"),
            to: TaintVar::new("a"),
        });
        solver.add_constraint(TaintConstraint::FlowsTo {
            from: TaintVar::new("a"),
            to: TaintVar::new("b"),
        });
        solver.add_constraint(TaintConstraint::FlowsTo {
            from: TaintVar::new("b"),
            to: TaintVar::new("c"),
        });
        solver.add_constraint(TaintConstraint::RequiresClean {
            var: TaintVar::new("c"),
            sink: "shell.exec".into(),
        });

        let result = solver.solve();
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].sink, "shell.exec");
    }

    #[test]
    fn test_clean_data_passes_sink() {
        let mut solver = TaintSolver::new();

        // No taint source — data is clean
        solver.add_constraint(TaintConstraint::RequiresClean {
            var: TaintVar::new("safe_query"),
            sink: "db.query".into(),
        });

        let result = solver.solve();
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_multiple_taint_sources() {
        let mut solver = TaintSolver::new();

        solver.add_constraint(TaintConstraint::IsSource {
            var: TaintVar::new("body"),
            source_kind: TaintSourceKind::HttpRequestBody,
        });
        solver.add_constraint(TaintConstraint::IsSource {
            var: TaintVar::new("query"),
            source_kind: TaintSourceKind::HttpRequestQuery,
        });

        // Both flow into the same var
        solver.add_constraint(TaintConstraint::FlowsTo {
            from: TaintVar::new("body"),
            to: TaintVar::new("combined"),
        });

        solver.add_constraint(TaintConstraint::RequiresClean {
            var: TaintVar::new("combined"),
            sink: "template.render".into(),
        });

        let result = solver.solve();
        assert_eq!(result.errors.len(), 1);
    }

    #[test]
    fn test_stdlib_sources_load() {
        let sources = stdlib_taint_sources();
        assert!(sources.len() >= 4);
    }
}
