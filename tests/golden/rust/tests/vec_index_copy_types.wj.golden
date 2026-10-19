#[derive(Debug, Clone, Default)]

struct Node {
    value: i32,
    children: Vec<Node>,
}

#[inline]
pub fn test_copy_type_indexing() {
    let numbers = vec![1_i64, 2_i64, 3_i64, 4_i64, 5_i64];
    let x = numbers[0];
    assert_eq!(x, 1_i64);
    let bytes = vec![10_i64, 20_i64, 30_i64];
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

