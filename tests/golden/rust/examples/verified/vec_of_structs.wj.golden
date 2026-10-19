#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]

struct Person {
    name: String,
    age: i64,
}

fn main() {
    let mut people = vec![Person { name: String::from("Al"), age: 30_i64 }, Person { name: String::from("Cy"), age: 41_i64 }, Person { name: String::from("Bo"), age: 25_i64 }];
    people.sort_by_key(|p| p.age);
    for p in &people {
        println!("{} {}", p.name, p.age);
    }
    match people.max_by_key(|p| p.age) {
        Some(oldest) => println!("oldest: {}", oldest.name),
        None => println!("nobody"),
    }
}

