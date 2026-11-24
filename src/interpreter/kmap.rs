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
//!
//! A hashed map of such integer keys whose values are all `Value::Int` of one 64-bit signed type is a typed array
//! (`IntDense`) instead: the values as `Vec<i64>` and a presence bitmap, the key implicit in the index (8 bytes and
//! one bit an entry, against 64). It claims: every key is `Value::Int` of the one type `kt` in `0..=u32::MAX`, every
//! value `Value::Int` of the one type `vt` (`typed_ok`), absent slots are 0 with their bit clear. Any operation it
//! cannot serve as the array would (a `&mut Value` into it, a key or value of another type, a key past the array's
//! reach) turns it back into `Dense` first and never promotes that map again (`int_off`). Reads of it build their
//! `Value`s (the accessors return `Cow`). See `perf/SOUNDNESS-intdense.md`.

use super::int::IntTy;
use super::value::{Key, Value};
use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap};
use std::hash::{BuildHasherDefault, Hash, Hasher};

#[derive(Clone, Debug)]
enum Store {
    Tree(BTreeMap<Key, Value>),
    Table(HashMap<HKey, (Key, Value), BuildHasherDefault<Fx>>),
    Dense { slots: Vec<Option<(Key, Value)>>, len: usize },
    IntDense(IntDense),
}

/// `vals[i]` is the value of key `i` when bit `i` of `present` is set (else 0).
#[derive(Clone, Debug)]
pub struct IntDense {
    vals: Vec<i64>,
    present: Vec<u64>,
    len: usize,
    kt: IntTy,
    vt: IntTy,
}

/// Value types an `IntDense` holds: the signed 64-bit ones (`Lit` is an unsuffixed literal's width, 64 signed too;
/// a value of it past `i64` is refused where it arrives, by the `i64::try_from` there). Other widths (i32, u64,
/// bool...) stay on the ordinary store.
fn typed_ok(t: IntTy) -> bool {
    matches!(t, IntTy::I64 | IntTy::Isize | IntTy::Lit)
}

impl IntDense {
    fn has(&self, i: usize) -> bool {
        self.present.get(i >> 6).is_some_and(|w| w >> (i & 63) & 1 == 1)
    }
    fn at(&self, i: usize) -> Option<i64> {
        if self.has(i) {
            Some(self.vals[i])
        } else {
            None
        }
    }
    fn value(&self, x: i64) -> Value {
        Value::Int(x as i128, self.vt)
    }
    fn key(&self, i: usize) -> Key {
        Key(Value::Int(i as i128, self.kt))
    }
    fn grow(&mut self, n: usize) {
        if n > self.vals.len() {
            self.vals.resize(n, 0);
            self.present.resize(n.div_ceil(64), 0);
        }
    }
    /// Store `x` at `i` (the array already reaches it); the old value if the slot was present.
    fn put(&mut self, i: usize, x: i64) -> Option<i64> {
        let old = self.at(i);
        self.vals[i] = x;
        if old.is_none() {
            self.present[i >> 6] |= 1 << (i & 63);
            self.len += 1;
        }
        old
    }
    fn take(&mut self, i: usize) -> Option<i64> {
        let old = self.at(i)?;
        self.vals[i] = 0;
        self.present[i >> 6] &= !(1 << (i & 63));
        self.len -= 1;
        Some(old)
    }
    /// The index of key `k`: its slot when present, a scan for a key that has no slot but is not hash-exact.
    fn find(&self, k: &Key) -> Option<usize> {
        match index(&k.0) {
            Some(i) => self.has(i).then_some(i),
            None if HKey::of(&k.0).is_some() => None,
            None => (0..self.vals.len()).find(|&i| self.has(i) && self.key(i) == *k),
        }
    }
    fn first_from(&self, mut i: usize, end: usize) -> Option<usize> {
        while i < end {
            let w = self.present[i >> 6] >> (i & 63);
            if w != 0 {
                let j = i + w.trailing_zeros() as usize;
                return (j < end).then_some(j);
            }
            i = (i | 63) + 1;
        }
        None
    }
    fn last_before(&self, mut end: usize, start: usize) -> Option<usize> {
        while end > start {
            let i = end - 1;
            let w = self.present[i >> 6] << (63 - (i & 63));
            if w != 0 {
                let j = i - w.leading_zeros() as usize;
                return (j >= start).then_some(j);
            }
            end = i & !63;
        }
        None
    }
    fn into_slots(self) -> Vec<Option<(Key, Value)>> {
        (0..self.vals.len()).map(|i| self.at(i).map(|x| (self.key(i), self.value(x)))).collect()
    }
    /// The typed array of these entries (all with a slot), when every key has one type and every value is an
    /// integer of one `typed_ok` type.
    fn from_entries<'a>(entries: impl Iterator<Item = (&'a Key, &'a Value)> + Clone) -> Option<IntDense> {
        let (mut kt, mut vt, mut max, mut n) = (None, None, 0usize, 0usize);
        for (k, v) in entries.clone() {
            let (Value::Int(_, k_t), Value::Int(x, v_t)) = (&k.0, v) else { return None };
            if *kt.get_or_insert(*k_t) != *k_t || *vt.get_or_insert(*v_t) != *v_t || !typed_ok(*v_t) || i64::try_from(*x).is_err() {
                return None;
            }
            max = max.max(index(&k.0)?);
            n += 1;
        }
        let mut d = IntDense { vals: Vec::new(), present: Vec::new(), len: 0, kt: kt?, vt: vt? };
        d.grow(max + 1);
        for (k, v) in entries {
            let (Value::Int(x, _), Some(i)) = (v, index(&k.0)) else { return None };
            d.put(i, *x as i64);
        }
        (d.len == n).then_some(d)
    }
}

