#[inline]
fn describe(n: i64) -> String {
    match n {
        0 => String::from("zero"),
        x if x < 10 && x % 2_i64 == 0_i64 => String::from("small even"),
        1 | 3 | 5 | 7 | 9 => String::from("small odd"),
        x if x >= 10 && x < 100 => String::from("medium"),
        _ => String::from("large"),
    }
}

fn main() {
    for n in [0, 4, 7, 50, 500] {
        println!("{}", describe(n));
    }
}

