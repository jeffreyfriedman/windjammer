//! Enumerable collection methods (ADR-0018 feature 1, spec area COLL).
//!
//! The Windjammer names (`group_by`, `count_by`, ...) are declared once in the compiler's surface table
//! (`src/stdlib_surface/enumerable.rs`); the Rust backend renames a call `xs.group_by(f)` to
//! `xs.wj_group_by(f)` and inlines THIS FILE (std-only, no dependencies) as `mod wj_enumerable` into the
//! generated crate, so programs do not pay for the rest of the runtime. Keep it dependency-free.
//!
//! Two receivers:
//! * collections (`Vec`, slices, arrays, `HashSet`, ...; anything whose `&C` iterates `&T`): elements in
//!   results are owned clones (free for `Copy` types), closures receive `&T`;
//! * iterators: results hold the iterator's items, closures receive `&Item`.
//!
//! `Map` results use `std::collections::HashMap` (iteration order unspecified, as for every `Map`).
#![allow(dead_code)]

use std::collections::{HashMap, HashSet};
use std::hash::Hash;

/// Enumerable methods on any iterator.
pub trait WjIterExt: Iterator + Sized {
    fn wj_group_by<K: Eq + Hash, F: FnMut(&Self::Item) -> K>(
        self,
        mut f: F,
    ) -> HashMap<K, Vec<Self::Item>> {
        let mut out: HashMap<K, Vec<Self::Item>> = HashMap::new();
        for x in self {
            out.entry(f(&x)).or_default().push(x);
        }
        out
    }
    fn wj_count_by<K: Eq + Hash, F: FnMut(&Self::Item) -> K>(self, mut f: F) -> HashMap<K, i64> {
        let mut out: HashMap<K, i64> = HashMap::new();
        for x in self {
            *out.entry(f(&x)).or_insert(0) += 1;
        }
        out
    }
    fn wj_tally(self) -> HashMap<Self::Item, i64>
    where
        Self::Item: Eq + Hash,
    {
        let mut out: HashMap<Self::Item, i64> = HashMap::new();
        for x in self {
            *out.entry(x).or_insert(0) += 1;
        }
        out
    }
    fn wj_sum_by<N: std::iter::Sum<N>, F: FnMut(&Self::Item) -> N>(self, mut f: F) -> N {
        self.map(|x| f(&x)).sum()
    }
    fn wj_unique(self) -> Vec<Self::Item>
    where
        Self::Item: Eq + Hash + Clone,
    {
        let mut seen = HashSet::new();
        self.filter(|x| seen.insert(x.clone())).collect()
    }
    fn wj_unique_by<K: Eq + Hash, F: FnMut(&Self::Item) -> K>(self, mut f: F) -> Vec<Self::Item> {
        let mut seen = HashSet::new();
        self.filter(|x| seen.insert(f(x))).collect()
    }
    fn wj_chunk_by<K: PartialEq, F: FnMut(&Self::Item) -> K>(
        self,
        mut f: F,
    ) -> Vec<Vec<Self::Item>> {
        let mut out: Vec<Vec<Self::Item>> = Vec::new();
        let mut last: Option<K> = None;
        for x in self {
            let k = f(&x);
            match (&last, out.last_mut()) {
                (Some(prev), Some(run)) if *prev == k => run.push(x),
                _ => out.push(vec![x]),
            }
            last = Some(k);
        }
        out
    }
    fn wj_each_slice<N: TryInto<usize>>(self, n: N) -> Vec<Vec<Self::Item>> {
        let n: usize = n
            .try_into()
            .unwrap_or_else(|_| panic!("chunk size must be non-negative"));
        assert!(n != 0, "chunk size must be non-zero");
        let mut out: Vec<Vec<Self::Item>> = Vec::new();
        for x in self {
            match out.last_mut() {
                Some(run) if run.len() < n => run.push(x),
                _ => out.push(vec![x]),
            }
        }
        out
    }
    fn wj_partition_by<F: FnMut(&Self::Item) -> bool>(
        self,
        f: F,
    ) -> (Vec<Self::Item>, Vec<Self::Item>) {
        self.partition(f)
    }
    fn wj_intersperse(self, sep: Self::Item) -> Vec<Self::Item>
    where
        Self::Item: Clone,
    {
        let mut out = Vec::new();
        for (i, x) in self.enumerate() {
            if i > 0 {
                out.push(sep.clone());
            }
            out.push(x);
        }
        out
    }
    fn wj_zip_with<J: IntoIterator, R, F: FnMut(Self::Item, J::Item) -> R>(
        self,
        other: J,
        mut f: F,
    ) -> Vec<R> {
        self.zip(other).map(|(a, b)| f(a, b)).collect()
    }
    fn wj_index_of<Q: PartialEq<Self::Item>>(mut self, x: Q) -> Option<usize> {
        self.position(|y| x == y)
    }
    fn wj_sorted(self) -> Vec<Self::Item>
    where
        Self::Item: Ord,
    {
        let mut v: Vec<Self::Item> = self.collect();
        v.sort();
        v
    }
    fn wj_sorted_by_key<K: Ord, F: FnMut(&Self::Item) -> K>(self, f: F) -> Vec<Self::Item> {
        let mut v: Vec<Self::Item> = self.collect();
        v.sort_by_key(f);
        v
    }
    fn wj_sorted_by_key_desc<K: Ord, F: FnMut(&Self::Item) -> K>(self, f: F) -> Vec<Self::Item> {
        let mut v: Vec<Self::Item> = self.collect();
        sort_desc(&mut v, f);
        v
    }
    fn wj_to_list(self) -> Vec<Self::Item> {
        self.collect()
    }
}

