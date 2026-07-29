//! Specialize stdlib generic method signatures from the concrete receiver type.
//!
//! `Vec::push` is registered as `push(T)` with `Owned`. At a `Vec<String>` call site the
//! formal must become `String` so IR/`string_literal_needs_owned_coercion` emit
//! `.to_string()` for string literals (and leave `&str` formals bare).

use crate::analyzer::FunctionSignature;
use crate::parser::Type;

/// Substitute stdlib type parameters (`T`, `K`, `V`, `E`) in `sig` using `receiver_ty`.
///
/// No-op when the receiver is not a known collection or has no concrete type args.
pub fn specialize_signature_for_receiver(sig: &mut FunctionSignature, receiver_ty: &Type) {
    let Some(subst) = collection_generic_subst(receiver_ty) else {
        return;
    };
    if subst.is_empty() {
        return;
    }
    for ty in sig.param_types.iter_mut() {
        *ty = substitute_type(ty, &subst);
    }
    for ty in sig.formal_param_types.iter_mut() {
        *ty = substitute_type(ty, &subst);
    }
    if let Some(ret) = sig.return_type.as_mut() {
        *ret = substitute_type(ret, &subst);
    }
}

/// Build T/K/V/E substitutions from a concrete collection type.
fn collection_generic_subst(receiver_ty: &Type) -> Option<Vec<(String, Type)>> {
    let bare = peel_ref(receiver_ty);
    match bare {
        Type::Vec(elem) => Some(vec![
            ("T".into(), elem.as_ref().clone()),
            ("E".into(), elem.as_ref().clone()),
        ]),
        Type::Parameterized(name, args) => {
            let base = name.split('<').next().unwrap_or(name.as_str());
            match base {
                "Vec" | "VecDeque" | "LinkedList" | "HashSet" | "BTreeSet" | "Set" => {
                    let elem = args.first()?.clone();
                    Some(vec![("T".into(), elem.clone()), ("E".into(), elem)])
                }
                "HashMap" | "BTreeMap" | "IndexMap" | "Map" | "OrderedMap" | "SlotMap"
                | "ConcurrentMap" => {
                    let key = args.first()?.clone();
                    let val = args.get(1).cloned().unwrap_or_else(|| Type::Custom("V".into()));
                    Some(vec![
                        ("K".into(), key),
                        ("V".into(), val),
                        ("T".into(), args.first()?.clone()),
                    ])
                }
                _ => None,
            }
        }
        Type::Custom(name) => {
            // `Vec<String>` may appear as a single Custom name from type_to_name.
            parse_angled_type_args(name).and_then(|(base, args)| {
                collection_generic_subst(&Type::Parameterized(base, args))
            })
        }
        _ => None,
    }
}

fn peel_ref(ty: &Type) -> &Type {
    match ty {
        Type::Reference(inner) | Type::MutableReference(inner) => peel_ref(inner),
        other => other,
    }
}

fn parse_angled_type_args(name: &str) -> Option<(String, Vec<Type>)> {
    let open = name.find('<')?;
    let close = name.rfind('>')?;
    if close <= open {
        return None;
    }
    let base = name[..open].to_string();
    let inner = &name[open + 1..close];
    let args = split_top_level_commas(inner)
        .into_iter()
        .map(|s| parse_simple_type(s.trim()))
        .collect::<Vec<_>>();
    if args.is_empty() {
        return None;
    }
    Some((base, args))
}

