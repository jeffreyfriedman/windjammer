fn main() {
    let mut x: i32 = 1_i32;
    let mut steps = 0_i32;
    let mut line = String::new();
    while x <= 32_i32 {
        line = line + &format!("{} ", x);
        x *= 2;
        steps += 1;
    }
    println!("{}", line);
    println!("steps: {}", steps);
}

