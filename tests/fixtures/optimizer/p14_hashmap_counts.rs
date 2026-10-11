// fixture: p14_hashmap_counts
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

use std::collections::HashMap;

fn main() {
    let mut rng = Rng::new(seed_arg());
    let mut counts: HashMap<u64, u64> = HashMap::new();
    for _i in 0_usize..5000_usize {
        let k = rng.next() % 64_u64;
        let c = counts.get(&k).copied().unwrap_or(0_u64);
        counts.insert(k, c + 1_u64);
    }
    let mut keys: Vec<u64> = counts.keys().copied().collect();
    keys.sort();
    let mut h: u64 = 0xcbf29ce484222325;
    for k in keys.iter() {
        h = fold(h, *k);
        h = fold(h, counts[k]);
    }
    println!("{} {}", keys.len(), h);
}
