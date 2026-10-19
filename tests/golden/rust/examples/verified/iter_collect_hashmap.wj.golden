use std::collections::HashMap;
fn main() {
    let words = vec!["a".to_string(), "bb".to_string(), "ccc".to_string()];
    let lens: HashMap<String, i64> = words.iter().map(|w| (w.to_string(), w.len())).collect::<HashMap<String, i64>>();
    println!("{}", lens.len());
    println!("{}", lens["bb"]);
    println!("{}", lens["ccc"]);
}