#[derive(Clone, Debug)]
pub struct KMap {
    store: Store,
    pub hashed: bool,
    /// This map was a typed array once and turned back: it is not promoted again (no flip-flop).
    int_off: bool,
}

impl Default for KMap {
    fn default() -> KMap {
        KMap { store: Store::Tree(BTreeMap::new()), hashed: false, int_off: false }
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
        KMap { store: Store::Tree(m), hashed: false, int_off: false }
    }
}

impl KMap {
    pub fn hash_map(m: BTreeMap<Key, Value>) -> KMap {
        let mut k = KMap { store: Store::Tree(m), hashed: true, int_off: false };
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
        self.demote();
        self.store = match std::mem::replace(&mut self.store, Store::Tree(BTreeMap::new())) {
            Store::Table(t) => Store::Tree(t.into_values().collect()),
            Store::Dense { slots, .. } => Store::Tree(slots.into_iter().flatten().collect()),
            Store::IntDense(_) => unreachable!("demoted above"),
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
        self.demote();
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
        if !self.int_off {
            if let Some(d) = IntDense::from_entries(t.values().map(|(k, v)| (k, v))) {
                self.store = Store::IntDense(d);
                return;
            }
        }
        let Store::Table(t) = std::mem::replace(&mut self.store, Store::Tree(BTreeMap::new())) else { unreachable!() };
        self.store = Self::dense(t.into_values());
    }

    /// An array whose length just reached a power of two becomes a typed array when its entries are all of one
    /// integer type (checked once per doubling: O(1) per insert; a map that was a typed array before stays an array).
    fn promote(&mut self) {
        let Store::Dense { slots, len } = &self.store else {
            return;
        };
        if self.int_off || *len < 8 || !len.is_power_of_two() {
            return;
        }
        if let Some(d) = IntDense::from_entries(slots.iter().flatten().map(|(k, v)| (k, v))) {
            self.store = Store::IntDense(d);
        }
    }

    /// A typed array back to an array (an operation it cannot serve): the same entries in the same slots.
    fn demote(&mut self) {
        if let Store::IntDense(_) = &self.store {
            let Store::IntDense(d) =
                std::mem::replace(&mut self.store, Store::Tree(BTreeMap::new()))
            else {
                unreachable!()
            };
            let len = d.len;
            self.store = Store::Dense {
                slots: d.into_slots(),
                len,
            };
            self.int_off = true;
        }
    }

    /// The kind of store, for tests ("tree", "table", "dense", "intdense").
    pub fn store_kind(&self) -> &'static str {
        match &self.store {
            Store::Tree(_) => "tree",
            Store::Table(_) => "table",
            Store::Dense { .. } => "dense",
            Store::IntDense(_) => "intdense",
        }
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
            Store::IntDense(d) => d.len,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// One of the values (the same one for the same contents), for machine code's guess at their type.
    pub fn any_value(&self) -> Option<Cow<'_, Value>> {
        match &self.store {
            Store::Tree(t) => t.values().next().map(Cow::Borrowed),
            Store::Table(t) => t.values().next().map(|(_, v)| Cow::Borrowed(v)),
            Store::Dense { slots, .. } => {
                slots.iter().flatten().next().map(|(_, v)| Cow::Borrowed(v))
            }
            Store::IntDense(d) => d
                .first_from(0, d.vals.len())
                .map(|i| Cow::Owned(d.value(d.vals[i]))),
        }
    }

