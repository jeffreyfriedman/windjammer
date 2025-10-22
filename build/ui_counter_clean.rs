use windjammer_runtime::ui::*;


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
#[inline]
fn decrement(&mut self) {
        self.count = self.count - 1;
}
#[inline]
fn reset(&mut self) {
        self.count = 0;
}
#[inline]
fn render(&self) -> VNode {
        h1("Counter App")
}
}

fn main() {
    let counter = Counter::new();
    println!("Counter app initialized with count: {}", counter.count)
}

