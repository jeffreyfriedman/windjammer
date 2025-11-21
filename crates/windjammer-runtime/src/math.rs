//! Mathematical functions
//!
//! Windjammer's `std::math` module maps to these functions.

/// Absolute value
pub fn abs_i64(x: i64) -> i64 {
    x.abs()
}

pub fn abs_f64(x: f64) -> f64 {
    x.abs()
}

/// Power
pub fn pow_f64(base: f64, exp: f64) -> f64 {
    base.powf(exp)
}

pub fn pow_i32(base: i32, exp: u32) -> i32 {
    base.pow(exp)
}

/// Square root
pub fn sqrt(x: f64) -> f64 {
    x.sqrt()
}

/// Ceiling
pub fn ceil(x: f64) -> f64 {
    x.ceil()
}

/// Floor
pub fn floor(x: f64) -> f64 {
    x.floor()
}

/// Round
pub fn round(x: f64) -> f64 {
    x.round()
}

/// Minimum
pub fn min_i64(a: i64, b: i64) -> i64 {
    a.min(b)
}

pub fn min_f64(a: f64, b: f64) -> f64 {
    a.min(b)
}

/// Maximum
pub fn max_i64(a: i64, b: i64) -> i64 {
    a.max(b)
}

pub fn max_f64(a: f64, b: f64) -> f64 {
    a.max(b)
}

/// Trigonometric functions
pub fn sin(x: f64) -> f64 {
    x.sin()
}

pub fn cos(x: f64) -> f64 {
    x.cos()
}

pub fn tan(x: f64) -> f64 {
    x.tan()
}

/// Constants
pub const PI: f64 = std::f64::consts::PI;
pub const E: f64 = std::f64::consts::E;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_abs() {
        assert_eq!(abs_i64(-5), 5);
        assert_eq!(abs_f64(-2.5), 2.5);
    }

    #[test]
    fn test_sqrt() {
        assert_eq!(sqrt(4.0), 2.0);
        assert_eq!(sqrt(9.0), 3.0);
    }

    #[test]
    fn test_round() {
        assert_eq!(round(3.7), 4.0);
        assert_eq!(round(3.2), 3.0);
    }

    #[test]
    fn test_min_max() {
        assert_eq!(min_i64(5, 10), 5);
        assert_eq!(max_i64(5, 10), 10);
    }
}
