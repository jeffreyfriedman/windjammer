fn main() {
    let v = vec![3_i32, 5_i32, 8_i32, 12_i32, 16_i32, 21_i32];
    println!("{}", v.iter().any(|x| *x > 20_i64));
    println!("{}", v.iter().all(|x| *x > 3_i64));
    println!("{}", v.iter().filter(|x| *x % 2_i64 == 0_i64).count());
    println!("{:?}", v.iter().find(|x| *x > 12_i64).copied());
}

