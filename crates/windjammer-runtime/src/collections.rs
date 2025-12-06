//! Collection utilities
//!
//! Windjammer's `std::collections` module provides ergonomic functional
//! programming methods that work directly on collections without requiring
//! explicit `.iter()` and `.collect()` chains.
//!
//! # Examples
//! ```windjammer
//! use std::collections::*
//!
//! let numbers = vec![1, 2, 3, 4, 5]
//! let doubled = map(numbers, |x| x * 2)           // [2, 4, 6, 8, 10]
//! let evens = filter(numbers, |x| x % 2 == 0)     // [2, 4]
//! let sum = reduce(numbers, 0, |acc, x| acc + x)  // 15
//! ```

// Re-export standard collections for public use
pub use std::collections::HashMap;
pub use std::collections::HashSet;
pub use std::collections::VecDeque;

// ============================================================================
// FUNCTIONAL PROGRAMMING METHODS
// ============================================================================

/// Transform each element of a collection
///
/// # Example
/// ```windjammer
/// let numbers = vec![1, 2, 3]
/// let doubled = map(numbers, |x| x * 2)  // [2, 4, 6]
/// ```
pub fn map<T, R, F>(vec: Vec<T>, f: F) -> Vec<R>
where
    F: FnMut(T) -> R,
{
    vec.into_iter().map(f).collect()
}

/// Keep only elements that satisfy the predicate
///
/// # Example
/// ```windjammer
/// let numbers = vec![1, 2, 3, 4, 5]
/// let evens = filter(numbers, |x| x % 2 == 0)  // [2, 4]
/// ```
pub fn filter<T, F>(vec: Vec<T>, f: F) -> Vec<T>
where
    F: FnMut(&T) -> bool,
{
    vec.into_iter().filter(f).collect()
}

/// Reduce a collection to a single value
///
/// # Example
/// ```windjammer
/// let numbers = vec![1, 2, 3, 4, 5]
/// let sum = reduce(numbers, 0, |acc, x| acc + x)  // 15
/// ```
pub fn reduce<T, R, F>(vec: Vec<T>, initial: R, f: F) -> R
where
    F: FnMut(R, T) -> R,
{
    vec.into_iter().fold(initial, f)
}

/// Alias for reduce (fold)
pub fn fold<T, R, F>(vec: Vec<T>, initial: R, f: F) -> R
where
    F: FnMut(R, T) -> R,
{
    reduce(vec, initial, f)
}

/// Find the first element that satisfies the predicate
///
/// # Example
/// ```windjammer
/// let numbers = vec![1, 2, 3, 4, 5]
/// let first_even = find(numbers, |x| x % 2 == 0)  // Some(2)
/// ```
pub fn find<T, F>(vec: Vec<T>, mut f: F) -> Option<T>
where
    F: FnMut(&T) -> bool,
{
    vec.into_iter().find(|x| f(x))
}

/// Check if any element satisfies the predicate
///
/// # Example
/// ```windjammer
/// let numbers = vec![1, 2, 3, 4, 5]
/// let has_even = any(numbers, |x| x % 2 == 0)  // true
/// ```
pub fn any<T, F>(vec: Vec<T>, mut f: F) -> bool
where
    F: FnMut(&T) -> bool,
{
    vec.into_iter().any(|x| f(&x))
}

/// Check if all elements satisfy the predicate
///
/// # Example
/// ```windjammer
/// let numbers = vec![2, 4, 6, 8]
/// let all_even = all(numbers, |x| x % 2 == 0)  // true
/// ```
pub fn all<T, F>(vec: Vec<T>, mut f: F) -> bool
where
    F: FnMut(&T) -> bool,
{
    vec.into_iter().all(|x| f(&x))
}

/// Check if no elements satisfy the predicate
///
/// # Example
/// ```windjammer
/// let numbers = vec![1, 3, 5, 7]
/// let no_even = none(numbers, |x| x % 2 == 0)  // true
/// ```
pub fn none<T, F>(vec: Vec<T>, f: F) -> bool
where
    F: FnMut(&T) -> bool,
{
    !any(vec, f)
}

