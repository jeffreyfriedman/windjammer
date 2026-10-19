#[inline]
fn classify(n: i64) -> String {
    if n < 0_i64 {
        String::from("negative")
    } else {
        if n == 0_i64 {
            String::from("zero")
        } else {
            if n < 100_i64 {
                String::from("small")
            } else {
                String::from("big")
            }
        }
    }
}

fn main() {
    println!("{}", classify(-5_i64));
    println!("{}", classify(0_i64));
    println!("{}", classify(42_i64));
    println!("{}", classify(1000_i64));
}

