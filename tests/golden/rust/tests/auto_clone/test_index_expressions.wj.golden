#[inline]
fn test_literal_index_auto_clone() {
    let items = vec!["apple".to_string(), "banana".to_string(), "cherry".to_string()];
    let processed = process_item(&items[0]);
    assert!(items[0] == "apple", "items[0] should still be apple");
    assert!(processed == "APPLE", "processed should be APPLE");
    println!("✓ Literal index auto-clone works");
}

#[inline]
fn test_variable_index_auto_clone() {
    let items = vec!["first".to_string(), "second".to_string(), "third".to_string()];
    let idx = 1_i32;
    let processed = process_item(&(((((items[(idx as usize)]))))));
    assert!(items[(idx as usize)] == "second", "items[idx] should still be second");
    assert!(processed == "SECOND", "processed should be SECOND");
    println!("✓ Variable index auto-clone works");
}

#[inline]
fn test_multiple_index_uses() {
    let items = vec!["a".to_string(), "b".to_string(), "c".to_string()];
    let first = process_item(&items[0]);
    let second = process_item(&items[0]);
    let third = process_item(&items[0]);
    assert!(first == "A", "first should be A");
    assert!(second == "A", "second should be A");
    assert!(third == "A", "third should be A");
    assert!(items[0] == "a", "items[0] should still be a");
    println!("✓ Multiple index uses auto-clone works");
}

#[inline]
fn process_item(item: &str) -> String {
    item.to_uppercase()
}

fn main() {
    test_literal_index_auto_clone();
    test_variable_index_auto_clone();
    test_multiple_index_uses();
    println!("✓ All index expression auto-clone tests passed!");
}

#[inline]
fn assert(condition: bool, message: &str) {
    if !condition {
        let _ = panic!("{}", message);
    }
}

