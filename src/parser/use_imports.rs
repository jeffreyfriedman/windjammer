//! The names a file brings into scope with `use`, by declaration: `use crate::m::f` and
//! `use crate::m::{A, f}` make the bare name `f` stand for the path `m::f` (C3 identity: a bare
//! call to an imported name calls that definition, whatever else shares its name).

use std::collections::HashMap;

use super::ast::core::Item;

/// Bare name -> the module path it names, with a leading `crate::` dropped (`list_items` ->
/// `usecases::list_items`). Aliased imports (`use m::f as g`) map `g`. Glob imports add nothing.
pub fn imported_paths(items: &[Item<'_>]) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for item in items {
        let Item::Use { path, alias, .. } = item else { continue };
        let joined = path.join("::");
        let joined = joined.strip_prefix("crate::").unwrap_or(&joined);
        match joined.split_once("::{") {
            Some((module, rest)) => {
                for name in rest.trim_end_matches('}').split(',').map(str::trim).filter(|n| !n.is_empty() && *n != "*") {
                    out.insert(name.to_string(), format!("{module}::{name}"));
                }
            }
            None => {
                let Some((_, last)) = joined.rsplit_once("::") else { continue };
                if last != "*" {
                    out.insert(alias.clone().unwrap_or_else(|| last.to_string()), joined.to_string());
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paths(src: &str) -> HashMap<String, String> {
        let mut lexer = crate::lexer::Lexer::new(src);
        let parser = Box::leak(Box::new(crate::parser::Parser::new(lexer.tokenize_with_locations())));
        let program = Box::leak(Box::new(parser.parse().expect("parse")));
        imported_paths(&program.items)
    }

    #[test]
    fn plain_braced_and_aliased_imports_name_their_paths() {
        let p = paths("use crate::ops::double\nuse crate::usecases::{Deps, list_items}\nuse crate::m::f as g\nfn main() {}\n");
        assert_eq!(p.get("double").map(String::as_str), Some("ops::double"));
        assert_eq!(p.get("list_items").map(String::as_str), Some("usecases::list_items"));
        assert_eq!(p.get("Deps").map(String::as_str), Some("usecases::Deps"));
        assert_eq!(p.get("g").map(String::as_str), Some("m::f"));
        assert!(!p.contains_key("f"));
    }
}
