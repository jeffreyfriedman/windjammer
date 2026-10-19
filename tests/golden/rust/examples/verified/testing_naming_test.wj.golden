use windjammer_runtime::test::*;
#[inline]
fn clamp01(x: f64) -> f64 {
    if x < 0.0_f64 {
        0.0_f64
    } else {
        if x > 1.0_f64 {
            1.0_f64
        } else {
            x
        }
    }
}

#[test]

fn test_clamp_low() {
    assert_eq!(clamp01(-2.0_f64), 0.0_f64);
}

#[test]
fn upper_bound() {
    assert_eq!(clamp01(5.5_f64), 1.0_f64);
}

