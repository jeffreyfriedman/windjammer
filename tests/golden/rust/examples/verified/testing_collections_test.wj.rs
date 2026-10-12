use windjammer_runtime::test::*;

use std::collections::HashMap;
#[inline]
fn count_words(text: &str) -> HashMap<String, i64> {
    let mut m: HashMap<String, i64> = HashMap::new();
    for w in text.split_whitespace() {
        m.entry(w.to_string()).and_modify(|c| {
            *c += 1;
        }).or_insert(1);
    }
    m
}

#[test]
fn counts_repeats() {
    let m: HashMap<String, i64> = count_words("a b a c a");
    assert_eq!(m["a"], 3);
    assert_eq!(m["b"], 1);
    assert_eq!(m.len(), 3_usize);
}

#[test]
fn sorts_and_dedups() {
    let mut v = vec![3_i32, 1_i32, 3_i32, 2_i32, 1_i32];
    v.sort();
    v.dedup();
    assert_eq!(v, [1_i64, 2_i64, 3_i64]);
}

