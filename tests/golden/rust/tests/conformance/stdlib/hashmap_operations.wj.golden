use std::collections::HashMap;
#[inline]
fn test_map_create() {
    let map: HashMap<String, i64> = HashMap::new();
    println!("[map_create] empty len={}", map.len());
}

#[inline]
fn test_map_insert() {
    let mut map = HashMap::new();
    map.insert(String::from("alice"), 30);
    map.insert(String::from("bob"), 25);
    map.insert(String::from("charlie"), 35);
    println!("[map_insert] after inserts: len={}", map.len());
}

fn test_map_get() {
    let mut map = HashMap::new();
    map.insert(String::from("alice"), 30);
    map.insert(String::from("bob"), 25);
    match map.get("alice") {
        Some(age) => println!("[map_get] alice -> {}", age),
        None => println!("[map_get] alice -> not found"),
    }
    match map.get("bob") {
        Some(age) => println!("[map_get] bob -> {}", age),
        None => println!("[map_get] bob -> not found"),
    }
    match map.get("unknown") {
        Some(age) => println!("[map_get] unknown -> {}", age),
        None => println!("[map_get] unknown -> not found"),
    }
}

#[inline]
fn test_map_contains() {
    let mut map = HashMap::new();
    map.insert(String::from("alice"), 30);
    println!("[map_contains] has alice: {}", map.contains_key("alice"));
    println!("[map_contains] has dave: {}", map.contains_key("dave"));
}

#[inline]
fn test_map_remove() {
    let mut map = HashMap::new();
    map.insert(String::from("alice"), 30);
    map.insert(String::from("bob"), 25);
    map.insert(String::from("charlie"), 35);
    println!("[map_remove] before remove: len={}", map.len());
    map.remove("bob");
    println!("[map_remove] after remove bob: len={}", map.len());
    println!("[map_remove] bob still exists: {}", map.contains_key("bob"));
}

fn test_map_update() {
    let mut map = HashMap::new();
    map.insert(String::from("charlie"), 35);
    match map.get("charlie") {
        Some(age) => println!("[map_update] charlie age before: {}", age),
        None => println!("[map_update] charlie not found"),
    }
    map.insert(String::from("charlie"), 36);
    match map.get("charlie") {
        Some(age) => println!("[map_update] charlie age after: {}", age),
        None => println!("[map_update] charlie not found"),
    }
}

#[inline]
fn test_map_empty() {
    let mut map: HashMap<String, i64> = HashMap::new();
    println!("[map_empty] is_empty before: {}", map.is_empty());
    map.insert(String::from("key"), 1_i64);
    println!("[map_empty] is_empty after: {}", map.is_empty());
}

fn main() {
    test_map_create();
    test_map_insert();
    test_map_get();
    test_map_contains();
    test_map_remove();
    test_map_update();
    test_map_empty();
    println!("[map_all] PASSED");
}

