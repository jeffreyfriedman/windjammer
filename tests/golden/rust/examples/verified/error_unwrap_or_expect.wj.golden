fn main() {
    let a: i64 = "42".parse().unwrap_or(0_i64);
    let b: i64 = "nope".parse().unwrap_or(0_i64);
    println!("{}", a);
    println!("{}", b);
    let c = "8".parse::<i64>().unwrap_or_default();
    println!("{}", c);
    let name: Option<String> = None;
    println!("{}", name.unwrap_or(String::from("fallback")));
}

