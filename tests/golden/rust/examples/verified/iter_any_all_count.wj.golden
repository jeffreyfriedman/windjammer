fn main() {
    let v = vec![3_i64, 5_i64, 8_i64, 12_i64, 16_i64, 21_i64];
    println!("{}", v.iter().any(|x| *x > 20_i64));
    println!("{}", v.iter().all(|x| *x > 3_i64));
    println!("{}", v.iter().filter(|x| *x % 2_i64 == 0_i64).count());
    println!("{:?}", v.iter().find(|x| *x > 12_i64).copied());
}

