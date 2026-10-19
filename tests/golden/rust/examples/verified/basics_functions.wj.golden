#[inline]
fn add(a: i64, b: i64) -> i64 {
    a + b
}

#[inline]
fn square(x: i64) -> i64 {
    x * x
}

#[inline]
fn factorial(n: i64) -> i64 {
    if n <= 1_i64 {
        1_i64
    } else {
        n * factorial(n - 1_i64)
    }
}

fn main() {
    println!("{}", add(3_i64, 4_i64));
    println!("{}", square(5_i64));
    println!("{}", factorial(5_i64));
}

