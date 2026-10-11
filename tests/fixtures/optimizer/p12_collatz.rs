// fixture: p12_collatz
struct Rng {
    state: u64,
}

impl Rng {
    #[inline]
    fn new(seed: u64) -> Rng {
        Rng { state: seed.wrapping_mul(0x9E3779B97F4A7C15) | 1 }
    }

    #[inline]
    fn next(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
}

fn fold(h: u64, v: u64) -> u64 {
    (h ^ v).wrapping_mul(0x100000001b3)
}

fn seed_arg() -> u64 {
    std::env::args().nth(1).unwrap().parse().unwrap()
}

fn steps(mut n: u64) -> u64 {
    let mut s = 0_u64;
    while n != 1_u64 {
        if n % 2_u64 == 0_u64 {
            n = n / 2_u64;
        } else {
            n = 3_u64 * n + 1_u64;
        }
        s += 1_u64;
    }
    s
}

fn main() {
    let seed = seed_arg();
    let mut best = 0_u64;
    let mut arg = 0_u64;
    for n in 1_u64..(3000_u64 + seed * 100_u64) {
        let s = steps(n);
        if s > best {
            best = s;
            arg = n;
        }
    }
    println!("{} {}", arg, best);
}
