// fixture: p09_early_break
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

fn collect_until(limit: u64, max: usize, rng: &mut Rng) -> Vec<u64> {
    let mut v: Vec<u64> = Vec::new();
    for i in 0_usize..max {
        let x = rng.next() % 1000_u64;
        v.push(x);
        if x < limit && i > 3_usize {
            break;
        }
    }
    v
}

fn main() {
    let mut rng = Rng::new(seed_arg());
    let v = collect_until(10_u64, usize::MAX / 4_usize, &mut rng);
    let mut h: u64 = 0xcbf29ce484222325;
    for x in v.iter() {
        h = fold(h, *x);
    }
    println!("{} {}", v.len(), h);
}
