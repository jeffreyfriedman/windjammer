// Rust twin of bench/interp/strings.wj: string building with a length-triggered reset.
use std::fmt::Write;

fn main() {
    let mut s = String::new();
    let mut total = 0;
    for i in 0..200_000 {
        write!(s, "{},", i % 10).unwrap();
        if s.len() > 1000 {
            total += s.len();
            s.clear();
        }
    }
    println!("{} {}", total, s.len());
}
