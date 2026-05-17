#[allow(unused_imports)]
use super::*;

use std::collections::HashMap as RustHashMap;
use std::collections::HashSet as RustHashSet;
use std::collections::BTreeMap as RustBTreeMap;
use std::collections::BTreeSet as RustBTreeSet;
use std::collections::VecDeque as RustVecDeque;
#[derive(Debug, Clone)]
#[repr(C)]
pub struct HashMap<K, V> {
    inner: RustHashMap<K, V>,
}

impl<K, V> HashMap<K, V> {
#[inline]
pub fn new() -> Self {
        HashMap { inner: RustHashMap::new() }
}
#[inline]
pub fn with_capacity(capacity: i64) -> Self {
        HashMap { inner: RustHashMap::with_capacity(capacity as usize) }
}
#[inline]
pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        self.inner.insert(key, value)
}
#[inline]
pub fn get(&self, key: &K) -> Option<&V> {
        self.inner.get(key)
}
#[inline]
pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        self.inner.get_mut(key)
}
#[inline]
pub fn remove(&mut self, key: &K) -> Option<V> {
        self.inner.remove(key)
}
#[inline]
pub fn contains_key(&self, key: &K) -> bool {
        self.inner.contains_key(key)
}
#[inline]
pub fn len(&self) -> i64 {
        self.inner.len() as i64
}
#[inline]
pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
}
#[inline]
pub fn clear(&mut self) {
        self.inner.clear();
}
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct HashSet<T> {
    inner: RustHashSet<T>,
}

impl<T> HashSet<T> {
#[inline]
pub fn new() -> Self {
        HashSet { inner: RustHashSet::new() }
}
#[inline]
pub fn with_capacity(capacity: i64) -> Self {
        HashSet { inner: RustHashSet::with_capacity(capacity as usize) }
}
#[inline]
pub fn insert(&mut self, value: T) -> bool {
        self.inner.insert(value)
}
#[inline]
pub fn remove(&mut self, value: &T) -> bool {
        self.inner.remove(value)
}
#[inline]
pub fn contains(&self, value: &T) -> bool {
        self.inner.contains(value)
}
#[inline]
pub fn len(&self) -> i64 {
        self.inner.len() as i64
}
#[inline]
pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
}
#[inline]
pub fn clear(&mut self) {
        self.inner.clear();
}
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct BTreeMap<K, V> {
    inner: RustBTreeMap<K, V>,
}

impl<K, V> BTreeMap<K, V> {
#[inline]
pub fn new() -> Self {
        BTreeMap { inner: RustBTreeMap::new() }
}
#[inline]
pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        self.inner.insert(key, value)
}
#[inline]
pub fn get(&self, key: &K) -> Option<&V> {
        self.inner.get(key)
}
#[inline]
pub fn remove(&mut self, key: &K) -> Option<V> {
        self.inner.remove(key)
}
#[inline]
pub fn contains_key(&self, key: &K) -> bool {
        self.inner.contains_key(key)
}
#[inline]
pub fn len(&self) -> i64 {
        self.inner.len() as i64
}
#[inline]
pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
}
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct BTreeSet<T> {
    inner: RustBTreeSet<T>,
}

impl<T> BTreeSet<T> {
#[inline]
pub fn new() -> Self {
        BTreeSet { inner: RustBTreeSet::new() }
}
#[inline]
pub fn insert(&mut self, value: T) -> bool {
        self.inner.insert(value)
}
#[inline]
pub fn remove(&mut self, value: &T) -> bool {
        self.inner.remove(value)
}
#[inline]
pub fn contains(&self, value: &T) -> bool {
        self.inner.contains(value)
}
#[inline]
pub fn len(&self) -> i64 {
        self.inner.len() as i64
}
#[inline]
pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
}
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct VecDeque<T> {
    inner: RustVecDeque<T>,
}

impl<T> VecDeque<T> {
#[inline]
pub fn new() -> Self {
        VecDeque { inner: RustVecDeque::new() }
}
#[inline]
pub fn with_capacity(capacity: i64) -> Self {
        VecDeque { inner: RustVecDeque::with_capacity(capacity as usize) }
}
#[inline]
pub fn push_front(&mut self, value: T) {
        self.inner.push_front(value);
}
#[inline]
pub fn push_back(&mut self, value: T) {
        self.inner.push_back(value);
}
#[inline]
pub fn pop_front(&mut self) -> Option<T> {
        self.inner.pop_front()
}
#[inline]
pub fn pop_back(&mut self) -> Option<T> {
        self.inner.pop_back()
}
#[inline]
pub fn len(&self) -> i64 {
        self.inner.len() as i64
}
#[inline]
pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
}
#[inline]
pub fn clear(&mut self) {
        self.inner.clear();
}
}

