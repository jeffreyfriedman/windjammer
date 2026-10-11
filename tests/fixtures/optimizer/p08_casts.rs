// fixture: p08_casts
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
    let n: usize = 17_usize;
    let k = (rng.next() % 5000_u64) as i64 - 2500_i64;
    let a = n as usize + 1_usize;
    let b = (k as i32) as i32;
    let c = (k as u8) as u32;
    let d = k as i32 as i32 as i64;
    let e = (k as i16) as i64;
    let x: u8 = (rng.next() % 256_u64) as u8;
    let y = x as u32 + (x as u8) as u32;
    println!("{} {} {} {} {} {}", a, b, c, d, e, y);
}
