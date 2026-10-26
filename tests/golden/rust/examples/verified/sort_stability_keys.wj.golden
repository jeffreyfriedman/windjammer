fn main() {
    let mut v = vec!["bb".to_string(), "aa".to_string(), "c".to_string(), "dd".to_string()];
    v.sort_by_key(|s| s.len());
    v.reverse();
    v.sort_by_key(|s| 0_i64 - s.len() as i64);
    println!("[{}]", v.join(", "));
}

