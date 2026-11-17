//! A field moved out of a borrowed item (rustc E0507, reported as WJ0016), found in the Rust that move prediction
//! already generated for the file (`moves::predict`): no extra code generation.
//!
//! The interpreter copies values, so `items.values().map(|i| i.name)` runs interpreted; the compiled build fails when
//! the generated closure moves a non-`Copy` field out of the reference the iterator yields. Whether the generator adds
//! the `.clone()` depends on its ownership inference (it does for some bindings and not others), so the check reads its
//! decision instead of guessing it: a generated `.map(|x| x.field)` whose chain iterates by reference (`.iter()`,
//! `.values()`, `.keys()`, with no `.cloned()`/`.copied()`/`.into_iter()` in between) and whose field is non-`Copy` in
//! every struct of the file that declares it.

use crate::diagnostics::Diagnostic;
use crate::parser::{Item, Program, Type};

/// WJ0016 for each such closure of the generated `rust`, located in the file's `source`.
pub fn predict(program: &Program<'_>, rust: &str, source: &str, file: &str) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for (at, var, field) in field_maps(rust) {
        if !by_reference(&rust[..at]) || !non_copy_field(program, &field) {
            continue;
        }
        let needle = format!("{var}.{field}");
        let in_source = format!("|{var}| {needle}");
        let span = super::moves::span_of(source, file, &in_source, |rest| !rest.starts_with(['.', '('])).map(|mut sp| {
            // point at `var.field`, not the closure's parameter list
            let skip = in_source.len() - needle.len();
            sp.byte_start += skip;
            sp.col_start += skip;
            sp
        });
        let mut d = Diagnostic::error("WJ0016", format!("cannot move `{needle}` out of a borrow (the closure receives a reference to each item)"))
            .with_help(format!("copy it: `|{var}| {needle}.clone()`, or iterate by value"))
            .with_note("the interpreter would run this, the compiled program would not build (rustc E0507)");
        if let Some(sp) = span {
            d = d.at(sp.with_label("moved out of a borrowed item"));
        }
        if !out.iter().any(|o: &Diagnostic| o.message == d.message) {
            out.push(d);
        }
    }
    out
}

/// Every `.map(|x| x.field)` of `rust` whose body moves the field (not followed by `.` or `(`): (offset of `.map(`,
/// `x`, `field`).
fn field_maps(rust: &str) -> Vec<(usize, String, String)> {
    let ident = |s: &str| s.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect::<String>();
    let mut out = Vec::new();
    for (at, _) in rust.match_indices(".map(|") {
        let rest = &rust[at + ".map(|".len()..];
        let var = ident(rest);
        if var.is_empty() {
            continue;
        }
        let Some(body) = rest[var.len()..].strip_prefix("| ").and_then(|b| b.strip_prefix(var.as_str())).and_then(|b| b.strip_prefix('.')) else { continue };
        let field = ident(body);
        if field.is_empty() || field.starts_with(|c: char| c.is_ascii_digit()) {
            continue;
        }
        if body[field.len()..].starts_with(')') {
            out.push((at, var, field));
        }
    }
    out
}

/// Does the method chain that ends at the end of `before` iterate by reference? Reads back over the chain's own
/// calls (to the start of the statement) and decides by the nearest adapter that sets the item type.
fn by_reference(before: &str) -> bool {
    let start = before.rfind([';', '{', '\n']).map_or(0, |i| i + 1);
    let chain = &before[start..];
    let last = |m: &str| chain.rfind(m);
    let by_ref = [".iter()", ".values()", ".keys()"].iter().filter_map(|m| last(m)).max();
    let by_value = [".into_iter()", ".cloned()", ".copied()", ".into_values()", ".into_keys()", ".drain("].iter().filter_map(|m| last(m)).max();
    match (by_ref, by_value) {
        (Some(r), Some(v)) => r > v,
        (Some(_), None) => true,
        _ => false,
    }
}

/// `field` is declared by some struct of the program, and with a non-`Copy` type by every struct that declares it.
fn non_copy_field(program: &Program<'_>, field: &str) -> bool {
    let mut seen = false;
    for item in &program.items {
        if let Item::Struct { decl, .. } = item {
            for f in &decl.fields {
                if f.name == field {
                    if !owns_heap(&f.field_type) {
                        return false;
                    }
                    seen = true;
                }
            }
        }
    }
    seen
}

fn owns_heap(t: &Type) -> bool {
    match t {
        Type::String => true,
        Type::Parameterized(n, _) => matches!(n.as_str(), "Vec" | "HashMap" | "HashSet" | "BTreeMap" | "BTreeSet" | "VecDeque"),
        Type::Vec(_) => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_generated_field_move_through_a_reference_is_found_and_a_clone_or_by_value_chain_is_not() {
        let found = field_maps("let v: Vec<String> = m.values().filter(|i| i.p > 1).map(|i| i.name).collect();");
        assert_eq!(found.iter().map(|(_, v, f)| (v.as_str(), f.as_str())).collect::<Vec<_>>(), vec![("i", "name")]);
        assert!(field_maps("m.values().map(|i| i.name.clone()).collect()").is_empty());
        assert!(field_maps("m.values().map(|i| i.len())").is_empty());
        assert!(by_reference("let v = m.values().filter(|i| i.p > 1)"));
        assert!(!by_reference("let v = m.into_values()"));
        assert!(!by_reference("let v = xs.iter().cloned()"));
        assert!(!by_reference("let v = xs.into_iter()"));
        assert!(by_reference("let v = xs.into_iter().collect::<Vec<_>>().iter()"));
    }
}
