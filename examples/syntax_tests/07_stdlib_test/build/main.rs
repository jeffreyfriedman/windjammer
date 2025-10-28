use std::time::*;

use std::strings::*;

use std::math::*;


fn test_time() {
    let now = time.now();
    let formatted = time.to_rfc3339(&now);
    println!("Current time: {}", formatted);
    let ts = time.timestamp(&now);
    println!("Unix timestamp: {}", ts);
    let dur = time.hours(1);
    let future = time.add(&now, dur);
    println!("One hour from now: {}", time.to_rfc3339(&future))
}

fn test_strings() {
    let parts = strings.split("hello,world", ",");
    println!("Split result: {:?}", parts);
    let joined = strings.join(&parts, " | ");
    println!("Joined: {}", joined);
    let upper = strings.to_uppercase("hello");
    println!("Uppercase: {}", upper);
    let trimmed = strings.trim("  spaces  ");
    println!("Trimmed: '{}'", trimmed)
}

fn test_math() {
    let sqrt_val = math.sqrt(16.0);
    println!("sqrt(16) = {}", sqrt_val);
    let pow_val = math.pow(2.0, 8.0);
    println!("2^8 = {}", pow_val);
    let rounded = math.round(3.7);
    println!("round(3.7) = {}", rounded);
    let random = math.random();
    println!("Random: {}", random);
    let min_val = math.min_f64(5.0, 10.0);
    println!("min(5, 10) = {}", min_val)
}

fn main() {
    println!("=== Stdlib Module Tests ===");
    test_time();
    println!("");
    test_strings();
    println!("");
    test_math();
    println!("
✅ All tests completed!")
}

