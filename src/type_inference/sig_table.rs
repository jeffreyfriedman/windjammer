//! The function signatures the numeric inference consults (`IntInference`, `FloatInference`): the stdlib's,
//! derived once per process and shared, under the program's own (inserted per inference). Building the stdlib part
//! for every inference copied every stdlib signature twice per checked file (5.8M instructions of a `hello`
//! check, ADR-0038 P2); a lookup now reads the program's entries, then the shared ones, and iteration merges both
//! in key order with the program's entry winning, as one map with the stdlib inserted first would.

use crate::parser::Type;
use std::collections::BTreeMap;

/// Parameter types and return type of a function.
pub type Sig = (Vec<Type>, Option<Type>);

/// The shared part: a function returning a table built on first use (`shared`); a lookup the program's entries
/// answer never builds it.
pub type Base = fn() -> &'static BTreeMap<String, Sig>;

fn empty() -> &'static BTreeMap<String, Sig> {
    static EMPTY: BTreeMap<String, Sig> = BTreeMap::new();
    &EMPTY
}

#[derive(Clone, Debug)]
pub struct SigTable {
    base: Base,
    own: BTreeMap<String, Sig>,
}

impl Default for SigTable {
    fn default() -> SigTable {
        SigTable { base: empty, own: BTreeMap::new() }
    }
}

impl SigTable {
    /// A table over the shared `base` with no entries of its own.
    pub fn over(base: Base) -> SigTable {
        SigTable { base, own: BTreeMap::new() }
    }

    pub fn get(&self, key: &str) -> Option<&Sig> {
        self.own.get(key).or_else(|| (self.base)().get(key))
    }

    pub fn contains_key(&self, key: &str) -> bool {
        self.own.contains_key(key) || (self.base)().contains_key(key)
    }

    /// Insert or replace the entry for `key` (a stdlib entry is shadowed, never changed).
    pub fn insert(&mut self, key: String, sig: Sig) {
        self.own.insert(key, sig);
    }

    pub fn extend<I: IntoIterator<Item = (String, Sig)>>(&mut self, entries: I) {
        for (k, v) in entries {
            self.insert(k, v);
        }
    }

    /// Every entry in key order; a key in both parts yields the program's entry.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &Sig)> {
        let mut own = self.own.iter().peekable();
        let mut base = (self.base)().iter().peekable();
        std::iter::from_fn(move || match (own.peek(), base.peek()) {
            (Some((a, _)), Some((b, _))) => {
                if a < b {
                    own.next()
                } else if b < a {
                    base.next()
                } else {
                    base.next();
                    own.next()
                }
            }
            (Some(_), None) => own.next(),
            (None, Some(_)) => base.next(),
            (None, None) => None,
        })
    }

    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.iter().map(|(k, _)| k)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sig(n: usize) -> Sig {
        (vec![Type::Int; n], None)
    }

    /// Lookups, inserts and iteration behave as one map with the base inserted first.
    #[test]
    fn a_layered_table_is_the_map_it_stands_for() {
        let mut base = BTreeMap::new();
        for (k, n) in [("a", 1), ("c", 3), ("e", 5)] {
            base.insert(k.to_string(), sig(n));
        }
        static BASE: std::sync::OnceLock<BTreeMap<String, Sig>> = std::sync::OnceLock::new();
        let _ = BASE.set(base.clone());
        let mut t = SigTable::over(|| BASE.get().expect("set"));
        let mut flat = base;
        for (k, n) in [("b", 2), ("c", 30), ("f", 6)] {
            t.insert(k.to_string(), sig(n));
            flat.insert(k.to_string(), sig(n));
        }
        t.extend(vec![("a".to_string(), sig(10))]);
        flat.extend(vec![("a".to_string(), sig(10))]);
        assert_eq!(t.iter().map(|(k, v)| (k.clone(), v.clone())).collect::<Vec<_>>(), flat.iter().map(|(k, v)| (k.clone(), v.clone())).collect::<Vec<_>>());
        assert_eq!(t.get("c"), flat.get("c"));
        assert_eq!(t.get("e"), flat.get("e"));
        assert!(t.contains_key("f") && !t.contains_key("z"));
        assert_eq!(t.keys().count(), flat.len());
    }
}
