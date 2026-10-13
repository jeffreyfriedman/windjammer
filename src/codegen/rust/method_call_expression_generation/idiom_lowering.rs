//! Lint-clean idiom lowering at method-call sites: iterator adapters called on collections (`.iter()`),
//! `Option<&T>.unwrap_or(default)`, `cmp(&x)` borrows and `usize` slice bounds.
//!
//! W0002 asks users to drop `.iter()`, so `v.map(f)` must compile. Rust only has
//! adapters on iterators, so we insert `.iter()` (borrowing, element type unchanged)
//! when the receiver's static type is a collection. Pure and type-driven.

use crate::codegen::rust::CodeGenerator;
use crate::parser::{Expression, Type};

/// Adapters that exist on `Iterator` but not on the collection types.
const ITERATOR_ONLY: &[&str] = &[
    "map", "filter", "filter_map", "flat_map", "flatten", "fold", "any", "all", "find", "find_map",
    "position", "take", "skip", "take_while", "skip_while", "step_by", "zip", "chain", "rev",
    "sum", "product", "max_by_key", "min_by_key", "max_by", "min_by", "partition", "cloned",
    "copied", "peekable", "inspect", "for_each", "count", "last", "unzip", "scan", "cycle",
];

/// Collections whose `.iter()` yields the adapters' input.
fn is_collection(ty: &Type) -> bool {
    match ty {
        Type::Vec(_) | Type::Array(..) => true,
        Type::Reference(t) | Type::MutableReference(t) => is_collection(t),
        Type::Parameterized(n, _) => matches!(
            n.as_str(),
            "Vec" | "VecDeque" | "HashSet" | "BTreeSet" | "HashMap" | "BTreeMap" | "BinaryHeap"
        ),
        _ => false,
    }
}

/// Whether `recv.method(..)` needs `.iter()` inserted after `recv`.
pub(super) fn needs_iter_prefix(recv_ty: Option<&Type>, method: &str, recv_text: &str) -> bool {
    needs_iter_prefix_for(recv_ty, method, recv_text, false)
}

/// `recv_is_adapter_call`: the receiver is itself an iterator-adapter call (`xs.filter(..)`), whose
/// result is an iterator even if the type inferer reports the collection type.
pub(super) fn needs_iter_prefix_for(
    recv_ty: Option<&Type>,
    method: &str,
    recv_text: &str,
    recv_is_adapter_call: bool,
) -> bool {
    if recv_is_adapter_call {
        return false;
    }
    if !ITERATOR_ONLY.contains(&method) {
        return false;
    }
    // `count`/`last`/`rev`... on an already-lowered iterator chain stay untouched.
    let already = recv_text.ends_with(".iter()")
        || recv_text.ends_with(".iter_mut()")
        || recv_text.ends_with(".into_iter()");
    !already && recv_ty.is_some_and(is_collection)
}

/// Drop a defaulted `_i32`/`_i64` suffix from a bare integer literal (slice bounds are `usize`).
pub(super) fn usize_bound(b: &str) -> String {
    for suffix in ["_i32", "_i64"] {
        if let Some(d) = b.strip_suffix(suffix) {
            if !d.is_empty() && d.chars().all(|c| c.is_ascii_digit()) {
                return d.to_string();
            }
        }
    }
    b.to_string()
}

impl<'ast> CodeGenerator<'ast> {
    /// Does the receiver's (user) type define `method`? Then the call is not the stdlib one of that name.
    pub(super) fn receiver_defines_method(&self, receiver_type: Option<&str>, method: &str) -> bool {
        receiver_type.is_some_and(|rt| self.method_exists_on_type_name(rt, method))
    }

    /// W0003-clean `v.map(f)`: insert `.iter()` when an iterator adapter is called on a collection-typed receiver.
    /// Method-call receivers (`m.keys()`, `s.split(..)`, `xs.filter(..)`) are already iterators or views whose
    /// inferred type is unreliable; only `clone`/`to_vec`/`collect` yield owned collections.
    pub(super) fn prefix_iter_for_collection_adapter(
        &self,
        object: &Expression<'ast>,
        method: &str,
        recv_ty: Option<&Type>,
        obj_str: &mut String,
    ) {
        let recv_is_adapter_call = matches!(
            object,
            Expression::MethodCall { method: m, .. } if !matches!(m.as_str(), "clone" | "to_vec" | "collect")
        );
        if needs_iter_prefix_for(recv_ty, method, obj_str, recv_is_adapter_call) {
            obj_str.push_str(".iter()");
        }
    }

