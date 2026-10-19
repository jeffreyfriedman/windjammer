use windjammer_runtime::test::*;
#[test]

fn test_print_simple() {
    print!("Hello, World!");
}

#[test]

fn test_print_with_interpolation() {
    let name: String = String::from("Windjammer");
    print!("Hello, {}!", name);
}

#[test]

fn test_print_with_number_interpolation() {
    let port = 8080_i64;
    print!("Port: {}", port);
}

#[test]

fn test_string_concatenation() {
    let greeting = format!("{}{}{}", "Hello", ", ", "World");
    assert!(greeting == "Hello, World");
}

#[test]

fn test_string_literal_in_function_call() {
    let s: String = String::from("test");
    assert!(s == "test");
}

#[test]

fn test_empty_vec_with_type() {
    let v: Vec<i64> = vec![];
    assert!(v.is_empty());
}

#[test]

fn test_vec_with_elements() {
    let v = vec![1_i64, 2_i64, 3_i64];
    assert!(v.len() == 3_usize);
}

