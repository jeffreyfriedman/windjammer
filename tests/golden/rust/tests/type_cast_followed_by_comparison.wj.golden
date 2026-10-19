fn test_cast_and_compare() {
    let x = 5_i64;
    let arr = vec![1_i64, 2_i64, 3_i64, 4_i64, 5_i64];
    if (x as usize) < arr.len() {
        let _ = println!("x is less than array length");
    }
    if (x as usize) > 0_usize {
        let _ = println!("x is positive");
    }
    if x as usize == arr.len() {
        let _ = println!("x equals array length");
    }
}

fn main() {
    test_cast_and_compare();
    println!("✅ Type casts followed by comparisons work!");
}

