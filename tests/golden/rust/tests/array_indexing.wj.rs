#[inline]
fn get_element(arr: &Vec<i32>, index: i64) -> i32 {
    arr[index as usize]
}

#[inline]
fn get_element_with_cast(arr: &Vec<i32>, index: i64) -> i32 {
    arr[index as usize]
}

#[inline]
fn test_array_access() -> i32 {
    let numbers = vec![10_i32, 20_i32, 30_i32, 40_i32, 50_i32];
    let idx = 2_i32;
    (numbers[(idx as usize)]) as i32
}

