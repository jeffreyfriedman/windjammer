fn main() {
    let v = vec![1_i32, 2_i32, 3_i32, 4_i32, 5_i32];
    let mut running: Vec<i64> = vec![];
    let mut acc = 0_i32;
    for x in v {
        acc += x as i32;
        running.push((acc as i64));
    }
    println!("{:?}", running);
}

