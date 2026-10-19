#[inline]
fn hanoi(n: i64) -> i64 {
    if n == 0_i64 {
        0_i64
    } else {
        2_i64 * hanoi(n - 1_i64) + 1_i64
    }
}

#[inline]
fn ackermann(m: i64, n: i64) -> i64 {
    if m == 0_i64 {
        n + 1_i64
    } else {
        if n == 0_i64 {
            ackermann(m - 1_i64, 1_i64)
        } else {
            ackermann(m - 1_i64, ackermann(m, n - 1_i64))
        }
    }
}

fn main() {
    println!("hanoi(10) = {} moves", hanoi(10_i64));
    println!("ackermann(2, 3) = {}", ackermann(2_i64, 3_i64));
}

