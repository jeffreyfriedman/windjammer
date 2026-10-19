#[inline]
fn make_adder(n: i64) -> impl Fn(i64) -> i64 {
    move |x| x + n
}

#[inline]
fn compose(f: impl Fn(i64) -> i64, g: impl Fn(i64) -> i64, x: i64) -> i64 {
    g(f(x))
}

fn main() {
    let add5 = make_adder(5_i64);
    println!("{}", add5(10));
    println!("{}", compose(|x| x + 1_i64, |x| x * 2_i64, 3_i64));
}

