fn steps(mut n: u64) -> u64 {
    let mut s = 0;
    while n != 1 {
        n = if n % 2 == 0 { n / 2 } else { 3 * n + 1 };
        s += 1;
    }
    s
}

fn main() {
    let (mut best, mut arg) = (0, 0);
    for n in 1..3_000_000u64 {
        let s = steps(n);
        if s > best {
            best = s;
            arg = n;
        }
    }
    println!("{} {}", arg, best);
}
