use std::cmp::Ordering;
fn main() {
    let mut v = vec![2.5_f64, 0.5_f64, 9.0_f64, 1.5_f64];
    v.sort_by(move |a, b| a.partial_cmp(&b).unwrap_or(Ordering::Equal));
    println!("{:?}", v);
}

