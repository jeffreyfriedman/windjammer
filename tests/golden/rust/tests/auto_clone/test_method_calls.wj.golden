#[derive(Debug, Clone, PartialEq, Eq, Default)]

struct DataSource {
    items: Vec<String>,
}

impl DataSource {
#[inline]
fn get_items(&self) -> Vec<String> {
        self.items.clone()
}
#[inline]
fn get_count(&self) -> i64 {
        self.items.len() as i64
}
}

#[inline]
fn test_method_call_auto_clone() {
    let source = DataSource { items: vec!["apple".to_string(), "banana".to_string()] };
    let processed = process_items(&source.get_items());
    assert!(source.get_items().len() == 2_usize, "items should still have 2 elements");
    assert!(processed == 2_i64, "processed count should be 2");
    println!("✓ Method call auto-clone works");
}

#[inline]
fn test_chained_method_calls() {
    let source = DataSource { items: vec!["a".to_string(), "b".to_string(), "c".to_string()] };
    let first = process_items(&source.get_items());
    let second = process_items(&source.get_items());
    let third = process_items(&source.get_items());
    assert!(first == 3_i64, "first should be 3");
    assert!(second == 3_i64, "second should be 3");
    assert!(third == 3_i64, "third should be 3");
    println!("✓ Chained method calls auto-clone works");
}

#[inline]
fn process_items(items: &Vec<String>) -> i64 {
    items.len() as i64
}

fn main() {
    test_method_call_auto_clone();
    test_chained_method_calls();
    println!("✓ All method call auto-clone tests passed!");
}

#[inline]
fn assert(condition: bool, message: &str) {
    if !condition {
        let _ = panic!("{}", message);
    }
}

