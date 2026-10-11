// fixture: p15_shadow_traps
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
    let x = seed as usize;
    let y = format!("{}", x);
    let x = y;
    let z = x.clone();
    let mut parts: Vec<String> = Vec::new();
    parts.push(x);
    parts.push(z);
    let w = 5_u32;
    let w2 = w.clone();
    let opt: Option<String> = Some("hi".to_string());
    if let Some(w) = opt {
        let c = w.clone();
        parts.push(c);
        parts.push(w);
    }
    println!("{} {} {:?}", w, w2, parts);
}
