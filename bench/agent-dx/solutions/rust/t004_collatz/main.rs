use std::io::Read;

fn collatz(mut n: i64) -> (u64, i64) {
    let mut steps = 0;
    let mut peak = n;
    while n != 1 {
        n = if n % 2 == 0 { n / 2 } else { 3 * n + 1 };
        peak = peak.max(n);
        steps += 1;
    }
    (steps, peak)
}

fn main() {
    let mut s = String::new();
    std::io::stdin().read_to_string(&mut s).unwrap();
    let mut it = s.split_whitespace();
    let t: usize = it.next().unwrap().parse().unwrap();
    for _ in 0..t {
        let n: i64 = it.next().unwrap().parse().unwrap();
        let (steps, peak) = collatz(n);
        println!("{} {}", steps, peak);
    }
}
