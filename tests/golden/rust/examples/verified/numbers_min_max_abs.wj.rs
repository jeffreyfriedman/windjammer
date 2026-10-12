fn main() {
    println!("{}", 3.min(9_i32));
    println!("{}", 3.max(9_i32));
    println!("{}", (-4).abs());
    println!("{}", 15.clamp(0_i32, 5_i32));
    println!("{}", (-8).clamp(5_i32, 10_i32));
}

