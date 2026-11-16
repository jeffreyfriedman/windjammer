//! Enum variant payload types by `Enum::Variant` key, indexed by enum path and by the enum's last path
//! segment, so "is this a unit-only enum" asks about the matching variants only instead of scanning every
//! variant of the library (that scan ran per type check and made codegen quadratic in library size).

use crate::parser::Type;
use std::collections::HashMap;

/// `Enum::Variant` (or `path::Enum::Variant`) -> payload types, with lookup indexes.
#[derive(Debug, Clone, Default)]
pub struct EnumVariantTable {
    map: HashMap<String, Vec<Type>>,
    /// Enum path of the key (`path::Enum`) -> keys.
    by_enum: HashMap<String, Vec<String>>,
    /// Last segment of the enum path (`Enum`) -> keys.
    by_base: HashMap<String, Vec<String>>,
}

/// (`path::Enum`, `Enum`) of a variant key `path::Enum::Variant`; a key without `::` is its own enum.
fn enum_parts(key: &str) -> (&str, &str) {
    let key_enum = key.rsplit_once("::").map(|(e, _)| e).unwrap_or(key);
    let key_base = key_enum.rsplit("::").next().unwrap_or(key_enum);
    (key_enum, key_base)
}

impl EnumVariantTable {
    /// Insert or replace the payload of a variant key.
    pub fn insert(&mut self, key: String, payload: Vec<Type>) {
        if !self.map.contains_key(&key) {
            let (key_enum, key_base) = enum_parts(&key);
            self.by_enum.entry(key_enum.to_string()).or_default().push(key.clone());
            self.by_base.entry(key_base.to_string()).or_default().push(key.clone());
        }
        self.map.insert(key, payload);
    }

    /// Insert or replace every entry of `entries`.
    pub fn extend(&mut self, entries: impl IntoIterator<Item = (String, Vec<Type>)>) {
        for (k, v) in entries {
            self.insert(k, v);
        }
    }

    /// The payload table.
    pub fn as_map(&self) -> &HashMap<String, Vec<Type>> {
        &self.map
    }

    /// Every variant whose enum path is `enum_name` or `name`, or whose enum's last segment is `base`:
    /// `Some(true)` when all of them are unit variants, `Some(false)` when one has a payload, `None` when
    /// there is none.
    pub fn unit_only(&self, name: &str, enum_name: &str, base: &str) -> Option<bool> {
        let mut saw = false;
        let candidates = [self.by_base.get(base), self.by_enum.get(enum_name), self.by_enum.get(name)];
        for key in candidates.into_iter().flatten().flatten() {
            saw = true;
            if self.map.get(key).is_some_and(|p| !p.is_empty()) {
                return Some(false);
            }
        }
        saw.then_some(true)
    }
}

impl std::ops::Deref for EnumVariantTable {
    type Target = HashMap<String, Vec<Type>>;
    fn deref(&self) -> &Self::Target {
        &self.map
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The indexed answer equals a full scan of every variant (the definition it replaces).
    fn scan(table: &EnumVariantTable, name: &str, enum_name: &str, base: &str) -> Option<bool> {
        let ordered: std::collections::BTreeMap<&String, &Vec<Type>> = table.as_map().iter().collect();
        let mut saw = false;
        for (key, payloads) in ordered {
            let (key_enum, key_base) = enum_parts(key);
            if key_base == base || key_enum == enum_name || key_enum == name {
                saw = true;
                if !payloads.is_empty() {
                    return Some(false);
                }
            }
        }
        saw.then_some(true)
    }

    #[test]
    fn indexed_lookup_matches_a_full_scan() {
        let mut t = EnumVariantTable::default();
        t.insert("Dir::Up".into(), vec![]);
        t.insert("Dir::Down".into(), vec![]);
        t.insert("geo::Shape::Dot".into(), vec![]);
        t.insert("geo::Shape::Circle".into(), vec![Type::Custom("f32".into())]);
        t.extend([("Dir::Up".to_string(), vec![]), ("other::Dir::Left".to_string(), vec![])]);
        t.insert("Mode".into(), vec![]);
        let queries = [
            ("Dir", "Dir", "Dir"),
            ("Dir::Up", "Dir", "Dir"),
            ("Shape", "Shape", "Shape"),
            ("geo::Shape", "geo::Shape", "Shape"),
            ("geo::Shape::Dot", "geo::Shape", "Shape"),
            ("Mode", "Mode", "Mode"),
            ("Nope", "Nope", "Nope"),
        ];
        for (name, enum_name, base) in queries {
            assert_eq!(t.unit_only(name, enum_name, base), scan(&t, name, enum_name, base), "{name}");
        }
        assert_eq!(t.len(), 6);
    }
}
