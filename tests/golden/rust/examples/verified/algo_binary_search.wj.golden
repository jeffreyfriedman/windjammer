#[inline]
fn binary_search(v: &Vec<i64>, target: i64) -> Option<i64> {
    let mut lo = 0_i64;
    let mut hi = v.len() as i64 - 1_i64;
    while lo <= hi {
        let mid = (lo + hi) / 2_i64;
        if v[mid as usize] == target {
            return Some::<i64>(mid);
        } else {
            if (v[mid as usize]) < target {
                lo = mid + 1_i64;
            } else {
                hi = mid - 1_i64;
            }
        }
    }
    None
}

fn main() {
    let v = vec![1_i64, 3_i64, 5_i64, 7_i64, 9_i64, 11_i64, 13_i64];
    println!("{:?}", binary_search(&v, 9_i64));
    println!("{:?}", binary_search(&v, 8_i64));
    println!("{:?}", binary_search(&v, 1_i64));
}

