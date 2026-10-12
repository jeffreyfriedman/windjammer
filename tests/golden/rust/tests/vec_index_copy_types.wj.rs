#[derive(Debug, Clone, Default)]

struct Node {
    value: i32,
    children: Vec<Node>,
}

#[inline]
pub fn test_copy_type_indexing() {
    let numbers = vec![1_i32, 2_i32, 3_i32, 4_i32, 5_i32];
    let x = numbers[0];
    assert_eq!(x, 1_i64);
    let bytes = vec![10_i32, 20_i32, 30_i32];
    let b = bytes[1];
    assert_eq!(b, 20_i64);
    let floats = vec![1.0_f64, 2.0_f64, 3.0_f64];
    let f = floats[2];
    assert_eq!(f, 3.0_f64);
}

#[inline]
pub fn test_non_copy_indexing() {
    let nodes = vec![Node { value: 1_i32, children: Vec::new() }];
    let node = &nodes[0];
    assert_eq!(node.value, 1);
}

