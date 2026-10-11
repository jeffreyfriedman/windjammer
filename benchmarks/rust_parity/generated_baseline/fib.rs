#[inline]
fn fib(n: u64) -> u64 {
    if n < 2_u64 {
        return n;
    }
    fib(n - 1_u64) + fib(n - 2_u64)
}

fn main() {
    println!("{}", fib(38_u64));
}