/// Take the first n elements
///
/// # Example
/// ```windjammer
/// let numbers = vec![1, 2, 3, 4, 5]
/// let first_three = take(numbers, 3)  // [1, 2, 3]
/// ```
pub fn take<T>(vec: Vec<T>, n: usize) -> Vec<T> {
    vec.into_iter().take(n).collect()
}

/// Skip the first n elements
///
/// # Example
/// ```windjammer
/// let numbers = vec![1, 2, 3, 4, 5]
/// let last_two = skip(numbers, 3)  // [4, 5]
/// ```
pub fn skip<T>(vec: Vec<T>, n: usize) -> Vec<T> {
    vec.into_iter().skip(n).collect()
}

/// Take elements while predicate is true
///
/// # Example
/// ```windjammer
/// let numbers = vec![1, 2, 3, 4, 5]
/// let small = take_while(numbers, |x| x < 4)  // [1, 2, 3]
/// ```
pub fn take_while<T, F>(vec: Vec<T>, f: F) -> Vec<T>
where
    F: FnMut(&T) -> bool,
{
    vec.into_iter().take_while(f).collect()
}

/// Skip elements while predicate is true
///
/// # Example
/// ```windjammer
/// let numbers = vec![1, 2, 3, 4, 5]
/// let big = skip_while(numbers, |x| x < 3)  // [3, 4, 5]
/// ```
pub fn skip_while<T, F>(vec: Vec<T>, f: F) -> Vec<T>
where
    F: FnMut(&T) -> bool,
{
    vec.into_iter().skip_while(f).collect()
}

/// Flatten a nested collection
///
/// # Example
/// ```windjammer
/// let nested = vec![vec![1, 2], vec![3, 4], vec![5]]
/// let flat = flatten(nested)  // [1, 2, 3, 4, 5]
/// ```
pub fn flatten<T>(vec: Vec<Vec<T>>) -> Vec<T> {
    vec.into_iter().flatten().collect()
}

/// Map and flatten in one step
///
/// # Example
/// ```windjammer
/// let words = vec!["hello", "world"]
/// let chars = flat_map(words, |s| s.chars().collect())
/// ```
pub fn flat_map<T, R, F>(vec: Vec<T>, f: F) -> Vec<R>
where
    F: FnMut(T) -> Vec<R>,
{
    vec.into_iter().flat_map(f).collect()
}

/// Zip two collections together
///
/// # Example
/// ```windjammer
/// let a = vec![1, 2, 3]
/// let b = vec!["a", "b", "c"]
/// let pairs = zip(a, b)  // [(1, "a"), (2, "b"), (3, "c")]
/// ```
pub fn zip<A, B>(a: Vec<A>, b: Vec<B>) -> Vec<(A, B)> {
    a.into_iter().zip(b).collect()
}

/// Add indices to elements
///
/// # Example
/// ```windjammer
/// let items = vec!["a", "b", "c"]
/// let indexed = enumerate(items)  // [(0, "a"), (1, "b"), (2, "c")]
/// ```
pub fn enumerate<T>(vec: Vec<T>) -> Vec<(usize, T)> {
    vec.into_iter().enumerate().collect()
}

/// Concatenate two collections
///
/// # Example
/// ```windjammer
/// let a = vec![1, 2, 3]
/// let b = vec![4, 5, 6]
/// let both = chain(a, b)  // [1, 2, 3, 4, 5, 6]
/// ```
pub fn chain<T>(a: Vec<T>, b: Vec<T>) -> Vec<T> {
    a.into_iter().chain(b).collect()
}

/// Partition elements by predicate
///
/// # Example
/// ```windjammer
/// let numbers = vec![1, 2, 3, 4, 5]
/// let (evens, odds) = partition(numbers, |x| x % 2 == 0)
/// // evens = [2, 4], odds = [1, 3, 5]
/// ```
pub fn partition<T, F>(vec: Vec<T>, mut f: F) -> (Vec<T>, Vec<T>)
where
    F: FnMut(&T) -> bool,
{
    vec.into_iter().partition(|x| f(x))
}