    pub fn get(&self, k: &Key) -> Option<Cow<'_, Value>> {
        match &self.store {
            Store::Tree(t) => t.get(k).map(Cow::Borrowed),
            Store::Table(t) => match HKey::of(&k.0) {
                Some(h) => t.get(&h).map(|(_, v)| Cow::Borrowed(v)),
                None => t
                    .values()
                    .find(|(kk, _)| kk == k)
                    .map(|(_, v)| Cow::Borrowed(v)),
            },
            Store::Dense { slots, .. } => Self::slot(slots, k).map(|(_, v)| Cow::Borrowed(v)),
            Store::IntDense(d) => d.find(k).map(|i| Cow::Owned(d.value(d.vals[i]))),
        }
    }

    /// `get(k).cloned()`.
    pub fn get_cloned(&self, k: &Key) -> Option<Value> {
        self.get(k).map(Cow::into_owned)
    }

    /// The typed array's answer for integer key `i` (machine code's lookup): `None` when this is not a typed
    /// array, else the value's bits and type (`None` inside: absent).
    #[inline]
    pub fn typed_get(&self, i: i128) -> Option<Option<(i64, IntTy)>> {
        match &self.store {
            Store::IntDense(d) => Some(
                usize::try_from(i)
                    .ok()
                    .and_then(|n| d.at(n))
                    .map(|x| (x, d.vt)),
            ),
            _ => None,
        }
    }

    /// `get` of an integer key, without building one (machine code's lookups: an array slot or one probe).
    pub fn get_int(&self, i: i128, t: IntTy) -> Option<Cow<'_, Value>> {
        match &self.store {
            Store::Dense { slots, .. } => match usize::try_from(i) {
                Ok(n) => slots
                    .get(n)
                    .and_then(|e| e.as_ref())
                    .map(|(_, v)| Cow::Borrowed(v)),
                Err(_) => None,
            },
            Store::Table(tb) => tb.get(&HKey::Int(i)).map(|(_, v)| Cow::Borrowed(v)),
            Store::IntDense(d) => usize::try_from(i)
                .ok()
                .and_then(|n| d.at(n))
                .map(|x| Cow::Owned(d.value(x))),
            Store::Tree(_) => self.get(&Key(Value::Int(i, t))),
        }
    }

    /// `insert` of an integer key whose old value (if any) is dropped: an array slot written in place.
    /// `set_int` of an integer value given as bits (machine code's insert): `true` when a typed array took it in
    /// place (the value's type is the array's, the slot is there, present or absent with this key type), `false`
    /// with nothing changed: the caller then builds the `Value` and calls `set_int`.
    #[inline]
    pub fn set_int_bits(&mut self, i: i128, t: IntTy, x: i64, xt: IntTy) -> bool {
        let Store::IntDense(d) = &mut self.store else { return false };
        match usize::try_from(i) {
            Ok(n) if xt == d.vt && n < d.vals.len() => {
                let (w, bit) = (n >> 6, 1u64 << (n & 63));
                if d.present[w] & bit != 0 {
                    d.vals[n] = x;
                    true
                } else if t == d.kt {
                    d.present[w] |= bit;
                    d.vals[n] = x;
                    d.len += 1;
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    pub fn set_int(&mut self, i: i128, t: IntTy, v: Value) {
        if let (Store::IntDense(d), Value::Int(x, xt)) = (&mut self.store, &v) {
            if let (Ok(n), Ok(xv)) = (usize::try_from(i), i64::try_from(*x)) {
                // an array of this value type, the slot there (a present one, or an absent one of this key type)
                if *xt == d.vt && (d.has(n) || (t == d.kt && n < d.vals.len())) {
                    d.put(n, xv);
                    // an integer owns nothing: no drop glue to run
                    std::mem::forget(v);
                    return;
                }
            }
        }
        if let Store::Dense { slots, .. } = &mut self.store {
            if let Some(Some((_, old))) = usize::try_from(i).ok().and_then(|n| slots.get_mut(n)) {
                // an integer over an integer: both fields in place (no drop glue for the replaced value)
                if let (Value::Int(a, at), &Value::Int(b, bt)) = (&mut *old, &v) {
                    *a = b;
                    *at = bt;
                    // an integer owns nothing: no drop glue to run
                    std::mem::forget(v);
                } else {
                    *old = v;
                }
                return;
            }
        }
        self.insert(Key(Value::Int(i, t)), v);
    }

    pub fn get_key_value(&self, k: &Key) -> Option<(Cow<'_, Key>, Cow<'_, Value>)> {
        fn b<'a>((kk, v): (&'a Key, &'a Value)) -> (Cow<'a, Key>, Cow<'a, Value>) {
            (Cow::Borrowed(kk), Cow::Borrowed(v))
        }
        match &self.store {
            Store::Tree(t) => t.get_key_value(k).map(b),
            Store::Table(t) => match HKey::of(&k.0) {
                Some(h) => t.get(&h).map(|(kk, v)| b((kk, v))),
                None => t.values().find(|(kk, _)| kk == k).map(|(kk, v)| b((kk, v))),
            },
            Store::Dense { slots, .. } => Self::slot(slots, k).map(|(kk, v)| b((kk, v))),
            Store::IntDense(d) => d
                .find(k)
                .map(|i| (Cow::Owned(d.key(i)), Cow::Owned(d.value(d.vals[i])))),
        }
    }

    pub fn contains_key(&self, k: &Key) -> bool {
        self.get(k).is_some()
    }

    pub fn get_mut(&mut self, k: &Key) -> Option<&mut Value> {
        // a `&mut Value` into the map: the typed array has none to give
        self.demote();
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
            Store::IntDense(_) => unreachable!("demoted above"),
        }
    }

    /// `BTreeMap::insert`: an equal key already present keeps its key and gets the new value.
    pub fn insert(&mut self, k: Key, v: Value) -> Option<Value> {
        if let Store::IntDense(d) = &mut self.store {
            if let (Some(i), Value::Int(x, xt), Value::Int(_, kt)) = (index(&k.0), &v, &k.0) {
                if let (true, Ok(xv)) = (*xt == d.vt, i64::try_from(*x)) {
                    if d.has(i) {
                        // an equal key keeps the stored key (BTreeMap::insert's rule); the value is replaced
                        return d.put(i, xv).map(|o| Value::Int(o as i128, d.vt));
                    }
                    if *kt == d.kt && (i < d.vals.len() || dense_fits(i, d.len + 1)) {
                        d.grow(i + 1);
                        d.put(i, xv);
                        return None;
                    }
                }
            }
            // a key or value the array cannot hold: an ordinary array, and the old rules
            self.demote();
        }
        if let Store::Dense { slots, len } = &mut self.store {
            match index(&k.0) {
                Some(i) if i < slots.len() => {
                    let r = match &mut slots[i] {
                        Some((_, old)) => Some(std::mem::replace(old, v)),
                        e => {
                            *e = Some((k, v));
                            *len += 1;
                            None
                        }
                    };
                    if r.is_none() {
                        self.promote();
                    }
                    return r;
                }
                Some(i) if dense_fits(i, *len + 1) => {
                    // the vector's capacity grows geometrically, so filling `0..n` in order costs O(n)
                    slots.resize_with(i + 1, || None);
                    slots[i] = Some((k, v));
                    *len += 1;
                    self.promote();
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
        if let Store::IntDense(d) = &mut self.store {
            match index(&k.0) {
                Some(i) => return d.take(i).map(|x| (d.key(i), d.value(x))),
                None if HKey::of(&k.0).is_some() => return None,
                // a key that is not hash-exact (a float...): found by a scan, as the array does
                None => self.demote(),
            }
        }
        match &mut self.store {
            Store::IntDense(_) => unreachable!("demoted above"),
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
            Store::IntDense(d) => {
                d.vals.clear();
                d.present.clear();
                d.len = 0;
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
            Store::IntDense(d) => KIter::Int {
                d,
                lo: 0,
                hi: d.vals.len(),
            },
        }
    }

    pub fn keys(&self) -> impl DoubleEndedIterator<Item = Cow<'_, Key>> {
        self.iter().map(|(k, _)| k)
    }

    pub fn values(&self) -> impl DoubleEndedIterator<Item = Cow<'_, Value>> {
        self.iter().map(|(_, v)| v)
    }

    /// The entries as a `BTreeMap` (a copy).
    pub fn to_btree(&self) -> BTreeMap<Key, Value> {
        self.iter()
            .map(|(k, v)| (k.into_owned(), v.into_owned()))
            .collect()
    }

    /// Entries in iteration order (see the module docs); `seed` is `--hash-seed` (0 = key order).
    pub fn ordered(&self, seed: u64) -> Vec<(Cow<'_, Key>, Cow<'_, Value>)> {
        let mut v: Vec<(Cow<'_, Key>, Cow<'_, Value>)> = self.iter().collect();
        if self.hashed && seed != 0 {
            v.sort_by_cached_key(|(k, _)| seeded_hash(seed, k));
        }
        v
    }

    pub fn ordered_keys(&self, seed: u64) -> Vec<Value> {
        self.ordered(seed).into_iter().map(|(k, _)| k.into_owned().0).collect()
    }

    pub fn ordered_values(&self, seed: u64) -> Vec<Value> {
        self.ordered(seed).into_iter().map(|(_, v)| v.into_owned()).collect()
    }
}

/// A map's entries in key order (borrowed from the stores that hold them, built for the typed array).
pub enum KIter<'a> {
    Tree(std::collections::btree_map::Iter<'a, Key, Value>),
    Sorted(std::vec::IntoIter<(&'a Key, &'a Value)>),
    Dense(std::iter::Flatten<std::slice::Iter<'a, Option<(Key, Value)>>>),
    Int { d: &'a IntDense, lo: usize, hi: usize },
}

impl<'a> Iterator for KIter<'a> {
    type Item = (Cow<'a, Key>, Cow<'a, Value>);
    fn next(&mut self) -> Option<Self::Item> {
        let b = |(k, v): (&'a Key, &'a Value)| (Cow::Borrowed(k), Cow::Borrowed(v));
        match self {
            KIter::Tree(i) => i.next().map(b),
            KIter::Sorted(i) => i.next().map(b),
            KIter::Dense(i) => i.next().map(|(k, v)| b((k, v))),
            KIter::Int { d, lo, hi } => {
                let i = d.first_from(*lo, *hi)?;
                *lo = i + 1;
                Some((Cow::Owned(d.key(i)), Cow::Owned(d.value(d.vals[i]))))
            }
        }
    }
}

impl DoubleEndedIterator for KIter<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let b = |(k, v)| (Cow::Borrowed(k), Cow::Borrowed(v));
        match self {
            KIter::Tree(i) => i.next_back().map(b),
            KIter::Sorted(i) => i.next_back().map(b),
            KIter::Dense(i) => i.next_back().map(|(k, v)| b((k, v))),
            KIter::Int { d, lo, hi } => {
                let i = d.last_before(*hi, *lo)?;
                *hi = i;
                Some((Cow::Owned(d.key(i)), Cow::Owned(d.value(d.vals[i]))))
            }
        }
    }
}

impl<'a> IntoIterator for &'a KMap {
    type Item = (Cow<'a, Key>, Cow<'a, Value>);
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
        m.iter().map(|(k, v)| (k.into_owned(), Key(v.into_owned()))).eq(oracle.iter().map(|(k, v)| (k.clone(), Key(v.clone()))))
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
                        assert_eq!(m.get_cloned(&k).map(Key), oracle.get(&k).cloned().map(Key), "get {k:?} at {step}");
                        if let Value::Int(i, t) = k.0 {
                            assert_eq!(m.get_int(i, t).map(|x| Key(x.into_owned())), oracle.get(&k).cloned().map(Key), "get_int {i} at {step}");
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
                stores.insert(m.store_kind());
                assert_eq!(m.len(), oracle.len());
                if step % 97 == 0 {
                    assert!(same(&m, &oracle), "iteration at {step}");
                    assert_eq!(m.get_key_value(&int(3)).map(|(k, v)| (k.into_owned(), Key(v.into_owned()))), oracle.get_key_value(&int(3)).map(|(k, v)| (k.clone(), Key(v.clone()))));
                }
            }
            assert!(same(&m, &oracle));
            m.clear();
            assert_eq!(m.len(), 0);
            assert!(stores.contains("dense") || stores.contains("intdense"), "round {round} never used the array: {stores:?}");
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

    macro_rules! same {
        ($a:expr, $b:expr) => {
            assert_eq!(format!("{:?}", $a), format!("{:?}", $b))
        };
        ($a:expr, $b:expr, $($m:tt)+) => {
            assert_eq!(format!("{:?}", $a), format!("{:?}", $b), $($m)+)
        };
    }

    fn val(i: i128) -> Value {
        Value::Int(i, IntTy::I64)
    }

    /// An int-keyed, int-valued hashed map of `n` ascending entries.
    fn filled(n: i128) -> KMap {
        let mut m = KMap::hash_map(BTreeMap::new());
        for i in 0..n {
            m.insert(int(i), val(i * 3 - 7));
        }
        m
    }

    /// The same operations on a map that is never typed (`int_off`: exactly the previous store) and on one that may be.
    struct Pair {
        typed: KMap,
        plain: KMap,
    }

    impl Pair {
        fn new() -> Pair {
            let mut plain = KMap::hash_map(BTreeMap::new());
            plain.int_off = true;
            Pair {
                typed: KMap::hash_map(BTreeMap::new()),
                plain,
            }
        }
        fn agree(&self, what: &str) {
            let dump = |m: &KMap| {
                format!(
                    "{:?} {:?}",
                    m.iter().collect::<Vec<_>>(),
                    m.iter().rev().collect::<Vec<_>>()
                )
            };
            assert_eq!(
                dump(&self.typed),
                dump(&self.plain),
                "iteration after {what}"
            );
            assert_eq!(self.typed.len(), self.plain.len(), "len after {what}");
            assert_eq!(
                format!("{:?}", self.typed.any_value()),
                format!("{:?}", self.plain.any_value()),
                "any_value after {what}"
            );
            for seed in [0u64, 5] {
                assert_eq!(
                    format!("{:?}", self.typed.ordered(seed)),
                    format!("{:?}", self.plain.ordered(seed)),
                    "ordered({seed}) after {what}"
                );
            }
        }
    }

    /// Random operation sequences (a fixed seed) answer identically on the typed store and on the previous one,
    /// result by result and iteration by iteration; the run must actually use, leave and re-enter nothing but the
    /// typed store where promotion applies.
    #[test]
    fn typed_array_answers_as_the_previous_store() {
        let mut kinds_seen = std::collections::BTreeSet::new();
        for round in 0..24u64 {
            let mut seed = 0x1234_5678_9abc_def1u64 ^ (round * 0x9e37);
            let mut next = move || {
                seed = seed
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                seed >> 33
            };
            // rounds differ in how far keys reach, whether foreign operations occur, and how long
            let (spread, foreign, steps) = match round % 6 {
                0 => (24i128, false, 1500),
                1 => (300, false, 3000),
                2 => (60, true, 3000),
                3 => (9000, false, 3000),
                4 => (40, true, 1500),
                _ => (130, false, 3000),
            };
            let mut p = Pair::new();
            for step in 0..steps {
                let r = next() % 100;
                let kv = (next() as i128) % spread;
                let mut key = int(kv);
                if foreign && r % 17 == 0 {
                    key = match next() % 4 {
                        0 => int(-(kv + 1)),
                        1 => Key(Value::Int(kv, IntTy::I32)),
                        2 => Key(Value::Float(kv as f64 + 0.5, FloatTy::F64)),
                        _ => int(kv + 5_000_000),
                    };
                }
                let mut x = val((next() % 2000) as i128 - 1000);
                if foreign && r % 23 == 0 {
                    x = match next() % 3 {
                        0 => Value::Int(7, IntTy::I32),
                        1 => Value::Bool(true),
                        _ => Value::Int(5, IntTy::U64),
                    };
                }
                if r % 41 == 0 && round % 3 == 0 {
                    x = val(i64::MIN as i128 + (next() % 3) as i128);
                }
                let what = format!("round {round} step {step} op {r} key {key:?} value {x:?}");
                match r {
                    0..=39 => assert_eq!(
                        format!("{:?}", p.typed.insert(key.clone(), x.clone())),
                        format!("{:?}", p.plain.insert(key, x)),
                        "insert {what}"
                    ),
                    40..=49 => {
                        if let Value::Int(i, t) = key.0 {
                            // machine code's bits entry first (when the value is a plain int), else the `Value` one
                            let taken = match &x {
                                Value::Int(b, bt) if step % 2 == 0 => i64::try_from(*b).is_ok_and(|b| p.typed.set_int_bits(i, t, b, *bt)),
                                _ => false,
                            };
                            if !taken {
                                p.typed.set_int(i, t, x.clone());
                            }
                            p.plain.set_int(i, t, x);
                        }
                    }
                    50..=64 => assert_eq!(
                        format!("{:?}", p.typed.remove_entry(&key)),
                        format!("{:?}", p.plain.remove_entry(&key)),
                        "remove {what}"
                    ),
                    65..=79 => {
                        assert_eq!(
                            format!("{:?}", p.typed.get(&key)),
                            format!("{:?}", p.plain.get(&key)),
                            "get {what}"
                        );
                        assert_eq!(
                            p.typed.contains_key(&key),
                            p.plain.contains_key(&key),
                            "contains {what}"
                        );
                        assert_eq!(
                            format!("{:?}", p.typed.get_key_value(&key)),
                            format!("{:?}", p.plain.get_key_value(&key)),
                            "get_key_value {what}"
                        );
                        if let Value::Int(i, t) = key.0 {
                            assert_eq!(
                                format!("{:?}", p.typed.get_int(i, t)),
                                format!("{:?}", p.plain.get_int(i, t)),
                                "get_int {what}"
                            );
                        }
                    }
                    80..=84 => {
                        // a clone mutated apart from its source, both directions
                        let (mut a, b) = (p.typed.clone(), p.typed.clone());
                        a.insert(key.clone(), x.clone());
                        a.remove(&key);
                        a.clear();
                        assert_eq!(
                            format!("{:?}", b.iter().collect::<Vec<_>>()),
                            format!("{:?}", p.plain.iter().collect::<Vec<_>>()),
                            "clone untouched {what}"
                        );
                    }
                    85..=87 if round % 4 == 0 => {
                        let (a, b) = (
                            p.typed.get_mut(&key).map(|v| std::mem::replace(v, val(77))),
                            p.plain.get_mut(&key).map(|v| std::mem::replace(v, val(77))),
                        );
                        assert_eq!(format!("{a:?}"), format!("{b:?}"), "get_mut {what}");
                    }
                    88 if next() % 8 == 0 => {
                        p.typed.clear();
                        p.plain.clear();
                    }
                    _ => {}
                }
                kinds_seen.insert(p.typed.store_kind());
                if step % 53 == 0 {
                    p.agree(&what);
                }
            }
            p.agree("the end");
        }
        assert!(
            kinds_seen.contains("intdense")
                && kinds_seen.contains("dense")
                && kinds_seen.contains("table"),
            "{kinds_seen:?}"
        );
    }

    #[test]
    fn promotes_at_the_power_of_two_and_only_for_one_int_type() {
        assert_eq!(filled(7).store_kind(), "dense");
        assert_eq!(filled(8).store_kind(), "intdense");
        assert_eq!(filled(9).store_kind(), "intdense");
        // values of one other type: refused (i32, u64, bool), mixed i64 and i32: refused, float refused
        for other in [
            Value::Int(1, IntTy::I32),
            Value::Int(1, IntTy::U64),
            Value::Bool(true),
            Value::Float(1.0, FloatTy::F64),
            Value::Int(1, IntTy::Usize),
        ] {
            let mut m = KMap::hash_map(BTreeMap::new());
            for i in 0..16 {
                m.insert(int(i), other.clone());
            }
            assert_eq!(m.store_kind(), "dense", "{other:?}");
        }
        let mut mixed = KMap::hash_map(BTreeMap::new());
        for i in 0..7 {
            mixed.insert(int(i), val(i));
        }
        mixed.insert(int(7), Value::Int(1, IntTy::I32));
        assert_eq!(mixed.store_kind(), "dense");
        // keys of two int types: refused
        let mut keys = KMap::hash_map(BTreeMap::new());
        for i in 0..7 {
            keys.insert(int(i), val(i));
        }
        keys.insert(Key(Value::Int(7, IntTy::I32)), val(7));
        assert_eq!(keys.store_kind(), "dense");
        // a scattered fill (the hashmap kernel) goes table -> typed array without the 64-byte array in between
        let mut k = KMap::hash_map(BTreeMap::new());
        for i in 0..1009i128 {
            k.insert(int((i * 7919) % 1009), val(1));
        }
        assert_eq!(k.store_kind(), "intdense");
        assert_eq!(k.len(), 1009);
        // a set (unit values) and a BTreeMap never are
        let mut set = KMap::hash_map(BTreeMap::new());
        let mut bt = KMap::default();
        for i in 0..64 {
            set.insert(int(i), Value::Unit);
            bt.insert(int(i), val(i));
        }
        assert_eq!((set.store_kind(), bt.store_kind()), ("dense", "tree"));
    }

    #[test]
    fn demotes_on_what_it_cannot_hold_and_is_not_promoted_again() {
        let cases: Vec<(&str, Box<dyn Fn(&mut KMap)>)> = vec![
            (
                "negative key",
                Box::new(|m| {
                    m.insert(int(-1), val(1));
                }),
            ),
            (
                "key past the reach",
                Box::new(|m| {
                    m.insert(int(1_000_000), val(1));
                }),
            ),
            (
                "key of another int type",
                Box::new(|m| {
                    m.insert(Key(Value::Int(100, IntTy::I32)), val(1));
                }),
            ),
            (
                "foreign key",
                Box::new(|m| {
                    m.insert(Key(Value::Float(0.5, FloatTy::F64)), val(1));
                }),
            ),
            (
                "value of another type",
                Box::new(|m| {
                    m.insert(int(3), Value::Int(1, IntTy::I32));
                }),
            ),
            (
                "new key with a value of another type",
                Box::new(|m| {
                    m.insert(int(40), Value::Bool(false));
                }),
            ),
            (
                "get_mut",
                Box::new(|m| {
                    assert!(m.get_mut(&int(3)).is_some());
                }),
            ),
            (
                "remove of a float key",
                Box::new(|m| {
                    assert!(m.remove(&Key(Value::Float(0.5, FloatTy::F64))).is_none());
                }),
            ),
        ];
        for (what, f) in cases {
            let mut m = filled(32);
            assert_eq!(m.store_kind(), "intdense", "{what}");
            let before: Vec<_> = m
                .iter()
                .map(|(k, v)| (k.into_owned(), Key(v.into_owned())))
                .collect();
            f(&mut m);
            assert_ne!(m.store_kind(), "intdense", "{what}");
            // every entry that was there is still there with its value (unless the case replaced it)
            if what != "value of another type" && what != "get_mut" {
                for (k, v) in &before {
                    same!(m.get_cloned(k).map(Key), Some(v.clone()), "{what}");
                }
            }
            // and the map is never promoted again, however it grows
            for i in 200..400 {
                m.insert(int(i), val(i));
            }
            assert_ne!(m.store_kind(), "intdense", "{what}");
        }
    }

    #[test]
    fn a_stored_zero_is_present_and_remove_clears_the_bit() {
        let mut m = KMap::hash_map(BTreeMap::new());
        for i in 0..16 {
            m.insert(int(i), val(0));
        }
        assert_eq!(m.store_kind(), "intdense");
        same!(m.get_cloned(&int(3)), Some(val(0)));
        same!(m.get_cloned(&int(40)), None::<Value>);
        assert!(m.contains_key(&int(3)) && !m.contains_key(&int(40)) && !m.contains_key(&int(-3)));
        same!(m.remove(&int(3)), Some(val(0)));
        assert!(!m.contains_key(&int(3)));
        same!(m.remove(&int(3)), None::<Value>);
        assert_eq!(m.len(), 15);
        same!(m.insert(int(3), val(0)), None::<Value>);
        assert_eq!(m.len(), 16);
        same!(m.insert(int(3), val(5)), Some(val(0)));
        same!(m.typed_get(3), Some(Some((5i64, IntTy::I64))));
        assert_eq!(m.typed_get(99), Some(None));
        assert_eq!(m.typed_get(-1), Some(None));
        assert_eq!(filled(3).typed_get(1), None);
        // iteration skips the removed key, forward and backward, across a word boundary of the bitmap
        let mut big = KMap::hash_map(BTreeMap::new());
        for i in 0..200 {
            big.insert(int(i), val(i));
        }
        for i in 60..70 {
            big.remove(&int(i));
        }
        assert_eq!(big.store_kind(), "intdense");
        let fwd: Vec<i128> = big
            .keys()
            .map(|k| if let Value::Int(i, _) = k.0 { i } else { -1 })
            .collect();
        let mut back: Vec<i128> = big
            .keys()
            .rev()
            .map(|k| if let Value::Int(i, _) = k.0 { i } else { -1 })
            .collect();
        back.reverse();
        assert_eq!(fwd, back);
        assert_eq!(fwd.len(), 190);
        assert!(fwd.windows(2).all(|w| w[0] < w[1]) && !fwd.contains(&64));
        // meeting in the middle from both ends
        let mut it = big.iter();
        let (mut lo, mut hi) = (0, 0);
        while let (Some(_), Some(_)) = (it.next(), it.next_back()) {
            lo += 1;
            hi += 1;
        }
        assert_eq!(lo + hi, 190 - 190 % 2);
    }

    #[test]
    fn clones_copy_on_write_and_clear_keeps_the_array_usable() {
        let rc = std::rc::Rc::new(filled(64));
        let mut other = rc.clone();
        std::rc::Rc::make_mut(&mut other).insert(int(5), val(-1));
        std::rc::Rc::make_mut(&mut other).remove(&int(6));
        same!(rc.get_cloned(&int(5)), Some(val(5 * 3 - 7)));
        same!(rc.get_cloned(&int(6)), Some(val(6 * 3 - 7)));
        assert_eq!((rc.len(), other.len()), (64, 63));
        let mut c = filled(64);
        let d = c.clone();
        c.clear();
        assert_eq!((c.len(), d.len()), (0, 64));
        assert_eq!(c.store_kind(), "intdense");
        same!(c.get_cloned(&int(3)), None::<Value>);
        c.insert(int(2), val(9));
        same!(
            (c.len(), c.get_cloned(&int(2)), c.iter().count()),
            (1, Some(val(9)), 1)
        );
    }

    #[test]
    fn literal_width_values_are_held_while_they_fit_an_i64() {
        let lit = |i: i128| Value::Int(i, IntTy::Lit);
        let mut m = KMap::hash_map(BTreeMap::new());
        for i in 0..16 {
            m.insert(int(i), lit(i));
        }
        assert_eq!(m.store_kind(), "intdense");
        same!(
            m.get_cloned(&int(4)).map(|v| format!("{v:?}")),
            Some(format!("{:?}", lit(4)))
        );
        m.set_int(4, IntTy::I64, lit(-9));
        assert_eq!(m.store_kind(), "intdense");
        m.insert(int(5), lit(1 << 70));
        assert_ne!(m.store_kind(), "intdense");
        same!(
            m.get_cloned(&int(5)).map(|v| format!("{v:?}")),
            Some(format!("{:?}", lit(1 << 70)))
        );
        same!(
            m.get_cloned(&int(4)).map(|v| format!("{v:?}")),
            Some(format!("{:?}", lit(-9)))
        );
    }

    #[test]
    fn a_demoted_array_is_not_promoted_again_even_when_it_keeps_filling() {
        let mut m = filled(32);
        assert!(m.get_mut(&int(3)).is_some());
        assert_eq!(m.store_kind(), "dense");
        for i in 32..300 {
            m.insert(int(i), val(i));
        }
        assert_eq!(m.store_kind(), "dense");
        assert_eq!(m.len(), 300);
    }

    #[test]
    fn set_int_of_an_absent_key_of_another_type_keeps_that_key_type() {
        let mut m = filled(32);
        m.remove(&int(5));
        m.set_int(5, IntTy::I32, val(1));
        assert_ne!(m.store_kind(), "intdense");
        let k = m.iter().find(|(k, _)| matches!(k.0, Value::Int(5, _))).map(|(k, _)| format!("{:?}", k.0));
        assert_eq!(k.as_deref(), Some("Int(5, I32)"));
    }

    #[test]
    fn the_bits_entry_takes_only_what_the_array_holds() {
        let mut m = filled(32);
        m.remove(&int(5));
        assert!(!m.set_int_bits(5, IntTy::I32, 1, IntTy::I64), "an absent slot needs the array's key type");
        assert!(!m.set_int_bits(5, IntTy::I64, 1, IntTy::I32), "the value type must be the array's");
        assert!(!m.set_int_bits(3, IntTy::I64, 1, IntTy::I32), "a present slot too");
        assert!(!m.set_int_bits(64, IntTy::I64, 1, IntTy::I64), "past the array's reach: the general path grows it");
        assert!(!m.set_int_bits(-1, IntTy::I64, 1, IntTy::I64));
        assert_eq!((m.len(), m.store_kind()), (31, "intdense"));
        assert!(m.set_int_bits(5, IntTy::I64, -4, IntTy::I64));
        assert!(m.set_int_bits(3, IntTy::I32, 9, IntTy::I64), "a present slot keeps its key whatever type the call names");
        same!((m.len(), m.get_cloned(&int(5)), m.get_cloned(&int(3))), (32, Some(val(-4)), Some(val(9))));
        assert!(!filled(4).set_int_bits(1, IntTy::I64, 1, IntTy::I64), "not a typed array");
    }
}
