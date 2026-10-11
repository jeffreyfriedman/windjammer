// fixture: p19_repr_layout
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

#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[repr(C)]
struct Layout {
    a: u8,
    b: u32,
    c: u8,
}

fn main() {
    let seed = seed_arg();
    let l = Layout { a: seed as u8, b: (seed * 7) as u32, c: 1 };
    println!("{} {}", std::mem::size_of::<Layout>(), l.a as u32 + l.b + l.c as u32);
}
