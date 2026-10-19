use std::collections::HashSet;
fn main() {
    let mut s: HashSet<i64> = HashSet::new();
    s.insert(1_i64);
    s.insert(2_i64);
    s.insert(2_i64);
    s.insert(3_i64);
    println!("{}", s.len());
    println!("{}", s.contains(2_i64));
    println!("{}", s.contains(9_i64));
    let other: HashSet<i64> = [3, 4, 5].iter().cloned().collect::<HashSet<i64>>();
    let mut u: Vec<i64> = s.union(other.clone()).cloned().collect::<Vec<i64>>();
    u.sort();
    println!("{:?}", u);
    let mut i: Vec<i64> = s.intersection(other.clone()).cloned().collect::<Vec<i64>>();
    i.sort();
    println!("{:?}", i);
}

