#[inline]
fn apply(f: impl Fn(i64) -> i64, x: i64) -> i64 {
    f(x)
}

fn main() {
    let add_one = |x| x + 1_i64;
    println!("{}", add_one(10));
    let k = 5_i32;
    let add_k = |x| x + k;
    println!("{}", add_k(10));
    println!("{}", apply(|x| x - 1_i64, 8_i64));
}

