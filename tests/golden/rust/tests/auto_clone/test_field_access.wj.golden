#[derive(Debug, Clone, PartialEq, Eq, Default)]

struct Config {
    paths: Vec<String>,
    max_size: i64,
    enabled: bool,
}

#[inline]
fn test_field_access_auto_clone() {
    let config = Config { paths: vec!["/tmp".to_string(), "/usr".to_string()], max_size: 1024_i64, enabled: true };
    let count = count_paths(&config.paths);
    assert!(config.paths.len() == 2_usize, "paths should still have 2 elements");
    assert!(count == 2_i64, "count should be 2");
    println!("✓ Field access auto-clone works");
}

#[inline]
fn test_nested_field_access() {
    let config = Config { paths: vec!["a".to_string(), "b".to_string(), "c".to_string()], max_size: 2048_i64, enabled: false };
    let first_count = count_paths(&config.paths);
    let second_count = count_paths(&config.paths);
    assert!(first_count == 3_i64, "first count should be 3");
    assert!(second_count == 3_i64, "second count should be 3");
    assert!(config.paths.len() == 3_usize, "paths should still have 3 elements");
    println!("✓ Nested field access auto-clone works");
}

#[inline]
fn count_paths(paths: &Vec<String>) -> i64 {
    paths.len() as i64
}

fn main() {
    test_field_access_auto_clone();
    test_nested_field_access();
    println!("✓ All field access auto-clone tests passed!");
}

#[inline]
fn assert(condition: bool, message: &str) {
    if !condition {
        let _ = panic!("{}", message);
    }
}

