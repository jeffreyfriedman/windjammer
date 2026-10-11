fn matmul(n: usize, a: &[f64], b: &[f64]) -> Vec<f64> {
    let mut c = vec![0.0; n * n];
    for i in 0..n {
        for j in 0..n {
            let mut s = 0.0;
            for k in 0..n {
                s += a[i * n + k] * b[k * n + j];
            }
            c[i * n + j] = s;
        }
    }
    c
}

fn main() {
    let n = 300;
    let a: Vec<f64> = (0..n * n).map(|i| (i % 7) as f64).collect();
    let b: Vec<f64> = (0..n * n).map(|i| (i % 11) as f64).collect();
    let c = matmul(n, &a, &b);
    let total: f64 = c.iter().sum();
    println!("{}", total);
}
