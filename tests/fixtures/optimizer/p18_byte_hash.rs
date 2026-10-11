// fixture: p18_byte_hash
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
    let mut rng = Rng::new(seed_arg());
    let mut bytes: Vec<u8> = Vec::new();
    for _i in 0_usize..500_usize {
        bytes.push((rng.next() % 256_u64) as u8);
    }
    let mut h: u64 = 0xcbf29ce484222325;
    for b in bytes.iter() {
        h = fold(h, (*b as u32) as u64);
    }
    let total: u64 = bytes.iter().map(|b| *b as u64).sum();
    println!("{} {}", h, total);
}