/// Group elements by key
///
/// # Example
/// ```windjammer
/// let words = vec!["apple", "ant", "banana", "bear"]
/// let by_first = group_by(words, |s| s.chars().next().unwrap())
/// // {'a': ["apple", "ant"], 'b': ["banana", "bear"]}
/// ```
pub fn group_by<T, K, F>(vec: Vec<T>, mut f: F) -> HashMap<K, Vec<T>>
where
    K: std::hash::Hash + Eq,
    F: FnMut(&T) -> K,
{
    let mut result: HashMap<K, Vec<T>> = HashMap::new();
    for item in vec {
        let key = f(&item);
        result.entry(key).or_default().push(item);
    }
    result
}

/// Count elements by key
///
/// # Example
/// ```windjammer
/// let items = vec!["a", "b", "a", "c", "a", "b"]
/// let counts = count_by(items, |x| x)
/// // {"a": 3, "b": 2, "c": 1}
/// ```
pub fn count_by<T, K, F>(vec: Vec<T>, mut f: F) -> HashMap<K, usize>
where
    K: std::hash::Hash + Eq,
    F: FnMut(&T) -> K,
{
    let mut result: HashMap<K, usize> = HashMap::new();
    for item in vec {
        let key = f(&item);
        *result.entry(key).or_default() += 1;
    }
    result
}

/// Sum all elements
///
/// # Example
/// ```windjammer
/// let numbers = vec![1, 2, 3, 4, 5]
/// let total = sum(numbers)  // 15
/// ```
pub fn sum<T>(vec: Vec<T>) -> T
where
    T: std::iter::Sum,
{
    vec.into_iter().sum()
}

/// Product of all elements
///
/// # Example
/// ```windjammer
/// let numbers = vec![1, 2, 3, 4, 5]
/// let total = product(numbers)  // 120
/// ```
pub fn product<T>(vec: Vec<T>) -> T
where
    T: std::iter::Product,
{
    vec.into_iter().product()
}

/// Find the minimum element
///
/// # Example
/// ```windjammer
/// let numbers = vec![3, 1, 4, 1, 5]
/// let smallest = min(numbers)  // Some(1)
/// ```
pub fn min<T: Ord>(vec: Vec<T>) -> Option<T> {
    vec.into_iter().min()
}

/// Find the maximum element
///
/// # Example
/// ```windjammer
/// let numbers = vec![3, 1, 4, 1, 5]
/// let largest = max(numbers)  // Some(5)
/// ```
pub fn max<T: Ord>(vec: Vec<T>) -> Option<T> {
    vec.into_iter().max()
}

/// Find minimum by key
pub fn min_by<T, K, F>(vec: Vec<T>, mut f: F) -> Option<T>
where
    K: Ord,
    F: FnMut(&T) -> K,
{
    vec.into_iter().min_by_key(|x| f(x))
}

/// Find maximum by key
pub fn max_by<T, K, F>(vec: Vec<T>, mut f: F) -> Option<T>
where
    K: Ord,
    F: FnMut(&T) -> K,
{
    vec.into_iter().max_by_key(|x| f(x))
}

/// Get first element
///
/// # Example
/// ```windjammer
/// let numbers = vec![1, 2, 3]
/// let first = first(numbers)  // Some(1)
/// ```
pub fn first<T>(vec: Vec<T>) -> Option<T> {
    vec.into_iter().next()
}

/// Get last element
///
/// # Example
/// ```windjammer
/// let numbers = vec![1, 2, 3]
/// let last = last(numbers)  // Some(3)
/// ```
pub fn last<T>(vec: Vec<T>) -> Option<T> {
    vec.into_iter().last()
}

/// Get nth element (0-indexed)
///
/// # Example
/// ```windjammer
/// let numbers = vec![1, 2, 3, 4, 5]
/// let third = nth(numbers, 2)  // Some(3)
/// ```
pub fn nth<T>(vec: Vec<T>, n: usize) -> Option<T> {
    vec.into_iter().nth(n)
}

