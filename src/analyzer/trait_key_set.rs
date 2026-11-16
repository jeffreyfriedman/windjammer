//! Trait-definition key set with an index by method name (library multipass, Q-MEM-1).

use std::collections::{BTreeSet, HashMap};

/// Trait-definition keys (`Trait::method`, `path::Trait::method`) with an index by method name (the last
/// path segment), so looking up the trait definitions of one method never scans every trait key.
#[derive(Debug, Clone, Default)]
pub(crate) struct TraitKeySet {
    keys: BTreeSet<String>,
    by_method: HashMap<String, Vec<String>>,
}

impl PartialEq for TraitKeySet {
    fn eq(&self, other: &Self) -> bool {
        self.keys == other.keys
    }
}

impl TraitKeySet {
    pub(crate) fn insert(&mut self, key: String) -> bool {
        if self.keys.contains(&key) {
            return false;
        }
        let method = key.rsplit("::").next().unwrap_or(&key).to_string();
        self.by_method.entry(method).or_default().push(key.clone());
        self.keys.insert(key)
    }

    pub(crate) fn extend(&mut self, added: impl IntoIterator<Item = String>) {
        for k in added {
            self.insert(k);
        }
    }

    pub(crate) fn contains(&self, key: &str) -> bool {
        self.keys.contains(key)
    }

    pub(crate) fn iter(&self) -> std::collections::btree_set::Iter<'_, String> {
        self.keys.iter()
    }

    /// The keys `keep` accepts, in key order.
    pub(crate) fn keys_matching<'a>(&'a self, keep: impl Fn(&String) -> bool + 'a) -> impl Iterator<Item = &'a String> {
        self.keys.iter().filter(move |k| keep(k))
    }

    pub(crate) fn len(&self) -> usize {
        self.keys.len()
    }

    /// The keys whose last path segment is `method` (`method` itself has no `::`).
    pub(crate) fn named(&self, method: &str) -> &[String] {
        self.by_method.get(method).map(Vec::as_slice).unwrap_or(&[])
    }
}

impl FromIterator<String> for TraitKeySet {
    fn from_iter<I: IntoIterator<Item = String>>(keys: I) -> Self {
        let mut set = TraitKeySet::default();
        set.extend(keys);
        set
    }
}
