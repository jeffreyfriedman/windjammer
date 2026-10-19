#[derive(Debug, Clone, Default)]

struct Stack<T> {
    items: Vec<T>,
}

impl<T> Stack<T> {
#[inline]
fn new() -> Stack<T> {
        Stack { items: vec![] }
}
#[inline]
fn push(&mut self, x: T) {
        self.items.push(x);
}
#[inline]
fn pop(&mut self) -> Option<T> {
        self.items.pop()
}
#[inline]
fn len(&self) -> i64 {
        self.items.len() as i64
}
}

fn main() {
    let mut s: Stack<i64> = Stack::new();
    s.push(10);
    s.push(20);
    s.push(30);
    println!("{}", s.len() - 1_i64);
    println!("{:?}", s.pop());
    s.pop();
    s.pop();
    println!("{:?}", s.pop());
}

