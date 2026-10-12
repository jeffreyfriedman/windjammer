fn main() {
    let letters = vec!["a".to_string(), "b".to_string(), "c".to_string()];
    let mut s = String::new();
    for (i, l) in letters.iter().enumerate() {
        s = s + &format!("{}:{} ", i, l);
    }
    println!("{}", s.trim());
    let nums = vec![1_i32, 2_i32, 3_i32];
    let mut t = String::new();
    for (l, n) in letters.iter().zip(nums) {
        t = t + &format!("{}={} ", l, n);
    }
    println!("{}", t.trim());
}

