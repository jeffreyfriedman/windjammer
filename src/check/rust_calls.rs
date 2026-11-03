//! Calls into Rust crates checked against their signature surfaces (ADR-0026).
//!
//! For a crate with `rust/<crate>.wji`: `WJ0050` for a function the surface does not declare (and for a
//! malformed surface), `WJ0013` for a wrong argument count, `WJ0003` for a literal of the wrong kind. Calls are
//! `crate::f(..)` or `f(..)` brought in by `use crate::f` / `use crate::*`. A call into a crate with no
//! surface at all (hand-written or generated into `.wj-cache`) is `W0014` with the exact `wj rust-surface`
//! command; the effect pass still attributes it (`ir::effect_facts`). Stale generated surfaces: `rust_surface_health`.

use super::front::Parsed;
use super::locate::Locator;
use super::semantic::{check_literal, walk_block, walk_expr, Index, Visitor};
use crate::diagnostics::{Diagnostic, SourceIndex, Span};
use crate::package::rust_surface::{RustCrates, Surface};
use crate::parser::ast::core::{Expression, Item};
use crate::source_map::Location;
use std::collections::{BTreeMap, HashMap};

/// Crate -> (functions called, first call site), for crates that have no surface.
type Missing = BTreeMap<String, (Vec<String>, Option<Span>)>;

pub fn run(crates: &RustCrates, idx: &Index, parsed: &[&Parsed], out: &mut Vec<Diagnostic>) {
    if crates.is_empty() {
        return;
    }
    for s in &crates.surfaces {
        surface_problems(s, out);
    }
    let mut missing = Missing::new();
    for p in parsed {
        let imports = imports(crates, &p.program.items);
        let six = SourceIndex::new(p.file.clone(), &p.text);
        let mut v = Calls { crates, idx, imports, loc: Locator::new(&six), macro_locs: Vec::new(), missing: &mut missing, out: &mut *out };
        v.items(&p.program.items);
    }
    for (krate, (fns, span)) in missing {
        let mut d = Diagnostic::warning("W0014", format!("Rust crate `{krate}` has no signature surface, so its calls are not checked"));
        if let Some(sp) = span {
            d = d.at(sp.with_label("called here"));
        }
        let pin: String = fns.iter().map(|f| format!(" --fn {f}")).collect();
        out.push(
            d.with_help(format!("generate it: `wj rust-surface {krate}` (or `wj build`, which generates it into .wj-cache/rust-surfaces/)"))
                .with_note(format!("to pin a curated copy in rust/{krate}.wji: `wj rust-surface {krate} --write{pin}`")),
        );
    }
}

fn surface_problems(s: &Surface, out: &mut Vec<Diagnostic>) {
    let file = s.path.display().to_string();
    for p in &s.problems {
        let mut d = Diagnostic::error("WJ0050", format!("invalid signature surface for Rust crate `{}`: {}", s.crate_name, p.message));
        if p.line > 0 {
            let ix = SourceIndex::new(file.clone(), &s.text);
            let loc = Some(Location { file: s.path.clone(), line: p.line, column: 1 });
            if let Some(sp) = Locator::new(&ix).word(&loc, "fn").or_else(|| Locator::new(&ix).first_use("fn")) {
                d = d.at(sp.with_label("in this declaration"));
            }
        } else {
            d = d.with_note(format!("in {file}"));
        }
        out.push(d.with_help("a surface holds body-less, non-generic `fn` declarations, e.g. `pub fn levenshtein(a: string, b: string) -> usize`"));
    }
}

