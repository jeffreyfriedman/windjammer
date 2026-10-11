fn main() {
    let mut x: u64 = 88172645463325252;
    let bytes: Vec<u8> = (0..5_000_000)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            (x % 256) as u8
        })
        .collect();
    let mut h: u64 = 1469598103934665603;
    for _ in 0..10 {
        for &b in &bytes {
            h = (h ^ b as u64).wrapping_mul(1099511628211);
        }
    }
    println!("{}", h);
}
