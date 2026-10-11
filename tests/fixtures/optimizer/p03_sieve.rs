// fixture: p03_sieve
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

fn sieve(n: usize) -> Vec<bool> {
    let mut flags: Vec<bool> = Vec::new();
    for _i in 0_usize..n + 1_usize {
        flags.push(true);
    }
    flags[0] = false;
    flags[1] = false;
    let mut i: usize = 2_usize;
    while i * i <= n {
        if flags[i] {
            let mut j = i * i;
            while j <= n {
                flags[j] = false;
                j += i;
            }
        }
        i += 1;
    }
    flags
}

fn main() {
    let seed = seed_arg();
    let n = 20000_usize + (seed as usize) * 1000_usize;
    let flags = sieve(n);
    let mut h: u64 = 0xcbf29ce484222325;
    let mut count = 0_u64;
    for i in 0_usize..n + 1_usize {
        if flags[i] {
            count += 1;
            h = fold(h, i as u64);
        }
    }
    println!("{} {}", count, h);
}
