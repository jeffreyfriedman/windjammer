use std::collections::BTreeMap;
fn main() {
    let mut m: BTreeMap<i64, String> = BTreeMap::new();
    m.insert(3_i64, String::from("three"));
    m.insert(1_i64, String::from("one"));
    m.insert(2_i64, String::from("two"));
    for (k, v) in &m {
        println!("{}:{}", k, v);
    }
    match m.keys().next() {
        Some(first) => println!("first={}", first),
        None => println!("empty"),
    }
}

