pub mod math {
pub const PI: f64 = 3.141592653589793;
pub const E: f64 = 2.718281828459045;
pub const TAU: f64 = 6.283185307179586;
pub const SQRT_2: f64 = 1.4142135623730951;

pub fn abs_f64(x: f64) -> f64 {
    x.abs()
}

pub fn abs_i64(x: i64) -> i64 {
    x.abs()
}

pub fn pow(base: f64, exp: f64) -> f64 {
    base.powf(exp)
}

pub fn sqrt(x: f64) -> f64 {
    x.sqrt()
}

pub fn cbrt(x: f64) -> f64 {
    x.cbrt()
}

pub fn round(x: f64) -> f64 {
    x.round()
}

pub fn floor(x: f64) -> f64 {
    x.floor()
}

pub fn ceil(x: f64) -> f64 {
    x.ceil()
}

pub fn trunc(x: f64) -> f64 {
    x.trunc()
}

pub fn sin(x: f64) -> f64 {
    x.sin()
}

pub fn cos(x: f64) -> f64 {
    x.cos()
}

pub fn tan(x: f64) -> f64 {
    x.tan()
}

pub fn asin(x: f64) -> f64 {
    x.asin()
}

pub fn acos(x: f64) -> f64 {
    x.acos()
}

pub fn atan(x: f64) -> f64 {
    x.atan()
}

pub fn atan2(y: f64, x: f64) -> f64 {
    y.atan2(x)
}

pub fn sinh(x: f64) -> f64 {
    x.sinh()
}

pub fn cosh(x: f64) -> f64 {
    x.cosh()
}

pub fn tanh(x: f64) -> f64 {
    x.tanh()
}

pub fn exp(x: f64) -> f64 {
    x.exp()
}

pub fn exp2(x: f64) -> f64 {
    x.exp2()
}

pub fn ln(x: f64) -> f64 {
    x.ln()
}

pub fn log2(x: f64) -> f64 {
    x.log2()
}

pub fn log10(x: f64) -> f64 {
    x.log10()
}

pub fn log(x: f64, base: f64) -> f64 {
    x.log(base)
}

pub fn min_f64(a: f64, b: f64) -> f64 {
    a.min(b)
}

pub fn min_i64(a: i64, b: i64) -> i64 {
    a.min(b)
}

pub fn max_f64(a: f64, b: f64) -> f64 {
    a.max(b)
}

pub fn max_i64(a: i64, b: i64) -> i64 {
    a.max(b)
}

pub fn clamp_f64(value: f64, min: f64, max: f64) -> f64 {
    value.clamp(min, max)
}

pub fn clamp_i64(value: i64, min: i64, max: i64) -> i64 {
    value.clamp(min, max)
}

pub fn signum(x: f64) -> f64 {
    x.signum()
}

pub fn copysign(x: f64, y: f64) -> f64 {
    x.copysign(y)
}

pub fn hypot(x: f64, y: f64) -> f64 {
    x.hypot(y)
}

pub fn fma(x: f64, y: f64, z: f64) -> f64 {
    x.mul_add(y, z)
}

pub fn is_nan(x: f64) -> bool {
    x.is_nan()
}

pub fn is_infinite(x: f64) -> bool {
    x.is_infinite()
}

pub fn is_finite(x: f64) -> bool {
    x.is_finite()
}

pub fn to_radians(degrees: f64) -> f64 {
    degrees.to_radians()
}

pub fn to_degrees(radians: f64) -> f64 {
    radians.to_degrees()
}


}


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


use math::*;

use strings::*;


fn main() {
    println!("===== Module Aliases Demo =====");
    println!("");
    println!("Module aliases make imports cleaner:");
    println!("  Windjammer: use std.math as m");
    println!("  Rust:       use std::math as m");
    println!("");
    println!("Using math functions:");
    let pi_value = PI;
    println!("  PI = {}", pi_value);
    let sqrt_result = sqrt(16.0);
    println!("  sqrt(16) = {}", sqrt_result);
    let power = pow(2.0, 8.0);
    println!("  2^8 = {}", power);
    println!("");
    println!("Using string functions:");
    let text = "  windjammer  ";
    let cleaned = trim(text);
    println!("  trim('{}') = '{}'", text, cleaned);
    let upper = to_upper("hello");
    println!("  to_upper('hello') = '{}'", upper);
    let contains_result = contains("windjammer", "jam");
    println!("  contains('windjammer', 'jam') = {}", contains_result);
    println!("");
    println!("✓ Module aliases work!");
    println!("✓ (Full :: expression support coming in future version)")
}

