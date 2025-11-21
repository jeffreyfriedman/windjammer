fn add(a: i64, b: i64) -> i64 {
    a + b
}

fn main() {
    println!("=== Simple Language Test ===
");
    let x = 10;
    let y = 20;
    let sum = add(x, y);
    println!("{} + {} = {}", x, y, sum);
    println!("
✅ Test complete!")
}

