fn main() {
    let v = vec![1_i64, 2_i64, 3_i64, 4_i64, 5_i64];
    println!("{:?}", &v[1..4]);
    let sums: Vec<i64> = v.windows(2_usize).map(|w| w[0].clone() + w[1].clone()).collect::<Vec<i64>>();
    let strs: Vec<String> = sums.iter().map(|s| s.to_string()).collect::<Vec<String>>();
    println!("{}", strs.join(" "));
    let mut parts = String::new();
    for c in v.chunks(2_usize) {
        parts = parts + &format!("{:?} ", c);
    }
    println!("{}", parts.trim());
}

