struct Box<T> {
    value: T,
}

impl<T> Box<T> {
fn new(value: &T) -> Box<T> {
        Box { value: value }
}
fn get(&self) -> &T {
        &self.value
}
}

fn identity<T>(x: &T) -> T {
    x
}

fn swap<A, B>(a: &A, b: &B) -> (B, A) {
    (b, a)
}

fn main() {
    let x = identity(&42);
    println!("identity(42) = {}", x);
    let s = identity(&"hello");
    println!("identity(string) = {}", s);
    println!("Generic type parameters work! ✓")
}

