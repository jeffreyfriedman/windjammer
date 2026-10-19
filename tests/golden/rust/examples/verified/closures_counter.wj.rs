fn main() {
    let mut count = 0_i64;
    let mut inc = || {
        count += 1;
        count
    };
    println!("{}", inc());
    println!("{}", inc());
    println!("{}", inc());
    let mut total = 0_i32;
    for x in [1, 2, 3] {
        total += x as i32;
    }
    println!("total {}", total);
}

