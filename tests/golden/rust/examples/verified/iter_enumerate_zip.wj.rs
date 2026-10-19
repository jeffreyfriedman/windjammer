fn main() {
    let letters = vec!["a".to_string(), "b".to_string(), "c".to_string()];
    let mut s = String::new();
    for (i, l) in letters.iter().enumerate() {
        s = s + &format!("{}:{} ", i, l);
    }
    println!("{}", s.trim());
    let nums = vec![1_i64, 2_i64, 3_i64];
    let mut t = String::new();
    for (l, n) in letters.iter().zip(nums) {
        t = t + &format!("{}={} ", l, n);
    }
    println!("{}", t.trim());
}

