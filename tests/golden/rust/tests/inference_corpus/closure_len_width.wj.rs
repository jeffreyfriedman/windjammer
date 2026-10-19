fn main() {
    let v = vec![1_i64, 2_i64, 3_i64];
    let f = |x: i64| x + v.len();
    println!("{}", f(1));
}

