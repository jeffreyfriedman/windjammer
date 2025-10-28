pub mod strings {
pub fn to_upper(s: &str) -> String {
    s.to_uppercase()
}

pub fn to_lower(s: &str) -> String {
    s.to_lowercase()
}

pub fn trim(s: &str) -> String {
    s.trim().to_string()
}

pub fn trim_start(s: &str) -> String {
    s.trim_start().to_string()
}

pub fn trim_end(s: &str) -> String {
    s.trim_end().to_string()
}

pub fn is_empty(s: &str) -> bool {
    s.is_empty()
}

pub fn starts_with(s: &str, prefix: &str) -> bool {
    s.starts_with(prefix)
}

pub fn ends_with(s: &str, suffix: &str) -> bool {
    s.ends_with(suffix)
}

pub fn contains(s: &str, substring: &str) -> bool {
    s.contains(substring)
}

pub fn replace(s: &str, from: &str, to: &str) -> String {
    s.replace(from, to)
}

pub fn replacen(s: &str, from: &str, to: &str, count: usize) -> String {
    s.replacen(from, to, count)
}

pub fn len(s: &str) -> usize {
    s.len()
}

pub fn char_count(s: &str) -> usize {
    s.chars().count()
}

pub fn repeat(s: &str, n: usize) -> String {
    s.repeat(n)
}


}


use strings::*;


fn main() {
    println!("Testing std/strings module...");
    let text = "windjammer";
    let upper = to_upper(text);
    println!("to_upper('windjammer') = {}", upper);
    let lower_text = "HELLO";
    let lower = to_lower(lower_text);
    println!("to_lower('HELLO') = {}", lower);
    let padded = "  hello  ";
    let trimmed = trim(padded);
    println!("trim('  hello  ') = '{}'", trimmed);
    let original = "hello world";
    let replaced = replace(original, "world", "Windjammer");
    println!("replace('hello world', 'world', 'Windjammer') = {}", replaced);
    let empty = "";
    let is_empty_result = is_empty(&empty);
    println!("is_empty('') = {}", is_empty_result);
    let test_str = "windjammer";
    let starts = starts_with(&test_str, "wind");
    println!("starts_with('windjammer', 'wind') = {}", starts);
    let ends = ends_with(&test_str, "mer");
    println!("ends_with('windjammer', 'mer') = {}", ends);
    let has = contains(&test_str, "jam");
    println!("contains('windjammer', 'jam') = {}", has);
    let repeated = repeat("ha", 3);
    println!("repeat('ha', 3) = {}", repeated);
    println!("std/strings works! ✓")
}

