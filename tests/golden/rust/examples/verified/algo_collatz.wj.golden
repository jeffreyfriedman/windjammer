#[inline]
fn collatz(start: i64) -> i64 {
    let mut n = start;
    let mut steps = 0_i64;
    while n != 1_i64 {
        if n % 2_i64 == 0_i64 {
            n /= 2;
        } else {
            n = 3_i64 * n + 1_i64;
        }
        steps += 1;
    }
    steps
}

fn main() {
    println!("collatz(27) = {}", collatz(27_i64));
    println!("collatz(97) = {}", collatz(97_i64));
}

