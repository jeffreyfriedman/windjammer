//! The diagnostic data model (schema v2, see `docs/diagnostics-schema.json`).
//!
//! One shape for every producer (front end, lints, rustc mapper, IR solvers) and every consumer
//! (`wj check --json`, `wj build --json`, the LSP, `wj test --json`, human rendering).

use serde::{Deserialize, Serialize};

/// Bumped on any breaking change to the JSON shape. v1 was the legacy `wj build --json` output.
pub const SCHEMA_VERSION: u32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Note,
    Help,
}

/// How safe it is to apply a fix without a human looking (same levels as rustfix).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Applicability {
    /// Semantics-preserving or certainly what the user meant; `wj check --fix` applies it.
    MachineApplicable,
    /// A plausible fix that may change behaviour; never applied automatically.
    MaybeIncorrect,
    /// Contains a placeholder the user must fill in.
    HasPlaceholders,
    Unspecified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    Syntax,
    NameResolution,
    Type,
    Ownership,
    Mutability,
    Module,
    Lint,
    Effects,
    Internal,
}

/// Which stage produced the diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Origin {
    /// Found by the Windjammer front end (no rustc involved).
    FrontEnd,
    /// A rustc error translated back to `.wj` source (a front-end coverage gap, ADR-0005).
    RustcMapped,
    /// A lint or deprecation.
    Lint,
    /// A compiler bug or an untranslatable rustc error.
    Internal,
}

/// A source range. Lines and columns are 1-based; columns count Unicode scalar values;
/// byte offsets are 0-based and half-open (`byte_end` is exclusive).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
    pub file: String,
    pub byte_start: usize,
    pub byte_end: usize,
    pub line_start: usize,
    pub col_start: usize,
    pub line_end: usize,
    pub col_end: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

impl Span {
    /// Assertion layer (debug builds, tests, `WJ_STRICT_SPANS=1`): a span handed to a diagnostic
    /// must be structurally valid. Source-aware checks live in `error::span_check::validate_span`.
    pub fn assert_wellformed(&self) {
        crate::error::span_check::assert_extent(
            &self.file,
            (self.line_start, self.col_start),
            (self.line_end, self.col_end),
        );
        if self.byte_start > self.byte_end && crate::error::span_check::is_strict() {
            panic!("invalid diagnostic span in {}: byte_start {} > byte_end {}", self.file, self.byte_start, self.byte_end);
        }
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
}

/// One text replacement. Inserting is an empty range; deleting is an empty replacement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Edit {
    #[serde(flatten)]
    pub span: Span,
    pub replacement: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fix {
    pub description: String,
    pub applicability: Applicability,
    pub edits: Vec<Edit>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Docs {
    /// `wj explain CODE`
    pub explain: String,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Diagnostic {
    /// Stable code: `WJxxxx` (errors) or `Wxxxx` (lints). Never reused.
    pub code: String,
    pub severity: Severity,
    pub category: Category,
    pub origin: Origin,
    pub message: String,
    #[serde(default)]
    pub primary_span: Option<Span>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub secondary_spans: Vec<Span>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub help: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fixes: Vec<Fix>,
    pub docs: Docs,
    /// How much the compiler trusts this diagnostic (`proven`, `high_confidence`, `heuristic`, `hint`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maturity: Option<String>,
}

impl Diagnostic {
    /// Build a diagnostic for a registered code; category, origin and docs come from the registry.
    pub fn new(code: &str, severity: Severity, message: impl Into<String>) -> Self {
        let info = super::codes::lookup(code);
        Self {
            code: code.to_string(),
            severity,
            category: info.map_or(Category::Internal, |i| i.category),
            origin: info.map_or(Origin::Internal, |i| i.origin),
            message: message.into(),
            primary_span: None,
            secondary_spans: Vec::new(),
            help: Vec::new(),
            notes: Vec::new(),
            fixes: Vec::new(),
            docs: Docs {
                explain: format!("wj explain {code}"),
                url: format!("https://windjammer.dev/errors/{code}"),
            },
            maturity: info.map(|i| i.maturity.to_string()),
        }
    }

    pub fn error(code: &str, message: impl Into<String>) -> Self {
        Self::new(code, Severity::Error, message)
    }
    pub fn warning(code: &str, message: impl Into<String>) -> Self {
        Self::new(code, Severity::Warning, message)
    }
    pub fn at(mut self, span: Span) -> Self {
        span.assert_wellformed();
        self.primary_span = Some(span);
        self
    }
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help.push(help.into());
        self
    }
    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }
    pub fn with_fix(mut self, fix: Fix) -> Self {
        fix.edits.iter().for_each(|e| e.span.assert_wellformed());
        self.fixes.push(fix);
        self
    }
    pub fn with_secondary(mut self, span: Span) -> Self {
        span.assert_wellformed();
        self.secondary_spans.push(span);
        self
    }
    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Summary {
    pub errors: usize,
    pub warnings: usize,
    pub files_checked: usize,
    pub files_cached: usize,
    pub elapsed_ms: u64,
}

/// The complete output of one `check` / `build` / `test` invocation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Report {
    pub schema_version: u32,
    pub windjammer_version: String,
    /// `check`, `build` or `test`.
    pub command: String,
    pub success: bool,
    pub summary: Summary,
    pub diagnostics: Vec<Diagnostic>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effects_summary: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub taint_summary: Option<serde_json::Value>,
}

impl Report {
    pub fn new(command: &str, diagnostics: Vec<Diagnostic>) -> Self {
        let errors = diagnostics.iter().filter(|d| d.is_error()).count();
        let warnings = diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Warning)
            .count();
        Self {
            schema_version: SCHEMA_VERSION,
            windjammer_version: env!("CARGO_PKG_VERSION").to_string(),
            command: command.to_string(),
            success: errors == 0,
            summary: Summary {
                errors,
                warnings,
                ..Summary::default()
            },
            diagnostics,
            effects_summary: None,
            taint_summary: None,
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self)
            .unwrap_or_else(|e| format!("{{\"error\": \"serialization failed: {e}\"}}"))
    }
}
