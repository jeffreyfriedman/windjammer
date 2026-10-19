//! Items of an inline module including those of nested inline modules (`pub mod geo { pub mod area { fn f() } }`),
//! so every nested function is analyzed and registered (it was silently dropped below the first level: E0425).

use crate::parser::Item;

/// Depth-first: the module's own non-module items, then (recursively) those of its nested modules.
pub(crate) fn flatten<'a, 'ast>(items: &'a [Item<'ast>]) -> Vec<&'a Item<'ast>> {
    let mut out = Vec::new();
    for item in items {
        match item {
            Item::Mod { items: inner, .. } => out.extend(flatten(inner)),
            other => out.push(other),
        }
    }
    out
}
