fn main() {
    let mut n = 0_i64;
    loop {
        n += 1;
        if n * n > 40_i64 {
            break;
        }
    }
    println!("found {}", n);
    let mut odds = String::new();
    for i in 1_i64..=10_i64 {
        if i % 2_i64 == 0_i64 {
            continue;
        }
        odds = odds + &format!("{} ", i);
    }
    println!("odds: {}", odds.trim());
}

