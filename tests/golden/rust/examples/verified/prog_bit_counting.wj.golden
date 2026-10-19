#[inline]
fn popcount(n: u32) -> i64 {
    let mut x = n;
    let mut c = 0_i64;
    while x > 0_u32 {
        c += (x & 1_u32) as i64;
        x >>= 1;
    }
    c
}

fn main() {
    for n in [5, 255, 1024] {
        println!("{} -> {} bits", n, popcount(n));
    }
    let m: u32 = 1023_u32;
    if popcount(m) != 1_i64 {
        let _ = println!("{} is not a power of two", m);
    }
}

