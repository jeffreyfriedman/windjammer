fn main() {
    let v = vec![4_i32, 8_i32, 15_i32, 16_i32, 23_i32, 42_i32];
    println!("{}", v.contains(15));
    println!("{}", v.contains(5));
    println!("{:?}", v.iter().position(|x| *x == 15_i64));
    println!("{:?}", v.iter().position(|x| *x == 5_i64));
}

