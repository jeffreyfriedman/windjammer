//! Map and set bodies (`Value::Map` / `Value::Set`): every read and write goes through `KMap`.
//!
//! `hashed` marks `HashMap`/`HashSet`, whose iteration order is unspecified in Rust (ADR-0021 D6): by
//! default they iterate in key order, and `--hash-seed N` permutes that order deterministically so
//! order-dependent programs can be found. `BTreeMap`/`BTreeSet` always iterate sorted.
//!
//! Storage: a `BTreeMap` ordered by `structural_cmp`, or, for a hashed map whose keys are all of a
//! hash-exact kind (`Key::hashable`: integers, strings, chars, bools, unit), a hash table, which is
//! what makes lookups O(1). Both answer every query identically: a hash-exact key is equal (under
//! `structural_cmp`) only to keys of its own kind with the same value, which is what the hash
//! distinguishes; a lookup with any other key (a float, a reference...) scans; inserting a key of
//! another kind turns the table into a tree for good. Iteration sorts the table's entries by key, so
//! the order is the tree's.

use super::value::{Key, Value};
use std::collections::{BTreeMap, HashMap};
use std::hash::{BuildHasherDefault, Hash, Hasher};

#[derive(Clone, Debug)]
enum Store {
    Tree(BTreeMap<Key, Value>),
    Table(HashMap<HKey, (Key, Value), BuildHasherDefault<Fx>>),
}

#[derive(Clone, Debug)]
pub struct KMap {
    store: Store,
    pub hashed: bool,
}

impl Default for KMap {
    fn default() -> KMap {
        KMap { store: Store::Tree(BTreeMap::new()), hashed: false }
    }
}

/// A hash-exact key: its kind and value.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum HKey {
    Unit,
    Bool(bool),
    Int(i128),
    Char(char),
    Str(std::rc::Rc<String>),
}

impl HKey {
    fn of(k: &Value) -> Option<HKey> {
        Some(match k {
            Value::Unit => HKey::Unit,
            Value::Bool(b) => HKey::Bool(*b),
            Value::Int(i, _) => HKey::Int(*i),
            Value::Char(c) => HKey::Char(*c),
            Value::Str(s) => HKey::Str(s.clone()),
            _ => return None,
        })
    }
}

/// A small multiplicative hasher (keys are integers and short strings; the order is never observed).
#[derive(Default)]
struct Fx(u64);

impl Hasher for Fx {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, bytes: &[u8]) {
        for b in bytes {
            self.0 = (self.0.rotate_left(5) ^ *b as u64).wrapping_mul(0x51_7c_c1_b7_27_22_0a_95);
        }
    }
    fn write_u64(&mut self, n: u64) {
        self.0 = (self.0.rotate_left(5) ^ n).wrapping_mul(0x51_7c_c1_b7_27_22_0a_95);
    }
    fn write_u128(&mut self, n: u128) {
        self.write_u64(n as u64);
        self.write_u64((n >> 64) as u64);
    }
}

