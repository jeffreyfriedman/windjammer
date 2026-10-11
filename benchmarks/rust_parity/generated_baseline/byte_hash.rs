fn main() {
    let mut bytes: Vec<u8> = Vec::new();
    let mut x: u64 = 88172645463325252_u64;
    for _i in 0_i32..5000000_i32 {
        x ^= x << 13_u64;
        x ^= x >> 7_u64;
        x ^= x << 17_u64;
        bytes.push((x % 256_u64) as u8);
    }
    let mut h: u64 = 1469598103934665603_u64;
    for _round in 0_i32..10_i32 {
        for i in 0_usize..bytes.len() {
            h = (h ^ (bytes[(i as usize)] as u64)).wrapping_mul(1099511628211_u64);
        }
    }
    println!("{}", h);
}

