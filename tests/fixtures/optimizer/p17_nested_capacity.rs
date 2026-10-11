// fixture: p17_nested_capacity
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

fn grid(rows: usize, cols: usize, rng: &mut Rng) -> Vec<Vec<u64>> {
    let mut g: Vec<Vec<u64>> = Vec::new();
    for _r in 0_usize..rows {
        let mut row: Vec<u64> = Vec::new();
        for _c in 0_usize..cols {
            row.push(rng.next() % 100_u64);
        }
        g.push(row);
    }
    g
}

fn main() {
    let mut rng = Rng::new(seed_arg());
    let g = grid(20_usize, 30_usize, &mut rng);
    let mut h: u64 = 0xcbf29ce484222325;
    for r in 0_usize..g.len() {
        for c in 0_usize..g[r].len() {
            h = fold(h, g[r][c] * ((r * 31 + c) as u64));
        }
    }
    println!("{}", h);
}
