struct Person {
    name: String,
    scores: Vec<i64>,
}

fn main() {
    let p = Person { name: "Alice", scores: vec![90, 85, 95] };
    println!("Person: {p:?}")
}