fn seeded_hash(seed: u64, k: &Key) -> u64 {
    // FNV-1a over the key's structural text, then a splitmix-style finalizer mixed with the seed
    let mut h: u64 = 0xcbf29ce484222325;
    for b in format!("{:?}", k.0).bytes() {
        h = (h ^ b as u64).wrapping_mul(0x100000001b3);
    }
    let mut z = h ^ seed.wrapping_mul(0x9e3779b97f4a7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
    z ^ (z >> 31)
}

impl From<BTreeMap<Key, Value>> for KMap {
    fn from(m: BTreeMap<Key, Value>) -> KMap {
        KMap { store: Store::Tree(m), hashed: false }
    }
}

impl KMap {
    pub fn hash_map(m: BTreeMap<Key, Value>) -> KMap {
        let mut k = KMap { store: Store::Tree(m), hashed: true };
        k.tabulate();
        k
    }

    /// An empty map of the same kind (`HashMap`/`BTreeMap`).
    pub fn empty_like(&self) -> KMap {
        if self.hashed {
            KMap::hash_map(BTreeMap::new())
        } else {
            KMap::default()
        }
    }

    /// A hashed map whose keys are all hash-exact becomes a table.
    fn tabulate(&mut self) {
        if let (true, Store::Tree(t)) = (self.hashed, &self.store) {
            if t.keys().all(|k| HKey::of(&k.0).is_some()) {
                let Store::Tree(t) = std::mem::replace(&mut self.store, Store::Tree(BTreeMap::new())) else { unreachable!() };
                self.store = Store::Table(t.into_iter().map(|(k, v)| (HKey::of(&k.0).expect("checked"), (k, v))).collect());
            }
        }
    }

    /// Back to a tree (a key of another kind arrives).
    fn untabulate(&mut self) {
        if let Store::Table(t) = std::mem::replace(&mut self.store, Store::Tree(BTreeMap::new())) {
            self.store = Store::Tree(t.into_values().collect());
        }
    }

    pub fn len(&self) -> usize {
        match &self.store {
            Store::Tree(t) => t.len(),
            Store::Table(t) => t.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get(&self, k: &Key) -> Option<&Value> {
        match &self.store {
            Store::Tree(t) => t.get(k),
            Store::Table(t) => match HKey::of(&k.0) {
                Some(h) => t.get(&h).map(|(_, v)| v),
                None => t.values().find(|(kk, _)| kk == k).map(|(_, v)| v),
            },
        }
    }

    pub fn get_key_value(&self, k: &Key) -> Option<(&Key, &Value)> {
        match &self.store {
            Store::Tree(t) => t.get_key_value(k),
            Store::Table(t) => match HKey::of(&k.0) {
                Some(h) => t.get(&h).map(|(kk, v)| (kk, v)),
                None => t.values().find(|(kk, _)| kk == k).map(|(kk, v)| (kk, v)),
            },
        }
    }

    pub fn contains_key(&self, k: &Key) -> bool {
        self.get(k).is_some()
    }

    pub fn get_mut(&mut self, k: &Key) -> Option<&mut Value> {
        match &mut self.store {
            Store::Tree(t) => t.get_mut(k),
            Store::Table(t) => match HKey::of(&k.0) {
                Some(h) => t.get_mut(&h).map(|(_, v)| v),
                None => t.values_mut().find(|(kk, _)| kk == k).map(|(_, v)| v),
            },
        }
    }

    /// `BTreeMap::insert`: an equal key already present keeps its key and gets the new value.
    pub fn insert(&mut self, k: Key, v: Value) -> Option<Value> {
        if let Store::Table(t) = &mut self.store {
            match HKey::of(&k.0) {
                Some(h) => {
                    return match t.get_mut(&h) {
                        Some((_, old)) => Some(std::mem::replace(old, v)),
                        None => {
                            t.insert(h, (k, v));
                            None
                        }
                    };
                }
                None => {
                    if let Some((_, old)) = t.values_mut().find(|(kk, _)| *kk == k) {
                        return Some(std::mem::replace(old, v));
                    }
                    self.untabulate();
                }
            }
        }
        let Store::Tree(t) = &mut self.store else { unreachable!("a table was turned into a tree") };
        t.insert(k, v)
    }

    pub fn remove(&mut self, k: &Key) -> Option<Value> {
        self.remove_entry(k).map(|(_, v)| v)
    }

    pub fn remove_entry(&mut self, k: &Key) -> Option<(Key, Value)> {
        match &mut self.store {
            Store::Tree(t) => t.remove_entry(k),
            Store::Table(t) => {
                let h = match HKey::of(&k.0) {
                    Some(h) => h,
                    None => t.iter().find(|(_, (kk, _))| kk == k).map(|(h, _)| h.clone())?,
                };
                t.remove(&h)
            }
        }
    }

    pub fn clear(&mut self) {
        match &mut self.store {
            Store::Tree(t) => t.clear(),
            Store::Table(t) => t.clear(),
        }
    }

    /// Entries in key order.
    pub fn iter(&self) -> KIter<'_> {
        match &self.store {
            Store::Tree(t) => KIter::Tree(t.iter()),
            Store::Table(t) => {
                let mut v: Vec<(&Key, &Value)> = t.values().map(|(k, v)| (k, v)).collect();
                v.sort_by(|a, b| a.0.cmp(b.0));
                KIter::Sorted(v.into_iter())
            }
        }
    }

    pub fn keys(&self) -> impl DoubleEndedIterator<Item = &Key> {
        self.iter().map(|(k, _)| k)
    }

    pub fn values(&self) -> impl DoubleEndedIterator<Item = &Value> {
        self.iter().map(|(_, v)| v)
    }

    /// The entries as a `BTreeMap` (a copy).
    pub fn to_btree(&self) -> BTreeMap<Key, Value> {
        self.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
    }

    /// Entries in iteration order (see the module docs); `seed` is `--hash-seed` (0 = key order).
    pub fn ordered(&self, seed: u64) -> Vec<(&Key, &Value)> {
        let mut v: Vec<(&Key, &Value)> = self.iter().collect();
        if self.hashed && seed != 0 {
            v.sort_by_cached_key(|(k, _)| seeded_hash(seed, k));
        }
        v
    }

    pub fn ordered_keys(&self, seed: u64) -> Vec<Value> {
        self.ordered(seed).into_iter().map(|(k, _)| k.0.clone()).collect()
    }

    pub fn ordered_values(&self, seed: u64) -> Vec<Value> {
        self.ordered(seed).into_iter().map(|(_, v)| v.clone()).collect()
    }
}

/// A map's entries in key order.
pub enum KIter<'a> {
    Tree(std::collections::btree_map::Iter<'a, Key, Value>),
    Sorted(std::vec::IntoIter<(&'a Key, &'a Value)>),
}

impl<'a> Iterator for KIter<'a> {
    type Item = (&'a Key, &'a Value);
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            KIter::Tree(i) => i.next(),
            KIter::Sorted(i) => i.next(),
        }
    }
}

impl DoubleEndedIterator for KIter<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        match self {
            KIter::Tree(i) => i.next_back(),
            KIter::Sorted(i) => i.next_back(),
        }
    }
}

impl<'a> IntoIterator for &'a KMap {
    type Item = (&'a Key, &'a Value);
    type IntoIter = KIter<'a>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
