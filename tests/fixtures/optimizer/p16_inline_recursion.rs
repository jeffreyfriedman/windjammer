// fixture: p16_inline_recursion
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

#[inline]
fn fib(n: u64) -> u64 {
    if n < 2_u64 {
        return n;
    }
    let a = fib(n - 1_u64);
    let b = fib(n - 2_u64);
    let total = a + b;
    let doubled = total * 2_u64;
    let halved = doubled / 2_u64;
    let checked = if halved == total { halved } else { 0_u64 };
    let extra = checked + 0_u64;
    let more = extra + 0_u64;
    let last = more + 0_u64;
    last
}

#[inline]
fn main() {
    let seed = seed_arg();
    println!("{}", fib(15_u64 + seed));
}
