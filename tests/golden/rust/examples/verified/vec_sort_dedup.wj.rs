fn main() {
    let mut v = vec![5_i32, 3_i32, 8_i32, 1_i32, 3_i32, 2_i32, 1_i32];
    v.sort();
    println!("{:?}", v);
    v.dedup();
    println!("{:?}", v);
    v.sort_by(|a, b| b.cmp(&a));
    println!("{:?}", v);
}

