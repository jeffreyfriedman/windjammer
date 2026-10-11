// fixture: p07_copy_clone
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

fn weird(n: usize, s: String) -> String {
    let n = s;
    let m = n.clone();
    format!("{}{}", n, m)
}

fn main() {
    let mut rng = Rng::new(seed_arg());
    let a = (rng.next() % 100_u64) as u32;
    let b: u64 = rng.next() % 1000_u64;
    let c = 3.5_f64;
    let r = a.clone() as u64 + b.clone() + (c.clone() * 2.0_f64) as u64;
    let flag = true;
    let f2 = flag.clone();
    println!("{} {} {}", r, f2, weird(5_usize, "q".to_string()));
}
