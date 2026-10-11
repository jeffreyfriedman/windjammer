//! Lint-clean idiom lowering: iterator adapters called directly on a collection.
//!
//! W0002 asks users to drop `.iter()`, so `v.map(f)` must compile. Rust only has
//! adapters on iterators, so we insert `.iter()` (borrowing, element type unchanged)
//! when the receiver's static type is a collection. Pure and type-driven.

use crate::parser::Type;

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
