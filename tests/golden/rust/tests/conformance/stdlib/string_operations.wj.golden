#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]

struct Greeter {
    prefix: String,
}

impl Greeter {
#[inline]
fn greet(&self, name: &str) -> String {
        format!("{}{}{}{}", self.prefix, ", ", name, "!")
}
}

#[inline]
fn test_str_literal() {
    let s: String = String::from("hello");
    println!("[str_literal] {}", s);
}

#[inline]
fn test_str_concat() {
    let a: String = String::from("hello");
    let b: String = String::from(" world");
    let c = a + &b;
    println!("[str_concat] {}", c);
}

#[inline]
fn test_str_interpolation() {
    let name: String = String::from("Alice");
    let age = 30_i64;
    println!("[str_interp] name={}, age={}", name, age);
    let x = 2_i64;
    let y = 3_i64;
    let sum = x + y;
    println!("[str_interp] result: {} + {} = {}", x, y, sum);
}

#[inline]
fn test_str_len() {
    let empty = String::new();
    let hello: String = String::from("hello");
    let emoji: String = String::from("🎉");
    println!("[str_len] empty={}, hello={}, emoji={}", empty.len(), hello.len(), emoji.len());
}

#[inline]
fn test_str_equality() {
    let a: String = String::from("test");
    let b: String = String::from("test");
    let c: String = String::from("other");
    println!("[str_eq] same: {}", a == b);
    println!("[str_eq] different: {}", a == c);
    println!("[str_eq] not_equal: {}", a != c);
}

#[inline]
fn test_str_multi_concat() {
    let parts = vec!["a".to_string(), "b".to_string(), "c".to_string(), "d".to_string()];
    let mut result = String::new();
    for part in parts {
        if !result.is_empty() {
            { use std::fmt::Write as _; write!(result, "{}", "-").expect("a formatting trait implementation returned an error"); };
        }
        result = result + &part;
    }
    println!("[str_multi_concat] {}", result);
}

#[inline]
fn test_str_in_struct() {
    let g = Greeter { prefix: String::from("Hello") };
    let msg = g.greet("Bob");
    println!("[str_in_struct] greeting: {}", msg);
}

#[inline]
fn test_str_empty() {
    let mut s = String::new();
    println!("[str_empty] is empty: {}", s.is_empty());
    s = String::from("hello");
    println!("[str_empty] after assign: {}", s.is_empty());
}

fn main() {
    test_str_literal();
    test_str_concat();
    test_str_interpolation();
    test_str_len();
    test_str_equality();
    test_str_multi_concat();
    test_str_in_struct();
    test_str_empty();
    println!("[str_all] PASSED");
}

