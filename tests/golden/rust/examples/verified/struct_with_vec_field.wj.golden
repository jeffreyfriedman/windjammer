#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]

struct Item {
    name: String,
    price: i64,
}

#[derive(Debug, Clone, PartialEq, Default)]

struct Cart {
    items: Vec<Item>,
}

impl Cart {
#[inline]
fn new() -> Cart {
        Cart { items: vec![] }
}
#[inline]
fn add(&mut self, name: String, price: i64) {
        self.items.push(Item { name, price });
}
#[inline]
fn total(&self) -> i64 {
        self.items.as_ref().iter().map(|i| i.price).sum()
}
#[inline]
fn cheapest(&self) -> String {
        match self.items.iter().min_by_key(|i| i.price) {
            Some(i) => i.name,
            None => String::new(),
        }
}
}

fn main() {
    let mut cart = Cart::new();
    cart.add("book".to_string(), 30_i64);
    cart.add("pen".to_string(), 5_i64);
    cart.add("lamp".to_string(), 25_i64);
    println!("{} items, total {}", cart.items.len(), cart.total());
    println!("cheapest: {}", cart.cheapest());
}

