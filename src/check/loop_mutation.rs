//! WJ0202 in `wj check`: a `for` loop that changes the collection it iterates (found by
//! `analyzer::for_in_mutation`). Primary span: the change inside the body; secondary span: the iterated expression of
//! the `for` header. No machine-applicable fix: whether the author wants a snapshot, an index loop or a list of
//! changes applied afterwards is their intent.

use super::front::Parsed;
use crate::analyzer::for_in_mutation;
use crate::diagnostics::{Diagnostic, SourceIndex};

pub fn loops_that_change_what_they_iterate(p: &Parsed, ix: &SourceIndex, out: &mut Vec<Diagnostic>) {
    for f in for_in_mutation::check(&p.program, &p.parser.span_snapshot()) {
        let Some(change) = f.change_span else { continue };
        let mut d = Diagnostic::error(for_in_mutation::CODE, f.message())
            .at(ix.span(change.byte_start, change.byte_end).with_label(f.label()))
            .with_note(f.note());
        if let Some(h) = f.header_span {
            d = d.with_secondary(ix.span(h.byte_start, h.byte_end).with_label(f.header_label()));
        }
        for h in f.help() {
            d = d.with_help(h);
        }
        out.push(d);
    }
}
