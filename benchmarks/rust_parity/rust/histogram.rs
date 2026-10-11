fn main() {
    let mut counts = vec![0u64; 4096];
    let mut x: u64 = 88172645463325252;
    for _ in 0..30_000_000 {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        counts[(x % 4096) as usize] += 1;
    }
    let total: u64 = counts.iter().enumerate().map(|(k, c)| c * (k as u64 + 1)).sum();
    println!("{}", total);
}
