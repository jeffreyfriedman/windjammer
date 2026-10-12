#[inline]
pub fn test_integer_comparison_before_indexing() {
    let data = vec![1.0_f64, 2.0_f64, 3.0_f64, 4.0_f64, 5.0_f64];
    let n = data.len() as i32;
    let mut idx = n / 2_i32;
    if idx >= n {
        idx = n - 1_i32;
    }
    let value = data[(idx as usize)];
    assert!(value == 3.0_f64, "Should get middle element");
}

#[inline]
pub fn test_integer_arithmetic_then_indexing() {
    let nums = vec![10_i32, 20_i32, 30_i32, 40_i32, 50_i32];
    let count = nums.len() as i32;
    let offset = count / 2_i32;
    let adjusted = offset + 1_i32;
    if adjusted < count {
        let val = nums[(adjusted as usize)];
        let _ = assert!(val == 30_i64, "Should index correctly");
    }
}

#[inline]
pub fn test_percentage_calculation_then_indexing() {
    let values = vec![1_i32, 2_i32, 3_i32, 4_i32, 5_i32, 6_i32, 7_i32, 8_i32, 9_i32, 10_i32];
    let n = values.len() as i32;
    let pct = 50_i32;
    let numer = n * pct;
    let idx = numer / 100_i32;
    assert!(idx >= 0_i32, "Index should be non-negative");
    assert!(idx < n, "Index should be in bounds");
    let result = values[(idx as usize)];
    assert!(result == 5_i64, "Should get 50th percentile");
}

