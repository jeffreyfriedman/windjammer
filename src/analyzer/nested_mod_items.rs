//! Items of an inline module including those of nested inline modules (`pub mod geo { pub mod area { fn f() } }`),
//! so every nested function is analyzed and registered (it was silently dropped below the first level: E0425).
//! Each item carries the path of the module that contains it (`["geo", "area"]`): with the item's name that is
//! the identity the resolver gives it (`geo::area::f`), so same-name items of different modules stay distinct.

use crate::parser::Item;

/// Source order, nested modules expanded in place; each item with its containing module path
/// (starting with `module`).
pub(crate) fn flatten_with_paths<'a, 'ast>(module: &str, items: &'a [Item<'ast>]) -> Vec<(Vec<String>, &'a Item<'ast>)> {
    let mut out = Vec::new();
    collect(&mut vec![module.to_string()], items, &mut out);
    out
}

fn collect<'a, 'ast>(path: &mut Vec<String>, items: &'a [Item<'ast>], out: &mut Vec<(Vec<String>, &'a Item<'ast>)>) {
    for item in items {
        match item {
            Item::Mod { name, items: inner, .. } => {
                path.push(name.clone());
                collect(path, inner, out);
                path.pop();
            }
            other => out.push((path.clone(), other)),
        }
    }
}