/// Sort by key function
///
/// # Example
/// ```windjammer
/// let words = vec!["banana", "apple", "cherry"]
/// let by_len = sort_by(words, |s| s.len())  // ["apple", "banana", "cherry"]
/// ```
pub fn sort_by<T, K, F>(mut vec: Vec<T>, mut f: F) -> Vec<T>
where
    K: Ord,
    F: FnMut(&T) -> K,
{
    vec.sort_by_key(|x| f(x));
    vec
}

/// Reverse iterator
///
/// # Example
/// ```windjammer
/// let numbers = vec![1, 2, 3]
/// let rev = reversed(numbers)  // [3, 2, 1]
/// ```
pub fn reversed<T>(vec: Vec<T>) -> Vec<T> {
    vec.into_iter().rev().collect()
}

/// Create a range of numbers
///
/// # Example
/// ```windjammer
/// let nums = range(1, 5)  // [1, 2, 3, 4]
/// ```
pub fn range(start: i32, end: i32) -> Vec<i32> {
    (start..end).collect()
}

/// Create an inclusive range of numbers
///
/// # Example
/// ```windjammer
/// let nums = range_inclusive(1, 5)  // [1, 2, 3, 4, 5]
/// ```
pub fn range_inclusive(start: i32, end: i32) -> Vec<i32> {
    (start..=end).collect()
}

/// Repeat a value n times
///
/// # Example
/// ```windjammer
/// let zeros = repeat(0, 5)  // [0, 0, 0, 0, 0]
/// ```
pub fn repeat<T: Clone>(value: T, n: usize) -> Vec<T> {
    std::iter::repeat_n(value, n).collect()
}

// ============================================================================
// COLLECTION HELPERS (existing functionality)
// ============================================================================

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

/// Reverse a vector (in-place style, returns new vec)
pub fn reverse<T: Clone>(vec: &[T]) -> Vec<T> {
    let mut result = vec.to_vec();
    result.reverse();
    result
}

