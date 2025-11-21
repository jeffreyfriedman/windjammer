//! Collection utilities
//!
//! Windjammer's `std::collections` module maps to these functions.
//! Note: Most collections are built into Rust std, this provides helpers.

// Re-export standard collections for public use
pub use std::collections::HashMap;
pub use std::collections::HashSet;
pub use std::collections::VecDeque;

/// Create a new HashMap
pub fn new_map<K, V>() -> HashMap<K, V>
where
    K: std::hash::Hash + Eq,
{
    HashMap::new()
}

/// Create a new HashSet
pub fn new_set<T>() -> HashSet<T>
where
    T: std::hash::Hash + Eq,
{
    HashSet::new()
}

/// Create a new VecDeque
pub fn new_deque<T>() -> VecDeque<T> {
    VecDeque::new()
}

/// Check if slice contains element
pub fn contains<T: PartialEq>(slice: &[T], item: &T) -> bool {
    slice.contains(item)
}

/// Find index of element in slice
pub fn index_of<T: PartialEq>(slice: &[T], item: &T) -> Option<usize> {
    slice.iter().position(|x| x == item)
}

/// Reverse a vector
pub fn reverse<T: Clone>(vec: &[T]) -> Vec<T> {
    let mut result = vec.to_vec();
    result.reverse();
    result
}

/// Sort a vector
pub fn sort<T: Clone + Ord>(vec: &[T]) -> Vec<T> {
    let mut result = vec.to_vec();
    result.sort();
    result
}

/// Remove duplicates from vector
pub fn unique<T: Clone + Eq + std::hash::Hash>(vec: &[T]) -> Vec<T> {
    let set: HashSet<_> = vec.iter().cloned().collect();
    set.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contains() {
        let vec = vec![1, 2, 3, 4, 5];
        assert!(contains(&vec, &3));
        assert!(!contains(&vec, &10));
    }

    #[test]
    fn test_index_of() {
        let vec = vec!["a", "b", "c"];
        assert_eq!(index_of(&vec, &"b"), Some(1));
        assert_eq!(index_of(&vec, &"z"), None);
    }

    #[test]
    fn test_reverse() {
        let vec = vec![1, 2, 3];
        assert_eq!(reverse(&vec), vec![3, 2, 1]);
    }

    #[test]
    fn test_sort() {
        let vec = vec![3, 1, 2];
        assert_eq!(sort(&vec), vec![1, 2, 3]);
    }

    #[test]
    fn test_unique() {
        let vec = vec![1, 2, 2, 3, 3, 3];
        let result = unique(&vec);
        assert_eq!(result.len(), 3);
        assert!(result.contains(&1));
        assert!(result.contains(&2));
        assert!(result.contains(&3));
    }
}