    /// `Option<&T>.unwrap_or(default)`: the default has the owned element type. Lookups yield `Option<&T>`; a Copy `T`
    /// is copied out (`.copied()`) with the literal at `T`'s width, and a string-literal default stays `&str` for
    /// `Option<&String>`/`Option<&str>`. Also: `Ord::cmp`/`partial_cmp` take `&Self`, so the borrow is never stripped.
    pub(super) fn lower_option_default_and_cmp(
        &self,
        object: &Expression<'ast>,
        method: &str,
        recv_ty: Option<&Type>,
        obj_str: &mut String,
        args: &mut [String],
    ) {
        if matches!(method, "cmp" | "partial_cmp") && args.len() == 1 && !args[0].starts_with('&') {
            args[0] = format!("&{}", args[0]);
        }
        if method != "unwrap_or" || args.len() != 1 {
            return;
        }
        // `it.next()` on a synthetic `Vec<&T>` iterator type is `Option<&T>`.
        let derived = match object {
            Expression::MethodCall { object: it, method: m, .. }
                if matches!(m.as_str(), "next" | "last" | "nth" | "next_back") =>
            {
                match self.infer_expression_type(it) {
                    Some(Type::Vec(item)) => Some(Type::Option(item)),
                    _ => None,
                }
            }
            _ => None,
        };
        let Some(Type::Option(inner)) = recv_ty.or(derived.as_ref()) else { return };
        let Type::Reference(pointee) = inner.as_ref() else { return };
        let is_text = matches!(pointee.as_ref(), Type::String)
            || matches!(pointee.as_ref(), Type::Custom(n) if n == "String" || n == "str" || n == "string");
        if is_text {
            let lit = args[0].strip_prefix("String::from(").and_then(|r| r.strip_suffix(')'));
            if let Some(lit) = lit.filter(|l| l.starts_with('"')) {
                args[0] = lit.to_string();
            }
        } else if self.is_type_copy(pointee) {
            // `unwrap_or(&0)` keeps the `Option<&T>` (reference default); an already copied/cloned
            // receiver is `Option<T>` text even when the type query still reports the source `Option<&T>`.
            let by_value_default = !args[0].starts_with('&');
            if !by_value_default || obj_str.ends_with(".copied()") || obj_str.ends_with(".cloned()") {
                return;
            }
            obj_str.push_str(".copied()");
            let width = match pointee.as_ref() {
                Type::Custom(w) | Type::Parameterized(w, _) => Some(w.as_str()),
                Type::Int => Some("i64"),
                _ => None,
            };
            for suffix in ["_i32", "_i64"] {
                if let (Some(w), Some(d)) = (width, args[0].strip_suffix(suffix)) {
                    if !d.trim_start_matches('-').is_empty() && d.trim_start_matches('-').chars().all(|c| c.is_ascii_digit()) {
                        args[0] = format!("{d}_{w}");
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn v() -> Type {
        Type::Vec(Box::new(Type::Custom("i32".into())))
    }
    #[test]
    fn vec_map_needs_iter() {
        assert!(needs_iter_prefix(Some(&v()), "map", "xs"));
        assert!(needs_iter_prefix(Some(&Type::Reference(Box::new(v()))), "filter", "xs"));
    }
    #[test]
    fn non_adapters_and_iterators_untouched() {
        assert!(!needs_iter_prefix(Some(&v()), "push", "xs"));
        assert!(!needs_iter_prefix(Some(&v()), "map", "xs.iter()"));
        assert!(!needs_iter_prefix(Some(&Type::Option(Box::new(v()))), "map", "o"));
        assert!(!needs_iter_prefix(None, "map", "xs"));
    }
}
