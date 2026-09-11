//! Break infinite-size recursive struct layouts by inserting `Box<T>` (WDB-116).
//!
//! Rust requires every recursive owned field cycle to go through a pointer-sized
//! indirection. Windjammer source writes idiomatic owned embeds; the compiler
//! inserts `Box` on a deterministic edge per strongly connected component.

use crate::parser::Type;
use std::collections::{HashMap, HashSet};

/// Mutate `struct_fields` in place: wrap one owned field per recursive SCC in `Box<_>`.
pub fn apply_recursive_struct_boxing(
    struct_fields: &mut HashMap<String, HashMap<String, Type>>,
) {
    if struct_fields.len() < 1 {
        return;
    }

    let keys: Vec<String> = struct_fields.keys().cloned().collect();
    let key_set: HashSet<String> = keys.iter().cloned().collect();
    let mut by_simple: HashMap<String, Vec<String>> = HashMap::new();
    for k in &keys {
        let simple = simple_struct_name(k).to_string();
        by_simple.entry(simple).or_default().push(k.clone());
    }

    // Edges: (from_key, field_name, to_key) for bare owned Custom field types.
    let mut edges: Vec<(String, String, String)> = Vec::new();
    for from_key in &keys {
        let Some(fields) = struct_fields.get(from_key) else {
            continue;
        };
        for (field_name, field_ty) in fields {
            for target_name in owned_struct_type_names(field_ty) {
                if let Some(to_key) =
                    resolve_struct_key(&target_name, from_key, &key_set, &by_simple)
                {
                    edges.push((from_key.clone(), field_name.clone(), to_key));
                }
            }
        }
    }

    if edges.is_empty() {
        return;
    }

    // Adjacency for Tarjan (node → neighbors).
    let mut adj: HashMap<String, Vec<String>> = HashMap::new();
    for (from, _, to) in &edges {
        adj.entry(from.clone()).or_default().push(to.clone());
        adj.entry(to.clone()).or_default();
    }
    for k in &keys {
        adj.entry(k.clone()).or_default();
    }

    let sccs = tarjan_sccs(&adj);
    let mut to_box: Vec<(String, String)> = Vec::new();

    for scc in sccs {
        if scc.is_empty() {
            continue;
        }
        let scc_set: HashSet<&str> = scc.iter().map(|s| s.as_str()).collect();
        let is_self_loop = scc.len() == 1
            && edges.iter().any(|(f, _, t)| f == t && scc_set.contains(f.as_str()));
        let is_multi = scc.len() > 1;
        if !is_self_loop && !is_multi {
            continue;
        }

        // Collect internal edges; pick lexicographically smallest (key, field).
        let mut internal: Vec<(String, String)> = edges
            .iter()
            .filter(|(f, _, t)| scc_set.contains(f.as_str()) && scc_set.contains(t.as_str()))
            .map(|(f, field, _)| (f.clone(), field.clone()))
            .collect();
        internal.sort();
        internal.dedup();
        if let Some((key, field)) = internal.into_iter().next() {
            to_box.push((key, field));
        }
    }

    for (key, field) in to_box {
        box_field_on_key_and_aliases(struct_fields, &key, &field, &by_simple);
    }
}

fn simple_struct_name(key: &str) -> &str {
    key.rsplit("::").next().unwrap_or(key)
}

fn resolve_struct_key(
    name: &str,
    from_key: &str,
    key_set: &HashSet<String>,
    by_simple: &HashMap<String, Vec<String>>,
) -> Option<String> {
    if key_set.contains(name) {
        return Some(name.to_string());
    }
    let simple = simple_struct_name(name);
    let candidates = by_simple.get(simple)?;
    if candidates.len() == 1 {
        return Some(candidates[0].clone());
    }
    let from_mod = module_prefix(from_key);
    candidates
        .iter()
        .find(|c| module_prefix(c) == from_mod)
        .cloned()
        .or_else(|| candidates.first().cloned())
}

fn module_prefix(key: &str) -> &str {
    match key.rfind("::") {
        Some(i) => &key[..i],
        None => "",
    }
}

/// Direct owned struct embeds only — `Box`/`Vec`/`Option`/`&` already have finite size.
fn owned_struct_type_names(ty: &Type) -> Vec<String> {
    match ty {
        Type::Custom(name) => {
            let base = name.split('<').next().unwrap_or(name).trim();
            if base.is_empty()
                || crate::type_classification::is_prelude_or_primitive(base)
                || matches!(
                    base,
                    "string" | "String" | "str" | "usize" | "isize" | "char"
                )
            {
                Vec::new()
            } else {
                vec![base.to_string()]
            }
        }
        Type::Tuple(items) => items.iter().flat_map(owned_struct_type_names).collect(),
        // Already indirect / not an infinite-size edge:
        Type::Parameterized(base, _) if base == "Box" || base == "Arc" || base == "Rc" => {
            Vec::new()
        }
        Type::Vec(_)
        | Type::Option(_)
        | Type::Reference(_)
        | Type::MutableReference(_)
        | Type::Array(_, _) => Vec::new(),
        _ => Vec::new(),
    }
}

fn is_box_type(ty: &Type) -> bool {
    matches!(ty, Type::Parameterized(base, args) if base == "Box" && args.len() == 1)
}

