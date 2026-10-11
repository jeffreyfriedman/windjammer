// fixture: p20_macro_clone
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

fn describe(name: &String, n: u64) -> String {
    format!("{}:{}:{}", name.clone(), n, name.clone())
}

fn main() {
    let mut rng = Rng::new(seed_arg());
    let name = String::from("kernel");
    let n = rng.next() % 1000_u64;
    let d = describe(&name, n);
    let items = vec![name.clone(), d.clone()];
    println!("{} {} {} {} {}", d.clone(), name.clone(), items.len(), name, d);
}
