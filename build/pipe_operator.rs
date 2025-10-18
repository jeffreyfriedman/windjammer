#[inline]
fn double(x: i64) -> i64 {
    x * 2
}

#[inline]
fn add_ten(x: i64) -> i64 {
    x + 10
}

fn main() {
    let result = add_ten(double(5));
    println!("Result: {}", result)
}

