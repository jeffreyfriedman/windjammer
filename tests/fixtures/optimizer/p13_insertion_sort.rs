// fixture: p13_insertion_sort
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

fn sort(v: &mut Vec<u64>) {
    let n = v.len();
    for i in 1_usize..n {
        let key = v[i];
        let mut j = i;
        while j > 0_usize && v[j - 1_usize] > key {
            v[j] = v[j - 1_usize];
            j -= 1;
        }
        v[j] = key;
    }
}

fn main() {
    let mut rng = Rng::new(seed_arg());
    let mut v: Vec<u64> = Vec::new();
    for _i in 0_usize..300_usize {
        v.push(rng.next() % 10000_u64);
    }
    sort(&mut v);
    let mut h: u64 = 0xcbf29ce484222325;
    for i in 0_usize..v.len() {
        h = fold(h, v[i] * (i as u64 + 1_u64));
    }
    println!("{}", h);
}
