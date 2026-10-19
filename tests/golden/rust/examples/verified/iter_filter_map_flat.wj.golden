fn main() {
    let raw = vec!["1".to_string(), "a".to_string(), "3".to_string(), "b".to_string(), "5".to_string()];
    let nums: Vec<i64> = raw.iter().filter_map(|s| s.parse::<i64>().ok()).collect::<Vec<i64>>();
    println!("{:?}", nums);
    println!("{}", nums.iter().sum::<i64>());
}

