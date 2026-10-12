#[inline]
pub fn test_u8_indexing() {
    let mask = vec![1_i32, 2_i32, 3_i32, 4_i32];
    let color_id = mask[0];
    if color_id == 0_i64 {
        return;
    }
    let result = color_id + 10_i64;
    assert_eq!(result, 11_i64);
}

