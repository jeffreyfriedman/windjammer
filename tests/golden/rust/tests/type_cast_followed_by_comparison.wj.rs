fn test_cast_and_compare() {
    let x = 5_i32;
    let arr = vec![1_i32, 2_i32, 3_i32, 4_i32, 5_i32];
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

