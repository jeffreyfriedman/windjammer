#[inline]
fn steps(start: u64) -> u64 {
    let mut n = start;
    let mut s: u64 = 0_u64;
    while n != 1_u64 {
        if n % 2_u64 == 0_u64 {
            n /= 2;
        } else {
            n = 3_u64 * n + 1_u64;
        }
        s += 1;
    }
    s
}

fn main() {
    let mut best: u64 = 0_u64;
    let mut arg: u64 = 0_u64;
    for n in 1_u64..3000000_u64 {
        let s = steps(n);
        if s > best {
            best = s;
            arg = n;
        }
    }
    println!("{} {}", arg, best);
}

