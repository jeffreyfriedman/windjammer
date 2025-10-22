struct Counter {
    count: i64,
}

// Component trait implementation for Counter
// TODO: Implement Component trait

impl Counter {
#[inline]
fn new(&self) -> Self {
        Self { count: 0 }
}
#[inline]
fn increment(&mut self) {
        self.count = self.count + 1;
}
}

fn main() {
    let mut counter = Counter::new();
    counter.increment();
    println!("Count: {}", counter.count)
}

