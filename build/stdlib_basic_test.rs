use std::fmt::Write;

fn main() {
    println!("========================================");
    println!("Windjammer Basic Test Suite");
    println!("========================================");
    test_assertions();
    test_math();
    test_strings();
    println!("");
    println!("✓ All basic tests passed!")
}

#[inline]
fn test_assertions() {
    println!("Testing assertions...");
    assert!(true);
    assert!(true);
    assert!(true);
    println!("  ✓ Assertions work")
}

fn test_math() {
    println!("Testing math...");
    let x = 10;
    let y = 20;
    let sum = x + y;
    assert!(sum == 30);
    let product = x * y;
    assert!(product == 200);
    let diff = y - x;
    assert!(diff == 10);
    println!("  ✓ Math operations work")
}

#[inline]
fn test_strings() {
    println!("Testing strings...");
    let name = "Windjammer";
    let greeting = {
        let mut __s = String::with_capacity(64);
        write!(&mut __s, "Hello, {}!", name).unwrap();
        __s
    };
    println!(format!("  {}", greeting));
    assert!(name == "Windjammer");
    println!("  ✓ String operations work")
}

