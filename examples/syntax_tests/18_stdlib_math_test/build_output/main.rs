pub mod math {
const PI: f64 = 3.141592653589793;
const E: f64 = 2.718281828459045;
const TAU: f64 = 6.283185307179586;
const SQRT_2: f64 = 1.4142135623730951;

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


use math::*;


fn main() {
    println!("Testing std/math module...");
    let x = 16.0;
    let root = sqrt(x);
    println!("sqrt(16.0) = {}", root);
    let y = -5.5;
    let abs_y = abs_f64(y);
    println!("abs(-5.5) = {}", abs_y);
    let z = 2.0;
    let exp = 3.0;
    let power = pow(z, exp);
    println!("pow(2.0, 3.0) = {}", power);
    let pi = 3.14159;
    let rounded = round(pi);
    println!("round(3.14159) = {}", rounded);
    let floored = floor(pi);
    println!("floor(3.14159) = {}", floored);
    let ceiled = ceil(pi);
    println!("ceil(3.14159) = {}", ceiled);
    println!("std/math works! ✓")
}

