#[inline]
fn fill(n: usize) -> Vec<u64> {
    let mut v: Vec<u64> = Vec::new();
    for i in 0_usize..n {
        v.push(i as u64 * 2654435761_u64 % 1000_u64);
    }
    v
}

fn main() {
    let mut total: u64 = 0_u64;
    for round in 0_i32..40_i32 {
        let v: Vec<u64> = fill(2000000_usize);
        let mut s: u64 = 0_u64;
        for i in 0_usize..v.len() {
            s += v[i];
        }
        total += s + round as u64;
    }
    println!("{}", total);
}

