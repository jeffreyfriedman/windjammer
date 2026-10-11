// fixture: p04_string_build
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

fn build(words: Vec<u64>) -> String {
    let mut out = String::new();
    for w in words {
        out = format!("{}{}{}", out, w, ",");
    }
    out
}

fn main() {
    let mut rng = Rng::new(seed_arg());
    let mut words: Vec<u64> = Vec::new();
    for _i in 0_usize..200_usize {
        words.push(rng.next() % 1000_u64);
    }
    let s = build(words);
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.bytes() {
        h = fold(h, b as u64);
    }
    println!("{} {}", s.len(), h);
}
