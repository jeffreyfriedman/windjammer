use std::collections::HashMap;
fn main() {
    let mut m: HashMap<String, i64> = HashMap::new();
    m.insert(String::from("x"), 1_i64);
    println!("{:?}", m.get("x").copied());
    m.insert(String::from("x"), 11_i64);
    println!("{}", m["x"]);
    m.remove("x");
    println!("{:?}", m.get("x").copied());
    m.insert(String::from("y"), 1_i64);
    println!("{}", m.len());
}

