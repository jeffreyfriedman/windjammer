#[inline]
fn matmul(n: usize, a: &Vec<f64>, b: &Vec<f64>) -> Vec<f64> {
    let mut c: Vec<f64> = Vec::new();
    for _i in 0_usize..n * n {
        c.push(0.0_f64);
    }
    for i in 0_usize..n {
        for j in 0_usize..n {
            let mut s = 0.0_f64;
            for k in 0_usize..n {
                s += a[i * n + k] * b[k * n + j];
            }
            c[i * n + j] = s;
        }
    }
    c
}

fn main() {
    let n = 300_usize;
    let mut a: Vec<f64> = Vec::new();
    let mut b: Vec<f64> = Vec::new();
    for i in 0_usize..n * n {
        a.push(((i % 7_usize) as f64));
        b.push(((i % 11_usize) as f64));
    }
    let c: Vec<f64> = matmul(n as usize, &a, &b);
    let mut total = 0.0_f64;
    for i in 0_usize..n * n {
        total += c[i];
    }
    println!("{}", total);
}

