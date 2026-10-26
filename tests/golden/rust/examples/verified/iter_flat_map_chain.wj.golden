fn main() {
    let v = vec![1_i64, 2_i64, 3_i64];
    let dup: Vec<i64> = v.iter().flat_map(|x| [*x, *x]).collect::<Vec<i64>>();
    println!("{:?}", dup);
    let joined: Vec<i64> = [1_i64, 2_i64].iter().chain([3_i64, 4_i64]).cloned().collect::<Vec<i64>>();
    println!("{:?}", joined);
    let nested = vec![vec![1_i32, 2_i32], vec![3_i32, 4_i32]];
    let flat: Vec<i64> = nested.iter().flatten().cloned().collect::<Vec<i64>>();
    println!("{:?}", flat);
}