fn box_field_on_key_and_aliases(
    struct_fields: &mut HashMap<String, HashMap<String, Type>>,
    key: &str,
    field: &str,
    by_simple: &HashMap<String, Vec<String>>,
) {
    let simple = simple_struct_name(key).to_string();
    let mut keys_to_update = vec![key.to_string()];
    if let Some(aliases) = by_simple.get(&simple) {
        for a in aliases {
            if a != key {
                keys_to_update.push(a.clone());
            }
        }
    }
    for k in keys_to_update {
        if let Some(fields) = struct_fields.get_mut(&k) {
            if let Some(ty) = fields.get_mut(field) {
                if !is_box_type(ty) {
                    let inner = ty.clone();
                    *ty = Type::Parameterized("Box".into(), vec![inner]);
                }
            }
        }
    }
}

/// Classic Tarjan SCC. Returns components in discovery order.
fn tarjan_sccs(adj: &HashMap<String, Vec<String>>) -> Vec<Vec<String>> {
    let mut index = 0usize;
    let mut stack: Vec<String> = Vec::new();
    let mut on_stack: HashSet<String> = HashSet::new();
    let mut indices: HashMap<String, usize> = HashMap::new();
    let mut lowlink: HashMap<String, usize> = HashMap::new();
    let mut sccs: Vec<Vec<String>> = Vec::new();

    fn strongconnect(
        v: &str,
        adj: &HashMap<String, Vec<String>>,
        index: &mut usize,
        stack: &mut Vec<String>,
        on_stack: &mut HashSet<String>,
        indices: &mut HashMap<String, usize>,
        lowlink: &mut HashMap<String, usize>,
        sccs: &mut Vec<Vec<String>>,
    ) {
        indices.insert(v.to_string(), *index);
        lowlink.insert(v.to_string(), *index);
        *index += 1;
        stack.push(v.to_string());
        on_stack.insert(v.to_string());

        if let Some(neighbors) = adj.get(v) {
            for w in neighbors {
                if !indices.contains_key(w) {
                    strongconnect(w, adj, index, stack, on_stack, indices, lowlink, sccs);
                    let lw = *lowlink.get(w).unwrap_or(&usize::MAX);
                    let lv = lowlink.get_mut(v).unwrap();
                    *lv = (*lv).min(lw);
                } else if on_stack.contains(w) {
                    let iw = *indices.get(w).unwrap_or(&usize::MAX);
                    let lv = lowlink.get_mut(v).unwrap();
                    *lv = (*lv).min(iw);
                }
            }
        }

        if lowlink.get(v) == indices.get(v) {
            let mut comp = Vec::new();
            while let Some(w) = stack.pop() {
                on_stack.remove(&w);
                let done = w == v;
                comp.push(w);
                if done {
                    break;
                }
            }
            sccs.push(comp);
        }
    }

    let mut nodes: Vec<String> = adj.keys().cloned().collect();
    nodes.sort();
    for v in nodes {
        if !indices.contains_key(&v) {
            strongconnect(
                &v,
                adj,
                &mut index,
                &mut stack,
                &mut on_stack,
                &mut indices,
                &mut lowlink,
                &mut sccs,
            );
        }
    }
    sccs
}

/// True when `ty` is `Box<_>` (effective layout after boxing pass).
pub fn type_is_box(ty: &Type) -> bool {
    is_box_type(ty)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boxes_one_edge_in_two_node_cycle() {
        let mut map = HashMap::new();
        map.insert(
            "layer_a::LayerA".into(),
            HashMap::from([("child".into(), Type::Custom("LayerB".into()))]),
        );
        map.insert(
            "layer_b::LayerB".into(),
            HashMap::from([("child".into(), Type::Custom("LayerA".into()))]),
        );
        apply_recursive_struct_boxing(&mut map);
        let a_boxed = type_is_box(
            map.get("layer_a::LayerA")
                .and_then(|f| f.get("child"))
                .unwrap(),
        );
        let b_boxed = type_is_box(
            map.get("layer_b::LayerB")
                .and_then(|f| f.get("child"))
                .unwrap(),
        );
        assert!(
            a_boxed ^ b_boxed,
            "exactly one edge in the cycle should be boxed; a={a_boxed} b={b_boxed}"
        );
    }

    #[test]
    fn boxes_self_recursive_field() {
        let mut map = HashMap::new();
        map.insert(
            "Node".into(),
            HashMap::from([("next".into(), Type::Custom("Node".into()))]),
        );
        apply_recursive_struct_boxing(&mut map);
        assert!(type_is_box(
            map.get("Node").and_then(|f| f.get("next")).unwrap()
        ));
    }

    #[test]
    fn leaves_vec_indirection_alone() {
        let mut map = HashMap::new();
        map.insert(
            "A".into(),
            HashMap::from([("bs".into(), Type::Vec(Box::new(Type::Custom("B".into()))))]),
        );
        map.insert(
            "B".into(),
            HashMap::from([("a".into(), Type::Custom("A".into()))]),
        );
        apply_recursive_struct_boxing(&mut map);
        // A.bs is Vec — finite; B.a → A → Vec<B> is also finite. No boxing required.
        assert!(!type_is_box(
            map.get("B").and_then(|f| f.get("a")).unwrap()
        ));
        assert!(!type_is_box(
            map.get("A").and_then(|f| f.get("bs")).unwrap()
        ));
    }
}
