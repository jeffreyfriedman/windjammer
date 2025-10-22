struct Counter {
    count: i64,
    name: String,
}

impl Counter {
#[inline]
fn get_count(&self) -> i64 {
        self.count
}
#[inline]
fn display(&self) {
        println!("Counter: {} = {}", self.name, self.count)
}
#[inline]
fn increment(&mut self) {
        self.count += 1;
}
#[inline]
fn decrement(&mut self) {
        self.count -= 1;
}
#[inline]
fn set_name(&mut self, new_name: &str) {
        self.name = new_name.to_string();
}
#[inline]
fn create_default() -> Self {
        Self { count: 0, name: "Default".to_string() }
}
}

fn main() {
    let mut counter = Counter { count: 0, name: "Test".to_string() };
    println!("Initial: {}", counter.get_count());
    counter.increment();
    counter.increment();
    println!("After increment: {}", counter.get_count());
    counter.set_name("Updated");
    counter.display()
}

