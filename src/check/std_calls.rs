//! `WJ0002` for a call to a function a std module does not provide: `strings.index_of(..)` after `use std::strings`,
//! `strings::index_of(..)`, `std::strings::index_of(..)`. What exists is `stdlib_surface::std_functions` (every
//! engine's functions); a module the table does not cover is never judged. Only names bound by an explicit
//! `use std::<module>` (or its alias) count as the module, so a local value that happens to be called `path` or
//! `log` is not mistaken for it.

use super::front::Parsed;
use super::locate::Locator;
use super::semantic::{walk_block, walk_expr, Visitor};
use crate::diagnostics::{Applicability, Diagnostic, Edit, Fix, SourceIndex, Span};
use crate::parser::ast::core::{Expression, FunctionDecl, Item, Pattern};
use crate::source_map::Location;
use crate::stdlib_surface::std_functions;
use std::collections::{HashMap, HashSet};

pub fn run(parsed: &[&Parsed], out: &mut Vec<Diagnostic>) {
    for p in parsed {
        let aliases = std_imports(&p.program.items);
        let six = SourceIndex::new(p.file.clone(), &p.text);
        let mut v = Calls { aliases, shadowed: HashSet::new(), loc: Locator::new(&six), macro_locs: Vec::new(), out: &mut *out };
        v.items(&p.program.items);
    }
}

/// `use std::strings` -> `strings`, `use std::strings as s` -> `s`, mapped to the module.
pub(super) fn std_imports(items: &[Item<'_>]) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for item in items {
        let Item::Use { path, alias, .. } = item else { continue };
        if path.len() == 2 && path[0] == "std" && std_functions::module(&path[1]).is_some() {
            out.insert(alias.clone().unwrap_or_else(|| path[1].clone()), path[1].clone());
        }
    }
    out
}

struct Calls<'a, 'o> {
    aliases: HashMap<String, String>,
    /// Names bound in the current function (parameters, `let`, closure parameters): they hide a module alias.
    shadowed: HashSet<String>,
    loc: Locator<'a>,
    macro_locs: Vec<Option<Location>>,
    out: &'o mut Vec<Diagnostic>,
}

impl Calls<'_, '_> {
    fn items(&mut self, items: &[Item<'_>]) {
        for item in items {
            match item {
                Item::Function { decl, .. } => self.function(decl),
                Item::Impl { block, .. } => block.functions.iter().for_each(|f| self.function(f)),
                Item::Trait { decl, .. } => decl.methods.iter().filter_map(|m| m.body.as_ref()).for_each(|b| walk_block(self, b)),
                Item::Mod { items, .. } => self.items(items),
                Item::Const { value, .. } | Item::Static { value, .. } => walk_expr(self, value),
                _ => {}
            }
        }
    }

    fn function(&mut self, f: &FunctionDecl<'_>) {
        self.shadowed = f.parameters.iter().map(|p| p.name.clone()).collect();
        walk_block(self, &f.body);
        self.shadowed.clear();
    }

    /// The std module `written` (`strings`, `s`, `std::strings`) names, if any.
    fn module_of(&self, written: &str) -> Option<String> {
        match written.strip_prefix("std::") {
            Some(m) if !m.contains("::") => std_functions::module(m).map(|_| m.to_string()),
            _ if self.shadowed.contains(written) => None,
            _ => self.aliases.get(written).cloned(),
        }
    }

    fn check(&mut self, module: &str, func: &str, loc: &Option<Location>) {
        if std_functions::exists(module, func) != Some(false) {
            return;
        }
        let mut d = Diagnostic::error("WJ0002", format!("cannot find function `{func}` in `std::{module}`"));
        let loc = match (loc, self.macro_locs.last()) {
            (Some(l), Some(m)) if l.file.as_os_str().is_empty() => m.clone(),
            _ => loc.clone(),
        };
        if let Some(sp) = self.loc.word(&loc, func).or_else(|| self.loc.first_use(func)) {
            d = d.at(sp.with_label(format!("not a function of std::{module}")));
        }
        let fns = std_functions::module(module).unwrap_or_default();
        let best = fns
            .iter()
            .map(|f| (crate::fuzzy_matcher::levenshtein_distance(func, f), *f))
            .filter(|(dist, _)| *dist <= (func.len() / 3).clamp(1, 3))
            .min();
        if module == "test" {
            if let Some(new) = crate::stdlib_surface::removed_names::replacement(func) {
                if let Some(sp) = self.loc.word(&loc, func).or_else(|| self.loc.first_use(func)) {
                    d = d.with_fix(Fix {
                        description: format!("rename to `{new}`"),
                        applicability: Applicability::MachineApplicable,
                        edits: vec![Edit { span: Span { label: None, ..sp }, replacement: new.to_string() }],
                    });
                }
                self.out.push(d.with_help(format!("`{func}` was removed from `std::test`: use `{module}::{new}`")));
                return;
            }
        }
        self.out.push(match best {
            Some((_, f)) => d.with_help(format!("a function with a similar name exists: `{module}::{f}`")),
            None => d.with_help(format!("`wj docs {module}` lists the functions of std::{module}")),
        });
    }
}

impl Visitor for Calls<'_, '_> {
    fn expr(&mut self, e: &Expression<'_>) {
        match e {
            Expression::Call { function: Expression::Identifier { name, location }, .. } => {
                if let Some((head, func)) = name.rsplit_once("::") {
                    if let Some(m) = self.module_of(head) {
                        self.check(&m, func, location);
                    }
                }
            }
            Expression::MethodCall { object: Expression::Identifier { name, .. }, method, location, .. } => {
                if let Some(m) = self.module_of(name) {
                    self.check(&m, method, location);
                }
            }
            _ => {}
        }
    }

    fn pattern(&mut self, p: &Pattern<'_>) {
        if let Pattern::Identifier(n) = p {
            self.shadowed.insert(n.clone());
        }
    }

    fn closure(&mut self, params: &[String]) {
        self.shadowed.extend(params.iter().cloned());
    }

    fn macro_enter(&mut self, e: &Expression<'_>) {
        self.macro_locs.push(e.location());
    }

    fn macro_exit(&mut self) {
        self.macro_locs.pop();
    }
}
