//! ADR-0014 Phase 1 deprecations: W0007 (`name!(..)` macro spelling, `vec![..]`) and
//! W0008 (`{}`-style format strings). Backed by the same engine as `wj fix`, so every
//! diagnostic carries a span and the exact replacement `wj fix` would apply.

use super::{LintCategory, LintDiagnostic, LintLevel};
use crate::error::SourceLocation;
use crate::intrinsics::surface_fix;

pub fn diagnostics(file: &str, source: &str) -> Vec<LintDiagnostic> {
    surface_fix::lint_source(source)
        .into_iter()
        .map(|f| {
            let help = match &f.replacement {
                Some(rep) => format!("replace `{}` with `{}` (or run `wj fix`)", f.original, rep),
                None => "migrate manually (see docs/design/surface-syntax.md)".to_string(),
            };
            LintDiagnostic {
                lint_name: f.code.to_string(),
                category: LintCategory::Style,
                level: LintLevel::Warning,
                message: f.message.clone(),
                location: SourceLocation::new(file, f.line, f.col),
                help: Some(help),
                note: Some("ADR-0014: Windjammer has no macros; the `!` forms become errors in a future release".to_string()),
                suggestion: f.replacement.clone(),
            }
        })
        .collect()
}
