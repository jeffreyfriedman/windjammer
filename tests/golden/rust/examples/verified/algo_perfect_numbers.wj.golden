#[inline]
fn is_perfect(n: i64) -> bool {
    let mut s = 0_i64;
    for d in 1_i64..n {
        if n % d == 0_i64 {
            s += d;
        }
    }
    s == n
}

fn main() {
    let mut found: Vec<String> = vec![];
    for n in 2_i32..500_i32 {
        if is_perfect((n as i64)) {
            let _ = found.push(n.to_string());
        }
    }
    println!("perfect: {}", found.join(" "));
}

