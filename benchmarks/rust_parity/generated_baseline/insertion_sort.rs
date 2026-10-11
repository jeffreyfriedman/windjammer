#[inline]
fn sort(v: Vec<u64>) -> Vec<u64> {
    let mut a: Vec<u64> = v;
    let n = a.len();
    for i in 1_usize..n {
        let key = a[i];
        let mut j = i;
        while j > 0_usize && a[j - 1] > key {
            a[j] = a[j - 1];
            j -= 1;
        }
        a[j] = key;
    }
    a
}

fn main() {
    let mut v: Vec<u64> = Vec::new();
    let mut x: u64 = 88172645463325252_u64;
    for _i in 0_i32..20000_i32 {
        x ^= x << 13_u64;
        x ^= x >> 7_u64;
        x ^= x << 17_u64;
        v.push(x % 100000_u64);
    }
    let s: Vec<u64> = sort(v);
    let mut h: u64 = 0_u64;
    for i in 0_usize..s.len() {
        h += s[(i as usize)] * (i as u64 + 1_u64);
    }
    println!("{}", h);
}