fn split_top_level_commas(s: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0usize;
    let mut start = 0usize;
    for (i, ch) in s.char_indices() {
        match ch {
            '<' | '(' | '[' => depth += 1,
            '>' | ')' | ']' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                parts.push(&s[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    if start < s.len() {
        parts.push(&s[start..]);
    }
    parts
}

fn parse_simple_type(s: &str) -> Type {
    if let Some((base, args)) = parse_angled_type_args(s) {
        return Type::Parameterized(base, args);
    }
    match s {
        "string" | "String" => Type::String,
        "str" => Type::Custom("str".into()),
        "int" | "i64" => Type::Int,
        "i32" => Type::Custom("i32".into()),
        "uint" | "u64" | "usize" => Type::Uint,
        "bool" => Type::Bool,
        "f32" => Type::Custom("f32".into()),
        "f64" | "float" => Type::Float,
        other => Type::Custom(other.to_string()),
    }
}

fn substitute_type(ty: &Type, subst: &[(String, Type)]) -> Type {
    match ty {
        Type::Custom(name) => {
            if let Some((_, replacement)) = subst.iter().find(|(k, _)| k == name) {
                replacement.clone()
            } else if let Some((base, args)) = parse_angled_type_args(name) {
                Type::Parameterized(
                    base,
                    args.into_iter()
                        .map(|a| substitute_type(&a, subst))
                        .collect(),
                )
            } else {
                ty.clone()
            }
        }
        Type::Reference(inner) => Type::Reference(Box::new(substitute_type(inner, subst))),
        Type::MutableReference(inner) => {
            Type::MutableReference(Box::new(substitute_type(inner, subst)))
        }
        Type::Vec(inner) => Type::Vec(Box::new(substitute_type(inner, subst))),
        Type::Option(inner) => Type::Option(Box::new(substitute_type(inner, subst))),
        Type::Result(ok, err) => Type::Result(
            Box::new(substitute_type(ok, subst)),
            Box::new(substitute_type(err, subst)),
        ),
        Type::Parameterized(name, args) => Type::Parameterized(
            name.clone(),
            args.iter().map(|a| substitute_type(a, subst)).collect(),
        ),
        other => other.clone(),
    }
}

/// Infer a concrete receiver `Type` for specialization when only a type name is known.
pub fn receiver_type_from_name_and_hint(
    receiver_type_name: Option<&str>,
    inferred: Option<&Type>,
    return_type_hint: Option<&Type>,
) -> Option<Type> {
    if let Some(ty) = inferred {
        if collection_generic_subst(ty).is_some() {
            return Some(ty.clone());
        }
    }
    if let Some(name) = receiver_type_name {
        if let Some((base, args)) = parse_angled_type_args(name) {
            return Some(Type::Parameterized(base, args));
        }
        // Unparameterized `Vec` / `HashMap`: recover args from function return type.
        if let Some(ret) = return_type_hint {
            if let Some(subst_src) = match (name, peel_ref(ret)) {
                ("Vec", Type::Vec(_)) => Some(ret.clone()),
                ("Vec", Type::Parameterized(n, _)) if n == "Vec" => Some(ret.clone()),
                (map, Type::Parameterized(n, _))
                    if map == n
                        || (matches!(
                            map,
                            "HashMap" | "Map" | "BTreeMap" | "IndexMap" | "OrderedMap"
                        ) && matches!(
                            n.as_str(),
                            "HashMap" | "Map" | "BTreeMap" | "IndexMap" | "OrderedMap"
                        )) =>
                {
                    Some(ret.clone())
                }
                _ => None,
            } {
                return Some(subst_src);
            }
        }
        // Bare name only — insufficient for substitution.
        return Some(Type::Custom(name.to_string()));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::OwnershipMode;
    use crate::codegen::rust::method_signature::MethodSignature;
    use crate::codegen::rust::string_utilities::string_literal_needs_owned_coercion_with_enum;

    fn vec_push_sig() -> FunctionSignature {
        MethodSignature::new(
            "Vec",
            "push",
            vec![Type::Custom("T".into())],
            vec![OwnershipMode::Owned],
            None,
            true,
        )
        .to_function_signature()
    }

    #[test]
    fn specialize_vec_string_push_makes_owned_string_formal() {
        let mut sig = vec_push_sig();
        specialize_signature_for_receiver(
            &mut sig,
            &Type::Vec(Box::new(Type::String)),
        );
        let idx = sig.arg_param_index(0);
        assert!(
            matches!(sig.param_types.get(idx), Some(Type::String)),
            "T should become String, got {:?}",
            sig.param_types.get(idx)
        );
        assert!(
            string_literal_needs_owned_coercion_with_enum(
                Some(&sig),
                0,
                Some("push"),
                Some("Vec"),
                None,
                None,
            ),
            "specialized push must coerce string literals to owned String"
        );
    }

    #[test]
    fn specialize_hashmap_string_key_insert_value_owned() {
        let mut sig = MethodSignature::new(
            "HashMap",
            "insert",
            vec![Type::Custom("K".into()), Type::Custom("V".into())],
            vec![OwnershipMode::Owned, OwnershipMode::Owned],
            None,
            true,
        )
        .to_function_signature();
        specialize_signature_for_receiver(
            &mut sig,
            &Type::Parameterized(
                "HashMap".into(),
                vec![Type::String, Type::Int],
            ),
        );
        let key_idx = sig.arg_param_index(0);
        let val_idx = sig.arg_param_index(1);
        assert!(matches!(sig.param_types.get(key_idx), Some(Type::String)));
        assert!(matches!(sig.param_types.get(val_idx), Some(Type::Int)));
        assert!(string_literal_needs_owned_coercion_with_enum(
            Some(&sig),
            0,
            Some("insert"),
            Some("HashMap"),
            None,
            None,
        ));
    }

    #[test]
    fn unspecialized_generic_t_does_not_force_to_string() {
        let sig = vec_push_sig();
        // Without specialization, Custom("T") must not look like owned String —
        // this documents the pre-fix failure mode that finalize stripped .to_string().
        assert!(
            !crate::codegen::rust::string_utilities::param_is_owned_string_type(
                sig.param_types.get(sig.arg_param_index(0)).unwrap()
            )
        );
    }

    #[test]
    fn receiver_hint_from_return_type_vec_string() {
        let hint = receiver_type_from_name_and_hint(
            Some("Vec"),
            None,
            Some(&Type::Vec(Box::new(Type::String))),
        );
        assert!(matches!(hint, Some(Type::Vec(_))));
        let mut sig = vec_push_sig();
        specialize_signature_for_receiver(&mut sig, &hint.unwrap());
        assert!(matches!(
            sig.param_types.get(sig.arg_param_index(0)),
            Some(Type::String)
        ));
    }
}
