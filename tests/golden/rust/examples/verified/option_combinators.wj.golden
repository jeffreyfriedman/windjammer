#[inline]
fn half(n: i64) -> Option<i64> {
    if n % 2_i64 == 0_i64 {
        Some::<i64>(n / 2_i64)
    } else {
        None
    }
}

fn main() {
    println!("{:?}", Some(5).map(|x| x * 2_i64));
    let n: Option<i64> = None;
    println!("{:?}", n.map(|x| x * 2_i64));
    println!("{:?}", Some(12).and_then(half));
    println!("{}", Some(10).map_or(0, |x| x + 1_i64));
    println!("{}", n.map_or(-1, |x| x + 1_i64));
}

