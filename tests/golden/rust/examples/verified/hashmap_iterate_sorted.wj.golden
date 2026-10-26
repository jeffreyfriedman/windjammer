use std::collections::HashMap;
fn main() {
    let mut stock: HashMap<String, i64> = HashMap::new();
    stock.insert(String::from("pear"), 5_i64);
    stock.insert(String::from("apple"), 3_i64);
    stock.insert(String::from("kiwi"), 7_i64);
    let mut keys: Vec<String> = stock.keys().cloned().collect::<Vec<String>>();
    keys.sort();
    for k in keys {
        println!("{}={}", k, stock[&*k]);
    }
}

