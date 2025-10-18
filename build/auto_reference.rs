#[inline]
fn double(x: i64) -> i64 {
    x * 2
}

#[inline]
fn greet(name: &String) {
    println!("Hello, {}", name)
}

fn main() {
    let x = 5;
    let result = double(x);
    let name = "Alice";
    greet(&name)
}

