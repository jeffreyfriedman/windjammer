fn main() {
    let x = 42_i32;
    println!("[let] {}", x);
    let mut y = 10_i32;
    println!("[mut] {}", y);
    y = 20_i32;
    println!("[mut] {}", y);
    let z = 5_i32;
    println!("[shadow] {}", z);
    let z = z + 10_i32;
    println!("[shadow] {}", z);
    println!("PASSED");
}

