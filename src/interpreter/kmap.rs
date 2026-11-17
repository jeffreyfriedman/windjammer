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
//!
//! A hashed map whose keys are all integers in `0..n`, with `n` at most twice its length (counters, ids,
//! histograms), is an array instead (`Dense`, Lua's array part): a key is its slot, an entry is the key and
//! the value (64 bytes, against the table's 100 with its hash key and control byte), and index order is key
//! order, so iteration sorts nothing. A key outside that range turns it into a table; a table becomes an
//! array again when its length reaches a power of two and its keys fit.

use super::value::{Key, Value};
use std::collections::{BTreeMap, HashMap};
use std::hash::{BuildHasherDefault, Hash, Hasher};

#[derive(Clone, Debug)]
enum Store {
    Tree(BTreeMap<Key, Value>),
    Table(HashMap<HKey, (Key, Value), BuildHasherDefault<Fx>>),
    Dense { slots: Vec<Option<(Key, Value)>>, len: usize },
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

/// The array slot of an integer key, when it has one.
fn index(k: &Value) -> Option<usize> {
    match k {
        Value::Int(i, _) if *i >= 0 && *i <= u32::MAX as i128 => Some(*i as usize),
        _ => None,
    }
}

/// Whether entries whose largest key is `max` fit an array once there are `len` of them (at most half of it
/// empty, past a small one).
fn dense_fits(max: usize, len: usize) -> bool {
    max < (2 * len).max(16)
}

/// A small multiplicative hasher (keys are integers and short strings; the order is never observed).
#[derive(Default)]
struct Fx(u64);

impl Hasher for Fx {
    fn finish(&self) -> u64 {
        self.0
    }
    /// Eight bytes per round (a string key costs one multiply per word, not per byte), the tail as one word.
    fn write(&mut self, bytes: &[u8]) {
        let mut words = bytes.chunks_exact(8);
        for w in &mut words {
            self.write_u64(u64::from_le_bytes(w.try_into().expect("eight bytes")));
        }
        let rest = words.remainder();
        if !rest.is_empty() {
            let mut last = [0u8; 8];
            last[..rest.len()].copy_from_slice(rest);
            self.write_u64(u64::from_le_bytes(last) ^ ((rest.len() as u64) << 56));
        }
    }
    fn write_u64(&mut self, n: u64) {
        self.0 = (self.0.rotate_left(5) ^ n).wrapping_mul(0x51_7c_c1_b7_27_22_0a_95);
    }
    fn write_u128(&mut self, n: u128) {
        self.write_u64(n as u64);
        self.write_u64((n >> 64) as u64);
    }
    // the enum discriminant of `HKey` and a string's terminator: one round each, not a byte loop
    fn write_u8(&mut self, n: u8) {
        self.write_u64(n as u64);
    }
    fn write_usize(&mut self, n: usize) {
        self.write_u64(n as u64);
    }
    fn write_isize(&mut self, n: isize) {
        self.write_u64(n as u64);
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

    /// A hashed map whose keys are all hash-exact becomes a table, or an array when they fit one.
    fn tabulate(&mut self) {
        if let (true, Store::Tree(t)) = (self.hashed, &self.store) {
            let slots: Option<Vec<usize>> = t.keys().map(|k| index(&k.0)).collect();
            if slots.is_some_and(|s| s.last().is_none_or(|m| dense_fits(*m, s.len()))) {
                let Store::Tree(t) = std::mem::replace(&mut self.store, Store::Tree(BTreeMap::new())) else { unreachable!() };
                self.store = Self::dense(t.into_iter());
                return;
            }
            if t.keys().all(|k| HKey::of(&k.0).is_some()) {
                let Store::Tree(t) = std::mem::replace(&mut self.store, Store::Tree(BTreeMap::new())) else { unreachable!() };
                self.store = Store::Table(t.into_iter().map(|(k, v)| (HKey::of(&k.0).expect("checked"), (k, v))).collect());
            }
        }
    }

    /// Back to a tree (a key of another kind arrives).
    fn untabulate(&mut self) {
        self.store = match std::mem::replace(&mut self.store, Store::Tree(BTreeMap::new())) {
            Store::Table(t) => Store::Tree(t.into_values().collect()),
            Store::Dense { slots, .. } => Store::Tree(slots.into_iter().flatten().collect()),
            tree => tree,
        };
    }

    /// An array of these entries (their keys all have a slot).
    fn dense(entries: impl Iterator<Item = (Key, Value)>) -> Store {
        let mut slots: Vec<Option<(Key, Value)>> = Vec::new();
        let mut len = 0;
        for (k, v) in entries {
            let i = index(&k.0).expect("a key with a slot");
            if i >= slots.len() {
                slots.resize_with(i + 1, || None);
            }
            if slots[i].is_none() {
                len += 1;
            }
            slots[i] = Some((k, v));
        }
        Store::Dense { slots, len }
    }

    /// An array to a table (a hash-exact key without a slot, or too far past the others, arrives).
    fn spread(&mut self) {
        if let Store::Dense { slots, .. } = std::mem::replace(&mut self.store, Store::Tree(BTreeMap::new())) {
            self.store = Store::Table(slots.into_iter().flatten().map(|(k, v)| (HKey::of(&k.0).expect("an integer key"), (k, v))).collect());
        }
    }

    /// A table whose length just reached a power of two becomes an array when its keys fit one (checked once per
    /// doubling, so the check costs O(1) per insert).
    fn condense(&mut self) {
        let Store::Table(t) = &self.store else { return };
        let n = t.len();
        if n < 8 || !n.is_power_of_two() || !t.values().all(|(k, _)| index(&k.0).is_some_and(|i| dense_fits(i, n))) {
            return;
        }
        let Store::Table(t) = std::mem::replace(&mut self.store, Store::Tree(BTreeMap::new())) else { unreachable!() };
        self.store = Self::dense(t.into_values());
    }

    /// The entry of key `k` in an array: its slot, nothing for another hash-exact kind, a scan otherwise.
    fn slot<'a>(slots: &'a [Option<(Key, Value)>], k: &Key) -> Option<&'a (Key, Value)> {
        match index(&k.0) {
            Some(i) => slots.get(i).and_then(|e| e.as_ref()),
            None if HKey::of(&k.0).is_some() => None,
            None => slots.iter().flatten().find(|(kk, _)| kk == k),
        }
    }

