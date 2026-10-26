#[derive(Clone, Debug, PartialEq)]
enum MyError {
    InvalidFormat(String),
    TooLarge(String),
}

fn main() {
    match process_file("bin", 500_u64) {
        Ok(_) => println!("Success"),
        Err(MyError::InvalidFormat(msg)) => println!("Format error: {}", msg),
        Err(MyError::TooLarge(msg)) => println!("Size error: {}", msg),
    }
    println!("✅ Temporary variable ownership works!");
}

#[inline]
fn process_file(ext: &str, size: u64) -> Result<(), MyError> {
    if ext != "txt" {
        return { Err({ let _temp0 = format!("Unsupported: {}", ext); MyError::InvalidFormat(_temp0) }) };
    }
    if size > 1000_u64 {
        return { Err({ let _temp0 = format!("File too large: {}", size); MyError::TooLarge(_temp0) }) };
    }
    Ok(())
}

