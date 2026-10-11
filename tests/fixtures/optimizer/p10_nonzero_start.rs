// fixture: p10_nonzero_start
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

fn build(lo: usize, hi: usize) -> Vec<usize> {
    let mut v: Vec<usize> = Vec::new();
    for i in lo..hi {
        v.push(i * i);
    }
    v
}

fn main() {
    let seed = seed_arg() as usize;
    let a = build(3_usize + seed, 40_usize);
    let b = build(50_usize, 10_usize);
    let mut h: u64 = 0xcbf29ce484222325;
    for x in a.iter() {
        h = fold(h, *x as u64);
    }
    println!("{} {} {}", a.len(), b.len(), h);
}
