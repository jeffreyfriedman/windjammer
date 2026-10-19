//! Tiny deterministic PRNG (xorshift64*) for `--seed`.

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed.wrapping_mul(0x9E3779B97F4A7C15) | 1)
    }
    pub fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
    pub fn shuffle<T>(&mut self, v: &mut [T]) {
        for i in (1..v.len()).rev() {
            let j = (self.next() % (i as u64 + 1)) as usize;
            v.swap(i, j);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_shuffle() {
        let (mut a, mut b) = ((0..20).collect::<Vec<_>>(), (0..20).collect::<Vec<_>>());
        Rng::new(7).shuffle(&mut a);
        Rng::new(7).shuffle(&mut b);
        assert_eq!(a, b);
        let mut c = (0..20).collect::<Vec<_>>();
        Rng::new(8).shuffle(&mut c);
        assert_ne!(a, c);
    }
}
