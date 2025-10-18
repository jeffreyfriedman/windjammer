struct Item {
    id: i64,
    name: String,
}

struct Container {
    items: Vec<Item>,
}

fn main() {
    let c = Container { items: vec![Item { id: 1, name: "First" }, Item { id: 2, name: "Second" }] };
    println!("{c.items[0].name}")
}

