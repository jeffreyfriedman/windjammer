use std::collections::BTreeMap;
fn main() {
    let words = vec!["cat".to_string(), "bird".to_string(), "dog".to_string(), "horse".to_string()];
    let mut groups: BTreeMap<i64, Vec<String>> = BTreeMap::new();
    for w in words {
        groups.entry(w.len() as i64).or_insert(vec![]).push(w.to_string());
    }
    for (len, ws) in groups {
        println!("{}: [{}]", len, ws.join(", "));
    }
}