    pub fn len(&self) -> usize {
        match &self.store {
            Store::Tree(t) => t.len(),
            Store::Table(t) => t.len(),
            Store::Dense { len, .. } => *len,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// One of the values (the same one for the same contents), for machine code's guess at their type.
    pub fn any_value(&self) -> Option<&Value> {
        match &self.store {
            Store::Tree(t) => t.values().next(),
            Store::Table(t) => t.values().next().map(|(_, v)| v),
            Store::Dense { slots, .. } => slots.iter().flatten().next().map(|(_, v)| v),
        }
    }

    pub fn get(&self, k: &Key) -> Option<&Value> {
        match &self.store {
            Store::Tree(t) => t.get(k),
            Store::Table(t) => match HKey::of(&k.0) {
                Some(h) => t.get(&h).map(|(_, v)| v),
                None => t.values().find(|(kk, _)| kk == k).map(|(_, v)| v),
            },
            Store::Dense { slots, .. } => Self::slot(slots, k).map(|(_, v)| v),
        }
    }

    /// `get` of an integer key, without building one (machine code's lookups: an array slot or one probe).
    pub fn get_int(&self, i: i128, t: crate::interpreter::int::IntTy) -> Option<&Value> {
        match &self.store {
            Store::Dense { slots, .. } => match usize::try_from(i) {
                Ok(n) => slots.get(n).and_then(|e| e.as_ref()).map(|(_, v)| v),
                Err(_) => None,
            },
            Store::Table(tb) => tb.get(&HKey::Int(i)).map(|(_, v)| v),
            Store::Tree(_) => self.get(&Key(Value::Int(i, t))),
        }
    }

    /// `insert` of an integer key whose old value (if any) is dropped: an array slot written in place.
    pub fn set_int(&mut self, i: i128, t: crate::interpreter::int::IntTy, v: Value) {
        if let Store::Dense { slots, .. } = &mut self.store {
            if let Some(Some((_, old))) = usize::try_from(i).ok().and_then(|n| slots.get_mut(n)) {
                *old = v;
                return;
            }
        }
        self.insert(Key(Value::Int(i, t)), v);
    }

    pub fn get_key_value(&self, k: &Key) -> Option<(&Key, &Value)> {
        match &self.store {
            Store::Tree(t) => t.get_key_value(k),
            Store::Table(t) => match HKey::of(&k.0) {
                Some(h) => t.get(&h).map(|(kk, v)| (kk, v)),
                None => t.values().find(|(kk, _)| kk == k).map(|(kk, v)| (kk, v)),
            },
            Store::Dense { slots, .. } => Self::slot(slots, k).map(|(kk, v)| (kk, v)),
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
            Store::Dense { slots, .. } => match index(&k.0) {
                Some(i) => slots.get_mut(i).and_then(|e| e.as_mut()).map(|(_, v)| v),
                None if HKey::of(&k.0).is_some() => None,
                None => slots.iter_mut().flatten().find(|(kk, _)| kk == k).map(|(_, v)| v),
            },
        }
    }

    /// `BTreeMap::insert`: an equal key already present keeps its key and gets the new value.
    pub fn insert(&mut self, k: Key, v: Value) -> Option<Value> {
        if let Store::Dense { slots, len } = &mut self.store {
            match index(&k.0) {
                Some(i) if i < slots.len() => {
                    return match &mut slots[i] {
                        Some((_, old)) => Some(std::mem::replace(old, v)),
                        e => {
                            *e = Some((k, v));
                            *len += 1;
                            None
                        }
                    };
                }
                Some(i) if dense_fits(i, *len + 1) => {
                    // the vector's capacity grows geometrically, so filling `0..n` in order costs O(n)
                    slots.resize_with(i + 1, || None);
                    slots[i] = Some((k, v));
                    *len += 1;
                    return None;
                }
                Some(_) => self.spread(),
                None if HKey::of(&k.0).is_some() => self.spread(),
                None => {
                    if let Some((_, old)) = slots.iter_mut().flatten().find(|(kk, _)| *kk == k) {
                        return Some(std::mem::replace(old, v));
                    }
                    self.untabulate();
                }
            }
        }
        if let Store::Table(t) = &mut self.store {
            match HKey::of(&k.0) {
                Some(h) => {
                    return match t.get_mut(&h) {
                        Some((_, old)) => Some(std::mem::replace(old, v)),
                        None => {
                            t.insert(h, (k, v));
                            self.condense();
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
            Store::Dense { slots, len } => {
                let i = match index(&k.0) {
                    Some(i) => i,
                    None if HKey::of(&k.0).is_some() => return None,
                    None => slots.iter().position(|e| e.as_ref().is_some_and(|(kk, _)| kk == k))?,
                };
                let e = slots.get_mut(i)?.take()?;
                *len -= 1;
                Some(e)
            }
        }
    }

    pub fn clear(&mut self) {
        match &mut self.store {
            Store::Tree(t) => t.clear(),
            Store::Table(t) => t.clear(),
            Store::Dense { slots, len } => {
                slots.clear();
                *len = 0;
            }
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
            Store::Dense { slots, .. } => KIter::Dense(slots.iter().flatten()),
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
    Dense(std::iter::Flatten<std::slice::Iter<'a, Option<(Key, Value)>>>),
}

impl<'a> Iterator for KIter<'a> {
    type Item = (&'a Key, &'a Value);
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            KIter::Tree(i) => i.next(),
            KIter::Sorted(i) => i.next(),
            KIter::Dense(i) => i.next().map(|(k, v)| (k, v)),
        }
    }
}

impl DoubleEndedIterator for KIter<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        match self {
            KIter::Tree(i) => i.next_back(),
            KIter::Sorted(i) => i.next_back(),
            KIter::Dense(i) => i.next_back().map(|(k, v)| (k, v)),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interpreter::int::IntTy;
    use crate::interpreter::value::FloatTy;

    fn int(i: i128) -> Key {
        Key(Value::Int(i, IntTy::I64))
    }

    fn same(m: &KMap, oracle: &BTreeMap<Key, Value>) -> bool {
        m.iter().map(|(k, v)| (k.clone(), Key(v.clone()))).eq(oracle.iter().map(|(k, v)| (k.clone(), Key(v.clone()))))
    }

    /// Every store answers as the tree does, through each change of store: random inserts, removes and lookups
    /// of dense, sparse, negative and float keys, compared after every operation (a small LCG, so it replays).
    #[test]
    fn every_store_answers_as_the_tree() {
        for (round, spread) in [(0u64, 40i128), (1, 400), (2, 20_000), (3, 12)].into_iter() {
            let mut seed = 0x9e37_79b9_7f4a_7c15u64 ^ round;
            let mut next = move || {
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                seed >> 33
            };
            let mut m = KMap::hash_map(BTreeMap::new());
            let mut oracle: BTreeMap<Key, Value> = BTreeMap::new();
            let mut stores = std::collections::BTreeSet::new();
            for step in 0..4000 {
                let r = next();
                let k = match r % 50 {
                    0 => int(-((next() % 5) as i128) - 1),
                    1 if round == 3 => Key(Value::Float((next() % 8) as f64 + 0.5, FloatTy::F64)),
                    2 if round == 3 => Key(Value::Float((next() % 8) as f64, FloatTy::F64)),
                    _ => int((next() as i128) % spread),
                };
                match next() % 4 {
                    0 => assert_eq!(m.remove(&k).map(Key), oracle.remove(&k).map(Key), "remove {k:?} at {step}"),
                    1 => {
                        assert_eq!(m.get(&k).cloned().map(Key), oracle.get(&k).cloned().map(Key), "get {k:?} at {step}");
                        if let Value::Int(i, t) = k.0 {
                            assert_eq!(m.get_int(i, t).cloned().map(Key), oracle.get(&k).cloned().map(Key), "get_int {i} at {step}");
                        }
                    }
                    2 if matches!(k.0, Value::Int(..)) => {
                        let Value::Int(i, t) = k.0 else { unreachable!() };
                        let v = Value::Int(step as i128, IntTy::I64);
                        m.set_int(i, t, v.clone());
                        oracle.insert(k, v);
                    }
                    _ => {
                        let v = Value::Int(step as i128, IntTy::I64);
                        assert_eq!(m.insert(k.clone(), v.clone()).map(Key), oracle.insert(k, v).map(Key), "insert at {step}");
                    }
                }
                stores.insert(match &m.store {
                    Store::Tree(_) => "tree",
                    Store::Table(_) => "table",
                    Store::Dense { .. } => "dense",
                });
                assert_eq!(m.len(), oracle.len());
                if step % 97 == 0 {
                    assert!(same(&m, &oracle), "iteration at {step}");
                    assert_eq!(m.get_key_value(&int(3)).map(|(k, v)| (k.clone(), Key(v.clone()))), oracle.get_key_value(&int(3)).map(|(k, v)| (k.clone(), Key(v.clone()))));
                }
            }
            assert!(same(&m, &oracle));
            m.clear();
            assert_eq!(m.len(), 0);
            assert!(stores.contains("dense"), "round {round} never used the array: {stores:?}");
        }
    }

    /// Counting into `0..n` in a scattered order (the hashmap kernel) ends as an array; keys far apart do not.
    #[test]
    fn scattered_dense_keys_become_an_array_and_sparse_ones_stay_a_table() {
        let mut m = KMap::hash_map(BTreeMap::new());
        for i in 0..10_007i128 {
            m.insert(int((i * 7919) % 10_007), Value::Unit);
        }
        assert!(matches!(m.store, Store::Dense { len: 10_007, .. }));
        let mut s = KMap::hash_map(BTreeMap::new());
        for i in 0..1000i128 {
            s.insert(int(i * 1000), Value::Unit);
        }
        assert!(matches!(s.store, Store::Table(_)));
    }
}
