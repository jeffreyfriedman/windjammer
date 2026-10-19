use std::cmp::Ordering;
fn main() {
    let data = vec![2.0_f64, 4.0_f64, 4.0_f64, 4.0_f64, 5.0_f64, 5.0_f64, 7.0_f64, 9.0_f64];
    let n = data.len() as f64;
    let mean = data.iter().sum::<f64>() / n;
    let var = data.iter().map(move |x| (*x - mean) * (*x - mean)).sum::<f64>() / n;
    println!("mean={:.1} stddev={:.1}", mean, var.sqrt());
    let mut sorted = data;
    sorted.sort_by(move |a, b| a.partial_cmp(&b).unwrap_or(Ordering::Equal));
    let mid = sorted.len() / 2_usize;
    let median = (sorted[mid - 1] + sorted[mid]) / 2.0_f64;
    println!("median={:.1}", median);
}

