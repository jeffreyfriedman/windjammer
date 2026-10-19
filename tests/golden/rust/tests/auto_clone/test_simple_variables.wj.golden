#[inline]
fn test_vec_auto_clone() {
    let data = vec![1_i64, 2_i64, 3_i64, 4_i64, 5_i64];
    let sum = sum_vec(&data);
    assert!(data.len() == 5_usize, "data should still have 5 elements");
    assert!(sum == 15_i64, "sum should be 15");
    println!("✓ Vec auto-clone works");
}

#[inline]
fn test_string_auto_clone() {
    let message: String = String::from("Hello, Windjammer!");
    let upper = to_upper(&message);
    assert!(message.len() == 18_usize, "message should still have 18 chars");
    assert!(upper == "HELLO, WINDJAMMER!", "uppercase should match");
    println!("✓ string auto-clone works");
}

#[inline]
fn test_multiple_uses() {
    let numbers = vec![10_i64, 20_i64, 30_i64];
    let first_sum = sum_vec(&numbers);
    let second_sum = sum_vec(&numbers);
    let third_sum = sum_vec(&numbers);
    assert!(first_sum == 60_i64, "first sum should be 60");
    assert!(second_sum == 60_i64, "second sum should be 60");
    assert!(third_sum == 60_i64, "third sum should be 60");
    println!("✓ Multiple uses auto-clone works");
}

#[inline]
fn sum_vec(v: &Vec<i64>) -> i64 {
    v.iter().sum()
}

#[inline]
fn to_upper(s: &str) -> String {
    s.to_uppercase()
}

fn main() {
    test_vec_auto_clone();
    test_string_auto_clone();
    test_multiple_uses();
    println!("✓ All simple variable auto-clone tests passed!");
}

#[inline]
fn assert(condition: bool, message: &str) {
    if !condition {
        let _ = panic!("{}", message);
    }
}

