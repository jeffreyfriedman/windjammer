// Rust twin of bench/interp/hashmap.wj: counting keys, then lookups.
use std::collections::HashMap;

fn main() {
    let mut m: HashMap<i64, i64> = HashMap::new();
    for i in 0..200_000_i64 {
        *m.entry((i * 7919) % 10007).or_insert(0) += 1;
    }
    let best = (0..10007_i64).filter_map(|j| m.get(&j).copied()).max().unwrap_or(0).max(0);
    println!("{} {}", m.len(), best);
}