impl<I: Iterator> WjIterExt for I {}

/// Stable descending sort by key: equal keys keep their original order.
fn sort_desc<T, K: Ord, F: FnMut(&T) -> K>(v: &mut [T], mut f: F) {
    v.sort_by(|a, b| f(b).cmp(&f(a)));
}

/// Enumerable methods on collections, returning owned (cloned) elements.
pub trait WjCollExt<T: Clone> {
    /// The elements as owned values (cloned lazily, one at a time).
    fn wj_items<'a>(&'a self) -> impl Iterator<Item = T> + 'a
    where
        T: 'a;

    fn wj_group_by<K: Eq + Hash, F: FnMut(&T) -> K>(&self, f: F) -> HashMap<K, Vec<T>> {
        self.wj_items().wj_group_by(f)
    }
    fn wj_count_by<K: Eq + Hash, F: FnMut(&T) -> K>(&self, f: F) -> HashMap<K, i64> {
        self.wj_items().wj_count_by(f)
    }
    fn wj_tally(&self) -> HashMap<T, i64>
    where
        T: Eq + Hash,
    {
        self.wj_items().wj_tally()
    }
    fn wj_sum_by<N: std::iter::Sum<N>, F: FnMut(&T) -> N>(&self, f: F) -> N {
        self.wj_items().wj_sum_by(f)
    }
    fn wj_unique(&self) -> Vec<T>
    where
        T: Eq + Hash,
    {
        self.wj_items().wj_unique()
    }
    fn wj_unique_by<K: Eq + Hash, F: FnMut(&T) -> K>(&self, f: F) -> Vec<T> {
        self.wj_items().wj_unique_by(f)
    }
    fn wj_chunk_by<K: PartialEq, F: FnMut(&T) -> K>(&self, f: F) -> Vec<Vec<T>> {
        self.wj_items().wj_chunk_by(f)
    }
    fn wj_each_slice<N: TryInto<usize>>(&self, n: N) -> Vec<Vec<T>> {
        self.wj_items().wj_each_slice(n)
    }
    fn wj_partition_by<F: FnMut(&T) -> bool>(&self, f: F) -> (Vec<T>, Vec<T>) {
        self.wj_items().wj_partition_by(f)
    }
    fn wj_intersperse(&self, sep: T) -> Vec<T> {
        self.wj_items().wj_intersperse(sep)
    }
    fn wj_zip_with<J: IntoIterator, R, F: FnMut(T, J::Item) -> R>(&self, other: J, f: F) -> Vec<R> {
        self.wj_items().wj_zip_with(other, f)
    }
    fn wj_index_of<Q: PartialEq<T>>(&self, x: Q) -> Option<usize> {
        self.wj_items().wj_index_of(x)
    }
    fn wj_sorted(&self) -> Vec<T>
    where
        T: Ord,
    {
        self.wj_items().wj_sorted()
    }
    fn wj_sorted_by_key<K: Ord, F: FnMut(&T) -> K>(&self, f: F) -> Vec<T> {
        self.wj_items().wj_sorted_by_key(f)
    }
    fn wj_sorted_by_key_desc<K: Ord, F: FnMut(&T) -> K>(&self, f: F) -> Vec<T> {
        self.wj_items().wj_sorted_by_key_desc(f)
    }
    fn wj_to_list(&self) -> Vec<T> {
        self.wj_items().collect()
    }
}

