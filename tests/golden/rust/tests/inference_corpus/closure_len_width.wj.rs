fn main() {
    let v = vec![1_i32, 2_i32, 3_i32];
    let f = |x: i64| x + v.len();
    println!("{}", f(1));
}

