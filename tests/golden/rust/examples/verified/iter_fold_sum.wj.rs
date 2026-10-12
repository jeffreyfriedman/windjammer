fn main() {
    let v = vec![1_i32, 2_i32, 3_i32, 4_i32, 5_i32];
    println!("{}", v.iter().sum::<i64>());
    println!("{}", v.iter().product::<i64>());
    println!("{}", v.iter().fold(0, |acc, x| {
        if *x > acc {
            *x;
        } else {
            acc;
        }
    }));
    let fs = vec![1.0_f64, 2.0_f64, 3.0_f64, 4.0_f64, 5.0_f64];
    let mean = fs.iter().sum::<f64>() / 5.0_f64;
    println!("{:.1}", mean);
}

