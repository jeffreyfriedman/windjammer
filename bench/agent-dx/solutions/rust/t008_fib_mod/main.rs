use std::io::Read;

// Fast doubling: returns (F(n), F(n+1)) mod m.
fn fib(n: u64, m: u128) -> (u128, u128) {
    if n == 0 {
        return (0, 1 % m);
    }
    let (a, b) = fib(n / 2, m);
    let c = a * ((2 * b + m - a) % m) % m;
    let d = (a * a + b * b) % m;
    if n % 2 == 0 {
        (c, d)
    } else {
        (d, (c + d) % m)
    }
}

fn main() {
    let mut s = String::new();
    std::io::stdin().read_to_string(&mut s).unwrap();
    let mut it = s.split_whitespace();
    let t: usize = it.next().unwrap().parse().unwrap();
    for _ in 0..t {
        let n: u64 = it.next().unwrap().parse().unwrap();
        let m: u128 = it.next().unwrap().parse().unwrap();
        println!("{}", fib(n, m).0 % m);
    }
}
