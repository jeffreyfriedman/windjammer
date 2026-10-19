//! `type Name<P..> = Target` aliases: expanded wherever a type is resolved (`ty::from_ast`).
//!
//! The alias table is installed for the duration of one `lower_program` call (an RAII guard over a
//! thread-local), so every resolution site, including the item tables built before lowering, sees
//! the same aliases without threading a context through each `from_ast` call.

use super::ty::{from_ast, Ty};
use crate::parser::{Item, Program, Type};
use std::cell::RefCell;
use std::collections::HashMap;

#[derive(Clone)]
struct Alias {
    params: Vec<String>,
    target: Type,
}

thread_local! {
    static MODULES: RefCell<std::collections::HashSet<String>> = RefCell::new(Default::default());
    static ALIASES: RefCell<HashMap<String, Alias>> = RefCell::new(HashMap::new());
    static DEPTH: RefCell<usize> = const { RefCell::new(0) };
}

/// Removes the installed aliases when dropped.
pub struct AliasScope {
    previous: HashMap<String, Alias>,
    previous_modules: std::collections::HashSet<String>,
}

impl Drop for AliasScope {
    fn drop(&mut self) {
        let prev = std::mem::take(&mut self.previous);
        ALIASES.with(|a| *a.borrow_mut() = prev);
        let pm = std::mem::take(&mut self.previous_modules);
        MODULES.with(|m| *m.borrow_mut() = pm);
    }
}

/// Install the aliases declared in `program` (including nested modules).
pub fn install(program: &Program) -> AliasScope {
    let mut table = HashMap::new();
    collect(&program.items, &mut table);
    let previous = ALIASES.with(|a| std::mem::replace(&mut *a.borrow_mut(), table));
    let mut mods = std::collections::HashSet::new();
    collect_modules(&program.items, &mut mods);
    let previous_modules = MODULES.with(|m| std::mem::replace(&mut *m.borrow_mut(), mods));
    AliasScope { previous, previous_modules }
}

fn collect_modules(items: &[Item], out: &mut std::collections::HashSet<String>) {
    for item in items {
        if let Item::Mod { name, items, .. } = item {
            out.insert(name.clone());
            collect_modules(items, out);
        }
    }
}

pub fn is_module(name: &str) -> bool {
    MODULES.with(|m| m.borrow().contains(name))
}

/// `geo::inner::one` -> `one`, `shapes::Color::Red` -> `Color::Red`: the leading segments that
/// name modules of the program are dropped (portable modules are flat: items of all modules share
/// one namespace). `None` when no leading segment is a module.
pub fn strip_modules(path: &str) -> Option<String> {
    let segs: Vec<&str> = path.split("::").collect();
    let n = MODULES.with(|m| {
        let m = m.borrow();
        segs.iter().take(segs.len().saturating_sub(1)).take_while(|s| m.contains(**s) || matches!(**s, "self" | "super" | "crate")).count()
    });
    (n > 0).then(|| segs[n..].join("::"))
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

/// Expand `name<args>` if it is an alias. Cyclic aliases stop at a fixed depth (`Unknown`).
pub fn expand(name: &str, args: &[Ty], generics: &[String]) -> Option<Ty> {
    let alias = ALIASES.with(|a| a.borrow().get(name).cloned())?;
    let depth = DEPTH.with(|d| *d.borrow());
    if depth > 32 {
        return Some(Ty::Unknown);
    }
    DEPTH.with(|d| *d.borrow_mut() += 1);
    let mut scope: Vec<String> = generics.to_vec();
    scope.extend(alias.params.iter().cloned());
    let t = from_ast(&alias.target, &scope);
    DEPTH.with(|d| *d.borrow_mut() -= 1);
    Some(if alias.params.is_empty() { t } else { t.subst(&alias.params, args) })
}

/// The nominal type an alias names (`type A = P` -> `P`), for `A { .. }` and `A::new()`.
pub fn nominal(name: &str) -> Option<String> {
    match expand(name, &[], &[])? {
        Ty::Adt(n, _) => Some(n),
        _ => None,
    }
}