/// Sort a vector (in-place style, returns new vec)
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

    // ========== FUNCTIONAL PROGRAMMING TESTS ==========

    #[test]
    fn test_map() {
        let vec = vec![1, 2, 3];
        let result = map(vec, |x| x * 2);
        assert_eq!(result, vec![2, 4, 6]);
    }

    #[test]
    fn test_filter() {
        let vec = vec![1, 2, 3, 4, 5];
        let result = filter(vec, |x| x % 2 == 0);
        assert_eq!(result, vec![2, 4]);
    }

    #[test]
    fn test_reduce() {
        let vec = vec![1, 2, 3, 4, 5];
        let result = reduce(vec, 0, |acc, x| acc + x);
        assert_eq!(result, 15);
    }

    #[test]
    fn test_fold() {
        let vec = vec![1, 2, 3];
        let result = fold(vec, String::new(), |acc, x| format!("{}{}", acc, x));
        assert_eq!(result, "123");
    }

    #[test]
    fn test_find() {
        let vec = vec![1, 2, 3, 4, 5];
        assert_eq!(find(vec.clone(), |x| *x > 3), Some(4));
        assert_eq!(find(vec, |x| *x > 10), None);
    }

    #[test]
    fn test_any() {
        let vec = vec![1, 2, 3, 4, 5];
        assert!(any(vec.clone(), |x| *x > 3));
        assert!(!any(vec, |x| *x > 10));
    }

    #[test]
    fn test_all() {
        let vec = vec![2, 4, 6, 8];
        assert!(all(vec.clone(), |x| *x % 2 == 0));
        assert!(!all(vec, |x| *x > 5));
    }

    #[test]
    fn test_none() {
        let vec = vec![1, 3, 5, 7];
        assert!(none(vec.clone(), |x| *x % 2 == 0));
        assert!(!none(vec, |x| *x > 5));
    }

    #[test]
    fn test_take() {
        let vec = vec![1, 2, 3, 4, 5];
        assert_eq!(take(vec, 3), vec![1, 2, 3]);
    }

    #[test]
    fn test_skip() {
        let vec = vec![1, 2, 3, 4, 5];
        assert_eq!(skip(vec, 3), vec![4, 5]);
    }

    #[test]
    fn test_take_while() {
        let vec = vec![1, 2, 3, 4, 5];
        assert_eq!(take_while(vec, |x| *x < 4), vec![1, 2, 3]);
    }

    #[test]
    fn test_skip_while() {
        let vec = vec![1, 2, 3, 4, 5];
        assert_eq!(skip_while(vec, |x| *x < 3), vec![3, 4, 5]);
    }

    #[test]
    fn test_flatten() {
        let vec = vec![vec![1, 2], vec![3, 4], vec![5]];
        assert_eq!(flatten(vec), vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_flat_map() {
        let vec = vec![1, 2, 3];
        let result = flat_map(vec, |x| vec![x, x * 10]);
        assert_eq!(result, vec![1, 10, 2, 20, 3, 30]);
    }

    #[test]
    fn test_zip() {
        let a = vec![1, 2, 3];
        let b = vec!["a", "b", "c"];
        assert_eq!(zip(a, b), vec![(1, "a"), (2, "b"), (3, "c")]);
    }

    #[test]
    fn test_enumerate() {
        let vec = vec!["a", "b", "c"];
        assert_eq!(enumerate(vec), vec![(0, "a"), (1, "b"), (2, "c")]);
    }

    #[test]
    fn test_chain() {
        let a = vec![1, 2, 3];
        let b = vec![4, 5, 6];
        assert_eq!(chain(a, b), vec![1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn test_partition() {
        let vec = vec![1, 2, 3, 4, 5];
        let (evens, odds) = partition(vec, |x| x % 2 == 0);
        assert_eq!(evens, vec![2, 4]);
        assert_eq!(odds, vec![1, 3, 5]);
    }

    #[test]
    fn test_group_by() {
        let vec = vec!["apple", "ant", "banana", "bear"];
        let groups = group_by(vec, |s| s.chars().next().unwrap());
        assert_eq!(groups.get(&'a').unwrap().len(), 2);
        assert_eq!(groups.get(&'b').unwrap().len(), 2);
    }

    #[test]
    fn test_count_by() {
        let vec = vec!["a", "b", "a", "c", "a", "b"];
        let counts = count_by(vec, |x| *x);
        assert_eq!(counts.get(&"a"), Some(&3));
        assert_eq!(counts.get(&"b"), Some(&2));
        assert_eq!(counts.get(&"c"), Some(&1));
    }

    #[test]
    fn test_sum() {
        let vec = vec![1, 2, 3, 4, 5];
        assert_eq!(sum(vec), 15);
    }

    #[test]
    fn test_product() {
        let vec = vec![1, 2, 3, 4, 5];
        assert_eq!(product(vec), 120);
    }

    #[test]
    fn test_min_max() {
        let vec = vec![3, 1, 4, 1, 5];
        assert_eq!(min(vec.clone()), Some(1));
        assert_eq!(max(vec), Some(5));
    }

    #[test]
    fn test_min_max_by() {
        let vec = vec!["aa", "bbb", "c"];
        assert_eq!(min_by(vec.clone(), |s| s.len()), Some("c"));
        assert_eq!(max_by(vec, |s| s.len()), Some("bbb"));
    }

    #[test]
    fn test_first_last() {
        let vec = vec![1, 2, 3];
        assert_eq!(first(vec.clone()), Some(1));
        assert_eq!(last(vec), Some(3));
    }

    #[test]
    fn test_nth() {
        let vec = vec![1, 2, 3, 4, 5];
        assert_eq!(nth(vec.clone(), 2), Some(3));
        assert_eq!(nth(vec, 10), None);
    }

    #[test]
    fn test_sort_by() {
        let vec = vec!["banana", "apple", "cherry"];
        let result = sort_by(vec, |s| s.len());
        assert_eq!(result, vec!["apple", "banana", "cherry"]);
    }

    #[test]
    fn test_reversed() {
        let vec = vec![1, 2, 3];
        assert_eq!(reversed(vec), vec![3, 2, 1]);
    }

    #[test]
    fn test_range() {
        assert_eq!(range(1, 5), vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_range_inclusive() {
        assert_eq!(range_inclusive(1, 5), vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_repeat() {
        assert_eq!(repeat(0, 5), vec![0, 0, 0, 0, 0]);
    }

    // ========== COLLECTION HELPER TESTS ==========

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
