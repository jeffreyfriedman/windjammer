//! Taint tracking system (WJ-SEC-02).
//!
//! Tracks data provenance through the program to prevent injection attacks.
//! Tainted data (from HTTP requests, user input, etc.) cannot reach dangerous
//! sinks (SQL queries, shell commands, etc.) without passing through a
//! declared sanitizer function.
//!
//! Taint propagates through assignments, function returns, and region sharing.
//! The solver catches violations at compile time with full provenance traces.

use crate::analyzer::{FunctionSignature, OwnershipMode, SignatureRegistry};
use crate::ir::effects::{effects_for_runtime_callee, runtime_module_segment};
use crate::ir::safety_type::Effect;
use crate::parser::Type;
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

/// Source of tainted data (canonical definition for the IR).
/// Used by both `SafetyType.taint` and `TaintSolver`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TaintSourceKind {
    HttpRequest,
    HttpRequestBody,
    HttpRequestQuery,
    HttpRequestHeader,
    UserInput,
    DatabaseRow,
    FileContents,
    EnvironmentVariable,
    Custom(String),
}

/// Where tainted data originated, with location info.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaintSource {
    pub kind: TaintSourceKind,
    pub location: String,
}

/// Taint status of a value (canonical definition for the IR).
/// Used by both `SafetyType.taint` and `TaintSolver`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaintStatus {
    /// Definitely clean (never touched tainted data).
    Clean,
    /// Tainted — came from an untrusted source.
    Tainted(TaintSource),
    /// Was tainted but passed through a sanitizer.
    Sanitized {
        original_source: TaintSourceKind,
        sanitizer: String,
    },
    /// Unknown — not yet determined by the solver.
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
                status.insert(
                    var.clone(),
                    TaintStatus::Tainted(TaintSource {
                        kind: source_kind.clone(),
                        location: String::new(),
                    }),
                );
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
                            TaintStatus::Tainted(ref src),
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

                        if let TaintStatus::Tainted(ref src) = &input_status {
                            let sanitized = TaintStatus::Sanitized {
                                original_source: src.kind.clone(),
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
                            source: Some(source.kind.clone()),
                            message: format!(
                                "tainted data from {:?} reaches sink '{}' without sanitization",
                                source.kind, sink
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

fn resolve_runtime_sig<'a>(
    registry: &'a SignatureRegistry,
    qualified_name: &str,
) -> Option<&'a FunctionSignature> {
    let module = runtime_module_segment(qualified_name);
    let simple = qualified_name.rsplit("::").next().unwrap_or(qualified_name);
    registry
        .get_signature(qualified_name)
        .or_else(|| registry.get_signature(&format!("{module}::{simple}")))
        .or_else(|| registry.get_signature(&format!("std::{module}::{simple}")))
        .or_else(|| {
            qualified_name
                .rsplit_once("::")
                .and_then(|(ty, method)| registry.get_signature(&format!("{ty}::{method}")))
        })
}

fn is_borrowed_text_param(sig: &FunctionSignature, arg_index: usize) -> bool {
    let idx = sig.arg_param_index(arg_index);
    sig.param_ownership
        .get(idx)
        .is_some_and(|o| matches!(o, OwnershipMode::Borrowed | OwnershipMode::MutBorrowed))
        && sig.param_types.get(idx).is_some_and(|t| {
            matches!(
                t,
                Type::String | Type::Reference(_) | Type::MutableReference(_)
            )
        })
}

fn returns_owned_text(sig: &FunctionSignature) -> bool {
    match sig.return_type.as_ref() {
        Some(Type::String) => true,
        Some(Type::Result(ok, _)) => matches!(**ok, Type::String | Type::Custom(_)),
        _ => false,
    }
}

/// HTTP request field accessors: match scanned struct fields, not a method-name table.
fn http_request_field_source(
    registry: &SignatureRegistry,
    type_name: &str,
    method: &str,
) -> Option<TaintSourceKind> {
    let fields = registry.runtime_type_fields(type_name);
    if !is_http_request_shape(fields) {
        return None;
    }
    for (field, _ty) in fields {
        if method_projects_request_field(method, field) {
            if let Some(kind) = taint_kind_from_request_field(field) {
                return Some(kind);
            }
        }
    }
    None
}

fn is_http_request_shape(fields: &[(String, String)]) -> bool {
    let names: Vec<&str> = fields.iter().map(|(n, _)| n.as_str()).collect();
    names.contains(&"query") || (names.contains(&"path") && names.contains(&"body"))
}

fn method_projects_request_field(method: &str, field: &str) -> bool {
    method == field || method.starts_with(field) || field.starts_with(method)
}

/// Inbound HTTP request field roles from scanned struct schema (not callee names).
fn taint_kind_from_request_field(field: &str) -> Option<TaintSourceKind> {
    match field {
        "body" => Some(TaintSourceKind::HttpRequestBody),
        "query" => Some(TaintSourceKind::HttpRequestQuery),
        "headers" => Some(TaintSourceKind::HttpRequestHeader),
        _ => None,
    }
}

fn type_is_http_request_type(registry: &SignatureRegistry, type_name: &str) -> bool {
    let base = type_name.rsplit("::").next().unwrap_or(type_name);
    is_http_request_shape(registry.runtime_type_fields(base))
}

/// Taint source for a runtime / std callee (signature + effects, not a function-name list).
pub fn taint_source_for_callee(qualified_name: &str) -> Option<TaintSourceKind> {
    let registry = SignatureRegistry::stdlib();
    let sig = resolve_runtime_sig(&registry, qualified_name);

    if let Some((receiver, method)) = qualified_name.rsplit_once("::") {
        if type_is_http_request_type(&registry, receiver) {
            if let Some(kind) = http_request_field_source(&registry, receiver, method) {
                return Some(kind);
            }
        }
    }

    let effects = effects_for_runtime_callee(qualified_name);
    let module = runtime_module_segment(qualified_name);
    if module == "env" && effects.contains(&Effect::EnvRead) {
        return Some(TaintSourceKind::EnvironmentVariable);
    }
    if module == "fs" && effects.contains(&Effect::FsRead) {
        return Some(TaintSourceKind::FileContents);
    }
    if module == "db" && effects.contains(&Effect::NetEgress) {
        return Some(TaintSourceKind::DatabaseRow);
    }
    if module == "io" {
        if sig.is_some_and(|s| !matches!(s.return_type, Some(Type::Bool))) {
            return Some(TaintSourceKind::UserInput);
        }
    }
    None
}

/// Dangerous sink description for a callee (derived from scanned effects).
pub fn taint_sink_for_callee(qualified_name: &str) -> Option<&'static str> {
    let effects = effects_for_runtime_callee(qualified_name);
    let module = runtime_module_segment(qualified_name);
    if effects.contains(&Effect::ProcessSpawn) {
        return Some("shell command");
    }
    if effects.contains(&Effect::FsWrite) {
        return Some("file write path");
    }
    if module == "db" && effects.contains(&Effect::NetEgress) {
        return Some("SQL query");
    }
    None
}

