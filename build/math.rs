const PI: f64 = 3.141592653589793;
const E: f64 = 2.718281828459045;
const TAU: f64 = 6.283185307179586;
const SQRT_2: f64 = 1.4142135623730951;

#[inline]
fn abs_f64(x: f64) -> f64 {
    x.abs()
}

#[inline]
fn abs_i64(x: i64) -> i64 {
    x.abs()
}

#[inline]
fn pow(base: f64, exp: f64) -> f64 {
    base.powf(exp)
}

#[inline]
fn sqrt(x: f64) -> f64 {
    x.sqrt()
}

#[inline]
fn cbrt(x: f64) -> f64 {
    x.cbrt()
}

#[inline]
fn round(x: f64) -> f64 {
    x.round()
}

#[inline]
fn floor(x: f64) -> f64 {
    x.floor()
}

#[inline]
fn ceil(x: f64) -> f64 {
    x.ceil()
}

#[inline]
fn trunc(x: f64) -> f64 {
    x.trunc()
}

#[inline]
fn sin(x: f64) -> f64 {
    x.sin()
}

#[inline]
fn cos(x: f64) -> f64 {
    x.cos()
}

#[inline]
fn tan(x: f64) -> f64 {
    x.tan()
}

#[inline]
fn asin(x: f64) -> f64 {
    x.asin()
}

#[inline]
fn acos(x: f64) -> f64 {
    x.acos()
}

#[inline]
fn atan(x: f64) -> f64 {
    x.atan()
}

#[inline]
fn atan2(y: f64, x: f64) -> f64 {
    y.atan2(x)
}

#[inline]
fn sinh(x: f64) -> f64 {
    x.sinh()
}

#[inline]
fn cosh(x: f64) -> f64 {
    x.cosh()
}

#[inline]
fn tanh(x: f64) -> f64 {
    x.tanh()
}

#[inline]
fn exp(x: f64) -> f64 {
    x.exp()
}

#[inline]
fn exp2(x: f64) -> f64 {
    x.exp2()
}

#[inline]
fn ln(x: f64) -> f64 {
    x.ln()
}

#[inline]
fn log2(x: f64) -> f64 {
    x.log2()
}

#[inline]
fn log10(x: f64) -> f64 {
    x.log10()
}

#[inline]
fn log(x: f64, base: f64) -> f64 {
    x.log(base)
}

#[inline]
fn min_f64(a: f64, b: f64) -> f64 {
    a.min(b)
}

#[inline]
fn min_i64(a: i64, b: i64) -> i64 {
    a.min(b)
}

#[inline]
fn max_f64(a: f64, b: f64) -> f64 {
    a.max(b)
}

#[inline]
fn max_i64(a: i64, b: i64) -> i64 {
    a.max(b)
}

#[inline]
fn clamp_f64(value: f64, min: f64, max: f64) -> f64 {
    value.clamp(min, max)
}

#[inline]
fn clamp_i64(value: i64, min: i64, max: i64) -> i64 {
    value.clamp(min, max)
}

#[inline]
fn signum(x: f64) -> f64 {
    x.signum()
}

#[inline]
fn copysign(x: f64, y: f64) -> f64 {
    x.copysign(y)
}

#[inline]
fn hypot(x: f64, y: f64) -> f64 {
    x.hypot(y)
}

#[inline]
fn fma(x: f64, y: f64, z: f64) -> f64 {
    x.mul_add(y, z)
}

#[inline]
fn is_nan(x: f64) -> bool {
    x.is_nan()
}

#[inline]
fn is_infinite(x: f64) -> bool {
    x.is_infinite()
}

#[inline]
fn is_finite(x: f64) -> bool {
    x.is_finite()
}

#[inline]
fn to_radians(degrees: f64) -> f64 {
    degrees.to_radians()
}

#[inline]
fn to_degrees(radians: f64) -> f64 {
    radians.to_degrees()
}

