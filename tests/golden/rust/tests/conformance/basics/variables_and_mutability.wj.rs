fn main() {
    let x = 42_i64;
    println!("[let] {}", x);
    let mut y = 10_i64;
    println!("[mut] {}", y);
    y = 20_i64;
    println!("[mut] {}", y);
    let z = 5_i64;
    println!("[shadow] {}", z);
    let z = z + 10_i64;
    println!("[shadow] {}", z);
    println!("PASSED");
}

