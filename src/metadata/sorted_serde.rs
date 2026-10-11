//! Deterministic serialization of `HashMap` fields in `.wj.meta` files.
//!
//! `HashMap` iteration order is randomized per process, which made identical builds
//! write different `.wj-cache/*.wj.meta` bytes (and defeated byte-wise cache checks).

use serde::{Serialize, Serializer};
use std::collections::{BTreeMap, HashMap};

pub fn sorted_map<S, V>(map: &HashMap<String, V>, ser: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    V: Serialize,
{
    map.iter().collect::<BTreeMap<_, _>>().serialize(ser)
}

pub fn sorted_nested_map<S>(
    map: &HashMap<String, HashMap<String, String>>,
    ser: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    map.iter()
        .map(|(k, v)| (k, v.iter().collect::<BTreeMap<_, _>>()))
        .collect::<BTreeMap<_, _>>()
        .serialize(ser)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize)]
    struct S {
        #[serde(serialize_with = "sorted_nested_map")]
        m: HashMap<String, HashMap<String, String>>,
        #[serde(serialize_with = "sorted_map")]
        f: HashMap<String, Vec<String>>,
    }

    #[test]
    fn output_is_independent_of_insertion_order() {
        let build = |rev: bool| {
            let mut keys: Vec<String> = (0..40).map(|i| format!("k{i}")).collect();
            if rev {
                keys.reverse();
            }
            let mut s = S {
                m: HashMap::new(),
                f: HashMap::new(),
            };
            for k in keys {
                let inner = (0..10)
                    .map(|j| (format!("f{j}"), "T".to_string()))
                    .collect();
                s.m.insert(k.clone(), inner);
                s.f.insert(k, vec![]);
            }
            serde_json::to_string(&s).unwrap()
        };
        assert_eq!(build(false), build(true));
        assert!(build(false).starts_with("{\"m\":{\"k0\":"));
    }
}
