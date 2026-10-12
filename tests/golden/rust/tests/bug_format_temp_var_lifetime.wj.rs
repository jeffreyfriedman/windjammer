#[derive(Clone, Debug, PartialEq)]
enum AssetError {
    InvalidFormat(String),
    TooLarge(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]

struct ErrorReport {
    message: String,
    code: i32,
}

pub fn test_format_in_error_enum() {
    let result = validate_extension("txt");
    assert!(result.is_err(), "Should reject invalid extension");
    match result {
        Err(AssetError::InvalidFormat(msg)) => {
            assert!(msg.contains("Unsupported"), "Error message should contain 'Unsupported'");
        },
        _ => assert!(false, "Wrong error variant"),
    }
    let result2 = validate_size(1000_i32, 500_i32);
    assert!(result2.is_err(), "Should reject large size");
    match result2 {
        Err(AssetError::TooLarge(msg)) => {
            assert!(msg.contains("exceeds"), "Error message should contain 'exceeds'");
        },
        _ => assert!(false, "Wrong error variant"),
    }
    println!("✅ format! in enum variants works correctly");
}

#[inline]
pub fn test_format_in_function_arg() {
    let code = 404_i32;
    let path: String = String::from("/api/users");
    { let _temp0 = format!("HTTP {} for path {}", code, path); log_error(&_temp0) };
    println!("✅ format! in function args works correctly");
}

#[inline]
pub fn test_format_in_struct_field() {
    let error_code = 500_i32;
    let details: String = String::from("Database connection failed");
    let report = ErrorReport { message: format!("Error {}: {}", error_code, details), code: error_code };
    assert!(report.message.contains("Error 500"), "Message should contain error code");
    println!("✅ format! in struct fields works correctly");
}

#[inline]
fn validate_extension(ext: &str) -> Result<(), AssetError> {
    if ext == "png" || ext == "jpg" {
        Ok(())
    } else {
        { Err({ let _temp0 = format!("Unsupported file extension: {}", ext); AssetError::InvalidFormat(_temp0) }) }
    }
}

#[inline]
fn validate_size(size: i32, max: i32) -> Result<(), AssetError> {
    if size <= max {
        Ok(())
    } else {
        { Err({ let _temp0 = format!("Asset size {} exceeds max {}", size, max); AssetError::TooLarge(_temp0) }) }
    }
}

#[inline]
fn log_error(message: &str) {
    println!("ERROR: {}", message);
}

