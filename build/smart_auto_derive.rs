#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
struct Point {
    x: i64,
    y: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
struct User {
    name: String,
    age: i64,
}

#[derive(Debug, Clone, Default)]
struct Container {
    items: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Config {
    host: String,
    port: i64,
}

fn main() {
    let p = Point { x: 1, y: 2 };
    let u = User { name: "Alice", age: 30 };
    println!("Point: ({}, {})", p.x, p.y)
}