/// Bare names imported from surfaced crates: `use strsim::levenshtein` (alias honoured) and `use strsim::*`.
fn imports(crates: &RustCrates, items: &[Item<'_>]) -> HashMap<String, (String, String)> {
    let mut out = HashMap::new();
    for item in items {
        let Item::Use { path, alias, .. } = item else { continue };
        let (Some(head), Some(last)) = (path.first(), path.last()) else { continue };
        let Some(s) = crates.surface(head) else {
            if crates.is_rust_crate(head) && path.len() == 2 && last != "*" {
                out.insert(alias.clone().unwrap_or_else(|| last.clone()), (head.clone(), last.clone()));
            }
            continue;
        };
        if path.len() == 2 && last == "*" {
            for f in &s.fns {
                out.insert(f.name.clone(), (head.clone(), f.name.clone()));
            }
        } else if path.len() == 2 {
            out.insert(alias.clone().unwrap_or_else(|| last.clone()), (head.clone(), last.clone()));
        }
    }
    out
}

struct Calls<'a, 'o> {
    crates: &'a RustCrates,
    idx: &'a Index,
    imports: HashMap<String, (String, String)>,
    loc: Locator<'a>,
    macro_locs: Vec<Option<Location>>,
    missing: &'o mut Missing,
    out: &'o mut Vec<Diagnostic>,
}

impl Calls<'_, '_> {
    fn items(&mut self, items: &[Item<'_>]) {
        for item in items {
            match item {
                Item::Function { decl, .. } => walk_block(self, &decl.body),
                Item::Impl { block, .. } => block.functions.iter().for_each(|f| walk_block(self, &f.body)),
                Item::Trait { decl, .. } => decl.methods.iter().filter_map(|m| m.body.as_ref()).for_each(|b| walk_block(self, b)),
                Item::Mod { items, .. } => self.items(items),
                Item::Const { value, .. } | Item::Static { value, .. } => walk_expr(self, value),
                _ => {}
            }
        }
    }

    fn span(&self, loc: &Option<Location>, word: &str) -> Option<Span> {
        let loc = match (loc, self.macro_locs.last()) {
            (Some(l), Some(m)) if l.file.as_os_str().is_empty() => m.clone(),
            _ => loc.clone(),
        };
        self.loc.word(&loc, word).or_else(|| self.loc.first_use(word))
    }

    fn call(&mut self, written: &str, loc: &Option<Location>, arguments: &[(Option<String>, &Expression<'_>)]) {
        let (krate, func) = match written.split_once("::") {
            Some((h, f)) if !f.contains("::") && self.crates.is_rust_crate(h) => (h.to_string(), f.to_string()),
            None => match self.imports.get(written) {
                Some((c, f)) => (c.clone(), f.clone()),
                None => return,
            },
            _ => return,
        };
        let last = written.rsplit("::").next().unwrap_or(written);
        let Some(s) = self.crates.surface(&krate) else {
            let span = self.span(loc, last);
            let entry = self.missing.entry(krate).or_insert_with(|| (Vec::new(), span));
            if !entry.0.contains(&func) {
                entry.0.push(func);
            }
            return;
        };
        let path = format!("{krate}::{func}");
        let Some(decl) = s.get(&func) else {
            let mut d = Diagnostic::error(
                "WJ0050",
                format!("function `{path}` is not declared in the signature surface of Rust crate `{krate}`"),
            );
            if let Some(sp) = self.span(loc, last) {
                d = d.at(sp.with_label("not in the surface"));
            }
            let best = s
                .fns
                .iter()
                .map(|f| (crate::fuzzy_matcher::levenshtein_distance(&func, &f.name), &f.name))
                .filter(|(dist, _)| *dist <= (func.len() / 3).clamp(1, 3))
                .min();
            d = match (s.skipped.iter().find(|(n, _)| *n == func), best) {
                (Some((_, why)), _) => d.with_help(format!(
                    "the generator skipped it ({why}); to declare it by hand run `wj rust-surface {krate} --write` and add `pub fn {func}(..)` to rust/{krate}.wji"
                )),
                (None, Some((_, n))) => d.with_help(format!("a function with a similar name is declared: `{krate}::{n}`")),
                (None, None) => d.with_help(format!("declare it in {} (`pub fn {func}(..) -> ..`)", s.path.display())),
            };
            self.out.push(d);
            return;
        };
        let params: Vec<_> = decl.parameters.iter().filter(|p| p.name != "self").collect();
        if params.len() != arguments.len() {
            let n = params.len();
            let mut d = Diagnostic::error(
                "WJ0013",
                format!(
                    "function `{path}` takes {n} argument{} but {} {} supplied",
                    if n == 1 { "" } else { "s" },
                    arguments.len(),
                    if arguments.len() == 1 { "was" } else { "were" }
                ),
            );
            if let Some(sp) = self.span(loc, last) {
                d = d.at(sp.with_label(format!("expected {n} argument(s)")));
            }
            self.out.push(d.with_help(format!("declared in {}", s.path.display())));
            return;
        }
        for ((_, arg), p) in arguments.iter().zip(params) {
            if let Expression::Literal { value, location } = arg {
                let span = self.loc.literal(location, value);
                if let Some(d) = check_literal(value, &p.type_, self.idx, span, &format!(" in argument of `{path}`")) {
                    self.out.push(d);
                }
            }
        }
    }
}

impl Visitor for Calls<'_, '_> {
    fn expr(&mut self, e: &Expression<'_>) {
        if let Expression::Call { function, arguments, .. } = e {
            if let Expression::Identifier { name, location } = function {
                let (name, location) = (name.clone(), location.clone());
                self.call(&name, &location, arguments);
            }
        }
    }

    fn macro_enter(&mut self, e: &Expression<'_>) {
        self.macro_locs.push(e.location());
    }

    fn macro_exit(&mut self) {
        self.macro_locs.pop();
    }
}
