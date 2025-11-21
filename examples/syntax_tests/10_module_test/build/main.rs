pub mod test_simple {
pub fn hello() -> String {
    "Hello from module!".to_string()
}

pub fn add(a: i64, b: i64) -> i64 {
    a + b
}


}




fn main() {
    println!("Testing module system...");
    let msg = test_simple::hello();
    println!("Module says: {}", msg);
    let result = test_simple::add(10, 20);
    println!("10 + 20 = {}", result);
    println!("✅ Module test complete!")
}

