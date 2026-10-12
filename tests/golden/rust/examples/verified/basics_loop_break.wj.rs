fn main() {
    let mut n = 0_i32;
    loop {
        n += 1;
        if n * n > 40_i32 {
            break;
        }
    }
    println!("found {}", n);
    let mut odds = String::new();
    for i in 1_i32..=10_i32 {
        if i % 2_i32 == 0_i32 {
            continue;
        }
        odds = odds + &format!("{} ", i);
    }
    println!("odds: {}", odds.trim());
}

