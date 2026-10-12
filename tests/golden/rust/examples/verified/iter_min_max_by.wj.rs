fn main() {
    let v = vec![4_i32, 1_i32, 9_i32, 3_i32];
    println!("{:?}", v.min().copied());
    println!("{:?}", v.max().copied());
    let words = vec!["fig".to_string(), "cherry".to_string(), "kiwi".to_string()];
    println!("{}", words.iter().max_by_key(|w| w.len()).unwrap_or(String::new()));
    println!("{}", words.iter().min_by_key(|w| w.len()).unwrap_or(String::new()));
}

