#[inline]
fn add(a: i64, b: i64) -> i64 {
    a + b
}

#[inline]
fn fibonacci(n: i64) -> i64 {
    if n <= 1_i64 {
        return n;
    }
    fibonacci(n - 1_i64) + fibonacci(n - 2_i64)
}

fn main() {
    let result = add(10_i64, 20_i64);
    println!("[add] {}", result);
    greet("Alice");
    let fib = fibonacci(10_i64);
    println!("[fib] {}", fib);
    println!("PASSED");
}

#[inline]
fn greet(name: &str) {
    println!("[greet] Hello {}", name);
}

