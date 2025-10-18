





use std::collections::HashMap as RustHashMap;

use std::collections::HashSet as RustHashSet;

use std::collections::BTreeMap as RustBTreeMap;

use std::collections::BTreeSet as RustBTreeSet;

use std::collections::VecDeque as RustVecDeque;


struct HashMap<K, V> {
    inner: RustHashMap<K, V>,
}

impl<K, V> HashMap<K, V> {
#[inline]
fn new() -> Self {
        HashMap { inner: RustHashMap::new() }
}
#[inline]
fn with_capacity(capacity: i64) -> Self {
        HashMap { inner: RustHashMap::with_capacity(capacity as usize) }
}
#[inline]
fn insert(&mut self, key: &K, value: &V) -> Option<V> {
        self.inner::insert(key, value)
}
#[inline]
fn get(&self, key: &K) -> Option<&V> {
        self.inner::get(key)
}
#[inline]
fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        self.inner::get_mut(key)
}
#[inline]
fn remove(&mut self, key: &K) -> Option<V> {
        self.inner::remove(key)
}
#[inline]
fn contains_key(&self, key: &K) -> bool {
        self.inner::contains_key(key)
}
#[inline]
fn len(&self) -> i64 {
        self.inner::len() as i64
}
#[inline]
fn is_empty(&self) -> bool {
        self.inner::is_empty()
}
#[inline]
fn clear(&mut self) {
        self.inner::clear()
}
}

struct HashSet<T> {
    inner: RustHashSet<T>,
}

impl<T> HashSet<T> {
#[inline]
fn new() -> Self {
        HashMap { inner: RustHashMap::new() }
}
#[inline]
fn with_capacity(capacity: i64) -> Self {
        HashMap { inner: RustHashMap::with_capacity(capacity as usize) }
}
#[inline]
fn insert(&mut self, key: &K, value: &V) -> Option<V> {
        self.inner::insert(key, value)
}
#[inline]
fn remove(&mut self, key: &K) -> Option<V> {
        self.inner::remove(key)
}
#[inline]
fn contains(&self, value: &T) -> bool {
        self.inner::contains(value)
}
#[inline]
fn len(&self) -> i64 {
        self.inner::len() as i64
}
#[inline]
fn is_empty(&self) -> bool {
        self.inner::is_empty()
}
#[inline]
fn clear(&mut self) {
        self.inner::clear()
}
}

struct BTreeMap<K, V> {
    inner: RustBTreeMap<K, V>,
}

impl<K, V> BTreeMap<K, V> {
#[inline]
fn new() -> Self {
        HashMap { inner: RustHashMap::new() }
}
#[inline]
fn insert(&mut self, key: &K, value: &V) -> Option<V> {
        self.inner::insert(key, value)
}
#[inline]
fn get(&self, key: &K) -> Option<&V> {
        self.inner::get(key)
}
#[inline]
fn remove(&mut self, key: &K) -> Option<V> {
        self.inner::remove(key)
}
#[inline]
fn contains_key(&self, key: &K) -> bool {
        self.inner::contains_key(key)
}
#[inline]
fn len(&self) -> i64 {
        self.inner::len() as i64
}
#[inline]
fn is_empty(&self) -> bool {
        self.inner::is_empty()
}
}

struct BTreeSet<T> {
    inner: RustBTreeSet<T>,
}

impl<T> BTreeSet<T> {
#[inline]
fn new() -> Self {
        HashMap { inner: RustHashMap::new() }
}
#[inline]
fn insert(&mut self, key: &K, value: &V) -> Option<V> {
        self.inner::insert(key, value)
}
#[inline]
fn remove(&mut self, key: &K) -> Option<V> {
        self.inner::remove(key)
}
#[inline]
fn contains(&self, value: &T) -> bool {
        self.inner::contains(value)
}
#[inline]
fn len(&self) -> i64 {
        self.inner::len() as i64
}
#[inline]
fn is_empty(&self) -> bool {
        self.inner::is_empty()
}
}

struct VecDeque<T> {
    inner: RustVecDeque<T>,
}

impl<T> VecDeque<T> {
#[inline]
fn new() -> Self {
        HashMap { inner: RustHashMap::new() }
}
#[inline]
fn with_capacity(capacity: i64) -> Self {
        HashMap { inner: RustHashMap::with_capacity(capacity as usize) }
}
#[inline]
fn push_front(&mut self, value: &T) {
        self.inner::push_front(value)
}
#[inline]
fn push_back(&mut self, value: &T) {
        self.inner::push_back(value)
}
#[inline]
fn pop_front(&mut self) -> Option<T> {
        self.inner::pop_front()
}
#[inline]
fn pop_back(&mut self) -> Option<T> {
        self.inner::pop_back()
}
#[inline]
fn len(&self) -> i64 {
        self.inner::len() as i64
}
#[inline]
fn is_empty(&self) -> bool {
        self.inner::is_empty()
}
#[inline]
fn clear(&mut self) {
        self.inner::clear()
}
}

