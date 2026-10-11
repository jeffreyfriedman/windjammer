// fixture: p02_matmul
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

fn matmul(n: usize, a: &Vec<f64>, b: &Vec<f64>) -> Vec<f64> {
    let mut c: Vec<f64> = Vec::new();
    for _i in 0_usize..n * n {
        c.push(0.0_f64);
    }
    for i in 0_usize..n {
        for j in 0_usize..n {
            let mut s = 0.0_f64;
            for k in 0_usize..n {
                s += a[i * n + k] * b[k * n + j];
            }
            c[i * n + j] = s;
        }
    }
    c
}

fn main() {
    let mut rng = Rng::new(seed_arg());
    let n = 24_usize;
    let mut a: Vec<f64> = Vec::new();
    let mut b: Vec<f64> = Vec::new();
    for _i in 0_usize..n * n {
        a.push((rng.next() % 100_u64) as f64);
        b.push((rng.next() % 100_u64) as f64);
    }
    let c = matmul(n as usize, &a, &b);
    let mut h: u64 = 0xcbf29ce484222325;
    for i in 0_usize..n * n {
        h = fold(h, c[i] as u64);
    }
    println!("{}", h);
}
