fn main() {
    let v = vec![1_i32, 2_i32, 3_i32, 4_i32, 5_i32];
    let a: Vec<i64> = v.iter().take(3_i64).cloned().collect::<Vec<i64>>();
    let b: Vec<i64> = v.iter().skip(3_i64).cloned().collect::<Vec<i64>>();
    let c: Vec<i64> = v.iter().rev().cloned().collect::<Vec<i64>>();
    let d: Vec<i64> = (0..10).step_by(3).collect::<Vec<i64>>();
    println!("{:?}", a);
    println!("{:?}", b);
    println!("{:?}", c);
    println!("{:?}", d);
}

