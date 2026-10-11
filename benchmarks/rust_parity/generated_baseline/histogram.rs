fn main() {
    let mut counts: Vec<u64> = Vec::new();
    for _i in 0_i32..4096_i32 {
        counts.push(0_u64);
    }
    let mut x: u64 = 88172645463325252_u64;
    for _i in 0_i32..30000000_i32 {
        x ^= x << 13_u64;
        x ^= x >> 7_u64;
        x ^= x << 17_u64;
        let k = (x % 4096_u64) as usize;
        counts[k] += 1;
    }
    let mut total: u64 = 0_u64;
    for k in 0_i32..4096_i32 {
        total += counts[(k as usize)] * (k as u64 + 1_u64);
    }
    println!("{}", total);
}

