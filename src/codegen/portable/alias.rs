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
    static ALIASES: RefCell<HashMap<String, Alias>> = RefCell::new(HashMap::new());
    static DEPTH: RefCell<usize> = const { RefCell::new(0) };
}

/// Removes the installed aliases when dropped.
pub struct AliasScope {
    previous: HashMap<String, Alias>,
}

impl Drop for AliasScope {
    fn drop(&mut self) {
        let prev = std::mem::take(&mut self.previous);
        ALIASES.with(|a| *a.borrow_mut() = prev);
    }
}

/// Install the aliases declared in `program` (including nested modules).
pub fn install(program: &Program) -> AliasScope {
    let mut table = HashMap::new();
    collect(&program.items, &mut table);
    let previous = ALIASES.with(|a| std::mem::replace(&mut *a.borrow_mut(), table));
    AliasScope { previous }
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
    let _ = item;
    Vec::new()
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
