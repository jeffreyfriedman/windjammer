// fixture: p11_repr_to_bytes
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
struct Unused {
    a: u8,
    b: u32,
    c: u8,
}
impl Unused {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(6);
        bytes.push(self.a);
        bytes.extend_from_slice(&self.b.to_ne_bytes());
        bytes.push(self.c);
        bytes
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[repr(C)]
struct Used {
    a: u8,
    b: u32,
}
impl Used {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(5);
        bytes.push(self.a);
        bytes.extend_from_slice(&self.b.to_le_bytes());
        bytes
    }
}

fn main() {
    let mut rng = Rng::new(seed_arg());
    let u = Used { a: (rng.next() % 256_u64) as u8, b: (rng.next() % 100000_u64) as u32 };
    let mut h: u64 = 0xcbf29ce484222325;
    for b in u.to_bytes() {
        h = fold(h, b as u64);
    }
    let w = Unused::default();
    println!("{} {:?}", h, w.a);
}
