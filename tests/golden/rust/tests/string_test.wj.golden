use windjammer_runtime::test::*;
#[test]

fn test_string_creation() {
    let s: String = String::from("Hello, World!");
    assert!(s == "Hello, World!");
}

#[test]

fn test_string_concatenation() {
    let s1: String = String::from("Hello");
    let s2: String = String::from("World");
    let result = format!("{}{}{}{}", s1, ", ", s2, "!");
    assert!(result == "Hello, World!");
}

#[test]

fn test_string_interpolation() {
    let name: String = String::from("Windjammer");
    let greeting = format!("Hello, {}!", name);
    assert!(greeting == "Hello, Windjammer!");
}

#[test]

fn test_string_interpolation_numbers() {
    let count = 42_i64;
    let message = format!("The answer is {}", count);
    assert!(message == "The answer is 42");
}

#[test]

fn test_string_length() {
    let s: String = String::from("Hello");
    assert!(s.len() == 5_usize);
}

#[test]

fn test_empty_string() {
    let s = String::new();
    assert!(s.is_empty());
}

#[test]

fn test_string_equality() {
    let s1: String = String::from("test");
    let s2: String = String::from("test");
    assert!(s1 == s2);
}

#[test]

fn test_string_inequality() {
    let s1: String = String::from("test");
    let s2: String = String::from("other");
    assert!(s1 != s2);
}

