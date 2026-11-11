//! ADR-0002: a local binding is immutable unless declared `mut`. `WJ0051` reports a `let` (or `for`) binding that
//! the program changes without declaring it `mut`, at the first change, with the declaration as a second span and a
//! machine-applicable fix that inserts `mut `.
//!
//! Decided from declarations and solver facts (`facts.rs`): a method changes its receiver when the IR solver gives
//! it `&mut self` (user types) or `std/rust_core.wji` declares it `&mut self` (core types); an argument is changed
//! when the solver lends that parameter `&mut`. Bindings whose value the declarations do not decide are only judged
//! by assignments to the binding itself, so the check never reports a binding that is not changed.
//!
//! One run covers the whole program (every file at once), so `wj check`, `wj build` and `wj run` report the same.

mod decl;
pub mod facts;
mod types;
mod walk;

use super::front::Parsed;
use crate::diagnostics::{Applicability, Diagnostic, Edit, Fix, SourceIndex};
use crate::ir::stdlib_tables::StdlibTables;
use crate::parser::ast::core::Item;
use std::sync::Arc;
use walk::{Change, Found, Kind, Walker};

/// The registry code.
pub const CODE: &str = "WJ0051";


/// `WJ0051` diagnostics for every file of one program.
pub fn run(parsed: &[&Parsed], tables: Arc<StdlibTables>) -> Vec<Diagnostic> {
    let files: Vec<(&str, &crate::parser::ast::core::Program<'static>)> = parsed.iter().map(|p| (p.file.as_str(), &p.program)).collect();
    let facts = facts::Facts::build(&files, tables);
    let mut out = Vec::new();
    for p in parsed {
        if facts.unanalyzed.contains(&p.file) {
            continue;
        }
        let mut w = Walker::new(&facts);
        functions(&p.program.items, &mut w);
        let ix = SourceIndex::new(p.file.clone(), &p.text);
        let loc = super::locate::Locator::new(&ix);
        let masked = super::locate::masked_ranges(&p.text);
        for f in &w.found {
            if let Some(d) = diagnostic(f, &ix, &loc, &masked) {
                out.push(d);
            }
        }
    }
    out
}

fn functions(items: &[Item<'static>], w: &mut Walker<'_>) {
    for item in items {
        match item {
            Item::Function { decl, .. } => w.function(decl),
            Item::Impl { block, .. } => block.functions.iter().for_each(|f| w.function(f)),
            Item::Mod { items, .. } => functions(items, w),
            _ => {}
        }
    }
}

fn label(f: &Found) -> String {
    let n = &f.binding.name;
    let what = match &f.change {
        Change::Assign { compound: false } => "assigned here".to_string(),
        Change::Assign { compound: true } => "changed here".to_string(),
        Change::Field => format!("a field of `{n}` is assigned here"),
        Change::Element => format!("an element of `{n}` is assigned here"),
        Change::Method(m) => format!("`{m}` changes `{n}` here"),
        Change::Argument(c) => format!("`{c}` changes `{n}` here"),
        Change::LendMut => format!("`{n}` is lent for change here"),
    };
    if f.in_closure {
        format!("{what} (inside a closure)")
    } else {
        what
    }
}

fn diagnostic(f: &Found, ix: &SourceIndex<'_>, loc: &super::locate::Locator<'_>, masked: &[(usize, usize)]) -> Option<Diagnostic> {
    let name = &f.binding.name;
    let keyword = if f.binding.kind == Kind::For { "for" } else { "let" };
    let end = f.binding.stmt_end.as_ref().map(|l| ix.offset(l.line, l.column))?;
    let at_name = decl::binding_offset(ix.text(), masked, end, keyword, name)?;
    let declared = ix.span(at_name, at_name + name.len()).with_label("declared without `mut`");
    let changed = loc.word(&f.at, name)?.with_label(label(f));
    let message = match f.change {
        Change::Assign { .. } => format!("`{name}` is assigned after its declaration but is not declared `mut`"),
        _ => format!("`{name}` is changed but is not declared `mut`"),
    };
    let note = "local bindings are immutable unless declared `mut` (ADR-0002); `wj fix` adds every missing `mut`";
    let help = if f.binding.kind == Kind::For { format!("declare it `for mut {name} in ..`") } else { format!("declare it with `mut {name}`") };
    Some(
        Diagnostic::error(CODE, message)
            .at(changed)
            .with_secondary(declared)
            .with_help(help)
            .with_note(note)
            .with_fix(Fix {
                description: format!("declare `{name}` as `mut`"),
                applicability: Applicability::MachineApplicable,
                edits: vec![Edit { span: ix.span(at_name, at_name), replacement: "mut ".to_string() }],
            }),
    )
}
