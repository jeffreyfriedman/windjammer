fn main() {
    let v = vec![1_i32, 2_i32, 3_i32, 4_i32, 5_i32];
    let squares: Vec<i64> = v.iter().map(|x| x * x).collect::<Vec<i64>>();
    println!("{:?}", squares);
    let evens: Vec<i64> = v.iter().filter(|x| *x % 2_i64 == 0_i64).cloned().collect::<Vec<i64>>();
    println!("{:?}", evens);
    let shifted: Vec<i64> = [1_i64, 2_i64, 3_i64].iter().map(|x| x + 10_i64).collect::<Vec<i64>>();
    println!("{:?}", shifted);
}

