use std::collections::HashMap;
fn main() {
    let text: String = String::from("the cat and the dog and the cat");
    let mut freq: HashMap<String, i64> = HashMap::new();
    for w in text.split(" ") {
        freq.entry(w.to_string()).and_modify(|c| {
            *c += 1;
        }).or_insert(1);
    }
    let mut entries: Vec<(String, i64)> = freq.iter().map(|(k, v)| (k, *v)).collect::<Vec<(String, i64)>>();
    entries.sort_by(|a, b| b.1.cmp(&a.1.clone()).then(a.0.cmp(&b.0.clone())));
    for (w, c) in entries.iter().take(3_i64) {
        println!("{}: {}", w, c);
    }
}

