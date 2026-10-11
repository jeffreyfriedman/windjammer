// fixture: p01_fill_sum
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

fn fill(n: usize, rng: &mut Rng) -> Vec<u64> {
    let mut v: Vec<u64> = Vec::new();
    for _i in 0_usize..n {
        v.push(rng.next() % 1000_u64);
    }
    v
}

fn main() {
    let mut rng = Rng::new(seed_arg());
    let n = 1000_usize + (rng.next() % 500_u64) as usize;
    let v = fill(n, &mut rng);
    let mut h: u64 = 0xcbf29ce484222325;
    for i in 0_usize..n {
        h = fold(h, v[i] * (i as u64));
    }
    println!("{}", h);
}
