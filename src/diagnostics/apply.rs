//! Applying machine-applicable fixes to source text.

use super::model::{Applicability, Diagnostic, Edit};
use std::collections::BTreeMap;

/// Collect edits from all `MachineApplicable` fixes, grouped by file.
/// A fix whose edits overlap an already accepted edit is skipped as a whole
/// (rustfix semantics), so applying never produces garbled text.
pub fn collect_edits(diags: &[Diagnostic]) -> BTreeMap<String, Vec<Edit>> {
    let mut by_file: BTreeMap<String, Vec<Edit>> = BTreeMap::new();
    for d in diags {
        for fix in &d.fixes {
            if fix.applicability != Applicability::MachineApplicable || fix.edits.is_empty() {
                continue;
            }
            let conflicts = fix.edits.iter().any(|e| {
                by_file.get(&e.span.file).is_some_and(|acc| {
                    acc.iter().any(|a| overlaps(a, e))
                })
            });
            if conflicts {
                continue;
            }
            for e in &fix.edits {
                by_file.entry(e.span.file.clone()).or_default().push(e.clone());
            }
        }
    }
    by_file
}

fn overlaps(a: &Edit, b: &Edit) -> bool {
    let (a0, a1, b0, b1) = (a.span.byte_start, a.span.byte_end, b.span.byte_start, b.span.byte_end);
    if a0 == a1 && b0 == b1 {
        return a0 == b0;
    }
    a0 < b1 && b0 < a1
}

/// Apply non-overlapping edits to `text`. Returns `None` when an edit is out of range or
/// does not fall on char boundaries.
pub fn apply_edits(text: &str, edits: &[Edit]) -> Option<String> {
    let mut sorted: Vec<&Edit> = edits.iter().collect();
    sorted.sort_by_key(|e| (e.span.byte_start, e.span.byte_end));
    let mut out = String::with_capacity(text.len());
    let mut cursor = 0;
    for e in sorted {
        let (s, t) = (e.span.byte_start, e.span.byte_end);
        if s < cursor || t < s || t > text.len() || !text.is_char_boundary(s) || !text.is_char_boundary(t) {
            return None;
        }
        out.push_str(&text[cursor..s]);
        out.push_str(&e.replacement);
        cursor = t;
    }
    out.push_str(&text[cursor..]);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::model::{Fix, Span};

    fn edit(s: usize, e: usize, r: &str) -> Edit {
        Edit {
            span: Span {
                file: "a.wj".into(),
                byte_start: s,
                byte_end: e,
                line_start: 1,
                col_start: 1,
                line_end: 1,
                col_end: 1,
                label: None,
            },
            replacement: r.into(),
        }
    }

    fn diag(applicability: Applicability, edits: Vec<Edit>) -> Diagnostic {
        Diagnostic::warning("W0007", "m").with_fix(Fix {
            description: "d".into(),
            applicability,
            edits,
        })
    }

    #[test]
    fn applies_in_order_and_skips_non_machine_applicable() {
        let ds = vec![
            diag(Applicability::MachineApplicable, vec![edit(0, 1, "X"), edit(4, 5, "Y")]),
            diag(Applicability::MaybeIncorrect, vec![edit(2, 3, "Z")]),
        ];
        let edits = collect_edits(&ds);
        assert_eq!(apply_edits("abcdef", &edits["a.wj"]).unwrap(), "XbcdYf");
    }

    #[test]
    fn overlapping_fix_is_skipped_whole() {
        let ds = vec![
            diag(Applicability::MachineApplicable, vec![edit(0, 3, "A")]),
            diag(Applicability::MachineApplicable, vec![edit(2, 4, "B"), edit(5, 6, "C")]),
        ];
        let edits = collect_edits(&ds);
        assert_eq!(apply_edits("abcdef", &edits["a.wj"]).unwrap(), "Adef");
    }

    #[test]
    fn out_of_range_edit_is_rejected() {
        assert!(apply_edits("ab", &[edit(1, 9, "x")]).is_none());
    }
}
