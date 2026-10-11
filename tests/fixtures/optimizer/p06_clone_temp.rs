// fixture: p06_clone_temp
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
    let base = vec![1_u64, 2_u64, 3_u64].clone();
    let mut v: Vec<u64> = Vec::new();
    for _i in 0_usize..50_usize {
        v.push(rng.next() % 10_u64);
    }
    let snapshot = v.clone();
    v.push(99_u64);
    v[0] = 7_u64;
    let name = "abc".to_string().clone();
    let mut h: u64 = 0xcbf29ce484222325;
    for x in snapshot.iter() {
        h = fold(h, *x);
    }
    for x in v.iter() {
        h = fold(h, *x * 3_u64);
    }
    for x in base.iter() {
        h = fold(h, *x);
    }
    println!("{} {} {}", h, name, snapshot.len() + v.len());
}
