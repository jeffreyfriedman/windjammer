fn main() {
    let a = 5_i64;
    if a > 0_i64 {
        println!("[if] positive");
    } else {
        println!("[if] non-positive");
    }
    let b = 0_i64;
    if b > 0_i64 {
        println!("[if] positive");
    } else {
        if b == 0_i64 {
            println!("[if] zero");
        } else {
            println!("[if] negative");
        }
    }
    let c = -3_i64;
    if c > 0_i64 {
        println!("[if] positive");
    } else {
        if c == 0_i64 {
            println!("[if] zero");
        } else {
            println!("[if] negative");
        }
    }
    let mut i = 0_i64;
    while i < 3_i64 {
        let _ = println!("[while] {}", i);
        i += 1;
    }
    for j in 0_i64..5_i64 {
        println!("[for] {}", j);
    }
    println!("PASSED");
}