/// True when a scanned signature is a text-in / text-out transform marked as a sanitizer.
pub fn is_sanitizer_callee(qualified_name: &str) -> bool {
    let registry = SignatureRegistry::stdlib();
    if !registry.is_taint_sanitizer(qualified_name) {
        return false;
    }
    let Some(sig) = resolve_runtime_sig(&registry, qualified_name) else {
        return false;
    };
    if !is_borrowed_text_param(sig, 0) || !returns_owned_text(sig) {
        return false;
    }
    let effects = effects_for_runtime_callee(qualified_name);
    !effects.iter().any(|e| {
        matches!(
            e,
            Effect::ProcessSpawn
                | Effect::FsWrite
                | Effect::FsRead
                | Effect::NetEgress
                | Effect::NetIngress
        )
    })
}

/// Standard library taint source declarations from scanned runtime signatures.
pub fn stdlib_taint_sources() -> Vec<TaintConstraint> {
    let registry = SignatureRegistry::stdlib();
    let mut out = Vec::new();
    for (name, _sig) in registry.all_signatures_for_suffix_search() {
        if let Some(kind) = taint_source_for_callee(name) {
            out.push(TaintConstraint::IsSource {
                var: TaintVar::new(name.clone()),
                source_kind: kind.clone(),
            });
            if !name.starts_with("std::") {
                out.push(TaintConstraint::IsSource {
                    var: TaintVar::new(format!("std::{name}")),
                    source_kind: kind,
                });
            }
        }
    }
    out
}

/// Standard library dangerous sink declarations (effect-driven).
pub fn stdlib_sinks() -> Vec<TaintConstraint> {
    let registry = SignatureRegistry::stdlib();
    let mut out = Vec::new();
    for (name, _sig) in registry.all_signatures_for_suffix_search() {
        if let Some(desc) = taint_sink_for_callee(name) {
            out.push(TaintConstraint::RequiresClean {
                var: TaintVar::new(format!("{name}::arg0")),
                sink: format!("{name} ({desc})"),
            });
        }
    }
    out
}

/// Legacy helper — prefer [`is_sanitizer_callee`].
pub fn stdlib_sanitizers() -> Vec<(&'static str, &'static str)> {
    vec![]
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
        assert!(matches!(
            result.status.get(&TaintVar::new("query_param")),
            Some(TaintStatus::Tainted(src)) if src.kind == TaintSourceKind::HttpRequestBody
        ));
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
        assert!(
            sources.len() >= 4,
            "scanned runtime must declare multiple taint sources, got {}",
            sources.len()
        );
    }

    #[test]
    fn scanned_server_request_body_and_regex_escape_classify() {
        assert_eq!(
            taint_source_for_callee("ServerRequest::body_string"),
            Some(TaintSourceKind::HttpRequestBody)
        );
        assert_eq!(
            taint_source_for_callee("ServerRequest::query_param"),
            Some(TaintSourceKind::HttpRequestQuery)
        );
        assert!(
            is_sanitizer_callee("regex::escape") || is_sanitizer_callee("regex_mod::escape"),
            "regex escape must be scanned wj-taint sanitizer"
        );
        assert!(
            !is_sanitizer_callee("strings::trim") && !is_sanitizer_callee("strings::to_uppercase"),
            "plain string transforms must not be sanitizers without wj-taint annotation"
        );
        assert_eq!(
            taint_source_for_callee("Request::query_param"),
            Some(TaintSourceKind::HttpRequestQuery)
        );
        assert!(
            taint_source_for_callee("ServerResponse::ok").is_none(),
            "HTTP response constructors are not request taint sources"
        );
        assert!(
            taint_sink_for_callee("process::run").is_some(),
            "process::run must be effect-driven sink"
        );
    }
}
