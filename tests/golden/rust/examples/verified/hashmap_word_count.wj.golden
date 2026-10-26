use std::collections::HashMap;
fn main() {
    let text: String = String::from("the cat and the dog and the cat");
    let mut counts: HashMap<String, i64> = HashMap::new();
    for w in text.split_whitespace() {
        *counts.entry(w.to_string()).or_insert(0) += 1;
    }
    println!("the {}", counts["the"]);
    println!("cat {}", counts["cat"]);
    println!("sat {}", counts.get("sat").copied().unwrap_or(1));
}

