use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;

fn main() {
    let mut s = String::new();
    std::io::stdin().read_to_string(&mut s).unwrap();
    let mut groups: BTreeMap<Vec<char>, BTreeSet<String>> = BTreeMap::new();
    for w in s.split_whitespace() {
        let lw = w.to_lowercase();
        let mut key: Vec<char> = lw.chars().collect();
        key.sort();
        groups.entry(key).or_default().insert(lw);
    }
    let mut lines: Vec<Vec<String>> = groups
        .into_values()
        .filter(|g| g.len() >= 2)
        .map(|g| g.into_iter().collect())
        .collect();
    if lines.is_empty() {
        println!("none");
        return;
    }
    lines.sort();
    for l in lines {
        println!("{}", l.join(" "));
    }
}
