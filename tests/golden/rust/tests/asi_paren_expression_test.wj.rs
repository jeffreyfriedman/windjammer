use windjammer_runtime::test::*;
#[test]

pub fn test_asi_before_paren() -> i64 {
    let a = 5;
    (a + 1_i64) * 2_i64
}

#[test]

pub fn test_asi_multiple_statements() -> f32 {
    let dx = 3.0_f32;
    let dy = 4.0_f32;
    let dz = 5.0_f32;
    (dx * dx + dy * dy + dz * dz).sqrt()
}

