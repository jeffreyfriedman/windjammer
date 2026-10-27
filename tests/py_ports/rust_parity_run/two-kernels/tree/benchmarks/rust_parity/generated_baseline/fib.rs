fn main() {
    let x = 1 as i64;
    let s = "a".to_string();
    let t = s.clone();
    println!("{}", format!("{}{}", t, x));
    // a comment .clone() as .to_string()
    let y = x as i32 as i64;
}
