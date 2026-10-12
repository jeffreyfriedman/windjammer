#[inline]
fn fib(n: i64) -> u64 {
    let mut a: u64 = 0_u64;
    let mut b: u64 = 1_u64;
    for _ in 0_i64..n {
        let t = a + b;
        a = b;
        b = t;
    }
    a
}

fn main() {
    let mut parts: Vec<String> = vec![];
    for i in 0_i32..15_i32 {
        parts.push(fib((i as i64)).to_string());
    }
    println!("{}", parts.join(" "));
    println!("fib(50) = {}", fib(50_i64));
}

