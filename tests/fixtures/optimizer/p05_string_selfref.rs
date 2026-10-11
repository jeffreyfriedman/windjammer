// fixture: p05_string_selfref
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

fn main() {
    let seed = seed_arg();
    let mut out = String::from("a");
    for _i in 0_usize..8_usize {
        out = format!("{}{}", out, out.len());
    }
    let mut t = String::new();
    for i in 0_u64..30_u64 {
        t = format!("{}{}-{:?};", t, i * seed, i % 3_u64 == 0);
    }
    let mut u = String::from("x");
    u = format!("{}{}", u, u);
    u = format!("{}{}", u, u);
    println!("{} {} {}", out, t, u);
}
