#[inline]
fn c_to_f(c: f64) -> f64 {
    c * 9.0_f64 / 5.0_f64 + 32.0_f64
}

fn main() {
    for c in [-40, 0, 37, 100] {
        println!("{}C = {:.1}F", c, c_to_f(c as f64));
    }
}

