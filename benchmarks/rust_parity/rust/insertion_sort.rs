fn sort(a: &mut [u64]) {
    for i in 1..a.len() {
        let key = a[i];
        let mut j = i;
        while j > 0 && a[j - 1] > key {
            a[j] = a[j - 1];
            j -= 1;
        }
        a[j] = key;
    }
}

fn main() {
    let mut x: u64 = 88172645463325252;
    let mut v: Vec<u64> = Vec::with_capacity(20000);
    for _ in 0..20000 {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        v.push(x % 100000);
    }
    sort(&mut v);
    let h: u64 = v.iter().enumerate().map(|(i, s)| s * (i as u64 + 1)).sum();
    println!("{}", h);
}
