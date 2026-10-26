//! `type Name<P..> = Target` aliases: expanded wherever a type is resolved (`ty::from_ast`).
//!
//! The table is built once per `lower_program` call, lives in the `ItemEnv` and is passed explicitly to every
//! resolution site (no global or thread-local state), so lowering is a pure function of its input program.

use super::ty::{from_ast, Ty};
use crate::parser::{Item, Program, Type};
use std::cell::Cell;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
struct Alias {
    params: Vec<String>,
    target: Type,
}

/// The aliases and module names declared in one program (including nested modules).
#[derive(Debug, Default)]
pub struct Aliases {
    table: HashMap<String, Alias>,
    modules: HashSet<String>,
    /// Expansion depth of the alias being resolved (cyclic aliases stop at a fixed depth).
    depth: Cell<usize>,
}

impl Aliases {
    pub fn from_program(program: &Program) -> Aliases {
        let mut a = Aliases::default();
        collect(&program.items, &mut a.table);
        collect_modules(&program.items, &mut a.modules);
        a
    }

    pub fn is_module(&self, name: &str) -> bool {
        self.modules.contains(name)
    }

    /// `geo::inner::one` -> `one`, `shapes::Color::Red` -> `Color::Red`: the leading segments that
    /// name modules of the program are dropped (portable modules are flat: items of all modules share
    /// one namespace). `None` when no leading segment is a module.
    pub fn strip_modules(&self, path: &str) -> Option<String> {
        let segs: Vec<&str> = path.split("::").collect();
        let n = segs
            .iter()
            .take(segs.len().saturating_sub(1))
            .take_while(|s| self.modules.contains(**s) || matches!(**s, "self" | "super" | "crate"))
            .count();
        (n > 0).then(|| segs[n..].join("::"))
    }

    /// Expand `name<args>` if it is an alias. Cyclic aliases stop at a fixed depth (`Unknown`).
    pub fn expand(&self, name: &str, args: &[Ty], generics: &[String]) -> Option<Ty> {
        let alias = self.table.get(name)?;
        if self.depth.get() > 32 {
            return Some(Ty::Unknown);
        }
        self.depth.set(self.depth.get() + 1);
        let mut scope: Vec<String> = generics.to_vec();
        scope.extend(alias.params.iter().cloned());
        let t = from_ast(&alias.target, &scope, self);
        self.depth.set(self.depth.get() - 1);
        Some(if alias.params.is_empty() { t } else { t.subst(&alias.params, args) })
    }

    /// The nominal type an alias names (`type A = P` -> `P`), for `A { .. }` and `A::new()`.
    pub fn nominal(&self, name: &str) -> Option<String> {
        match self.expand(name, &[], &[])? {
            Ty::Adt(n, _) => Some(n),
            _ => None,
        }
    }
}

fn collect_modules(items: &[Item], out: &mut HashSet<String>) {
    for item in items {
        if let Item::Mod { name, items, .. } = item {
            out.insert(name.clone());
            collect_modules(items, out);
        }
    }
}

fn collect(items: &[Item], out: &mut HashMap<String, Alias>) {
    for item in items {
        match item {
            Item::TypeAlias { name, target, .. } => {
                out.insert(name.clone(), Alias { params: alias_params(item), target: target.clone() });
            }
            Item::Mod { items, .. } => collect(items, out),
            _ => {}
        }
    }
}

/// The alias's own generic parameters (`type Pair<T> = (T, T)`).
fn alias_params(item: &Item) -> Vec<String> {
    match item {
        Item::TypeAlias { type_params, .. } => type_params.iter().map(|t| t.name.clone()).collect(),
        _ => Vec::new(),
    }
}