impl<C: ?Sized, T: Clone> WjCollExt<T> for C
where
    for<'a> &'a C: IntoIterator<Item = &'a T>,
{
    fn wj_items<'a>(&'a self) -> impl Iterator<Item = T> + 'a
    where
        T: 'a,
    {
        self.into_iter().cloned()
    }
}

/// In-place descending sort by key on lists (`xs.sort_by_key_desc(f)`).
pub trait WjSortDesc<T> {
    fn wj_sort_by_key_desc<K: Ord, F: FnMut(&T) -> K>(&mut self, f: F);
}

impl<T> WjSortDesc<T> for [T] {
    fn wj_sort_by_key_desc<K: Ord, F: FnMut(&T) -> K>(&mut self, f: F) {
        sort_desc(self, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn group_and_count() {
        let words = vec!["apple", "avocado", "banana", "blueberry", "cherry"];
        let g = words.wj_group_by(|w| w.chars().next().unwrap());
        assert_eq!(g[&'a'], vec!["apple", "avocado"]);
        assert_eq!(g[&'c'], vec!["cherry"]);
        let c = words.wj_count_by(|w| w.len());
        assert_eq!(c[&6], 2); // banana, cherry
        assert_eq!(vec![1, 1, 2].wj_tally()[&1], 2);
    }

    #[test]
    fn reshaping() {
        let v = vec![3, 1, 3, 2, 1];
        assert_eq!(v.wj_unique(), vec![3, 1, 2]);
        assert_eq!(v.wj_unique_by(|x| x % 2), vec![3, 2]);
        assert_eq!(
            vec![1, 1, 2, 3, 3].wj_chunk_by(|x| *x),
            vec![vec![1, 1], vec![2], vec![3, 3]]
        );
        assert_eq!(v.wj_each_slice(2), vec![vec![3, 1], vec![3, 2], vec![1]]);
        assert_eq!(vec![1, 2, 3].wj_intersperse(0), vec![1, 0, 2, 0, 3]);
        assert_eq!(
            vec![1, 2].wj_zip_with(vec![10, 20], |a, b| a + b),
            vec![11, 22]
        );
        assert_eq!(v.wj_index_of(2), Some(3));
        assert_eq!(v.wj_sorted(), vec![1, 1, 2, 3, 3]);
        assert_eq!(v.wj_partition_by(|x| *x > 1), (vec![3, 3, 2], vec![1, 1]));
        assert_eq!(v.wj_sum_by(|x| x * 10), 100);
    }

    #[test]
    fn descending_sort_is_stable() {
        let mut v = vec![(1, 'a'), (2, 'b'), (1, 'c')];
        v.wj_sort_by_key_desc(|p| p.0);
        assert_eq!(v, vec![(2, 'b'), (1, 'a'), (1, 'c')]);
        assert_eq!(
            vec![(1, 'a'), (2, 'b')].wj_sorted_by_key_desc(|p| p.0)[0],
            (2, 'b')
        );
    }

    #[test]
    fn iterator_receivers() {
        let g = (1..=6).wj_group_by(|x| x % 3);
        assert_eq!(g[&0], vec![3, 6]);
        assert_eq!((1..=4).wj_sum_by(|x| x * x), 30);
    }
}
