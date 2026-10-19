#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]

struct Node {
    value: i32,
}









#[inline]
fn process_node(node: Node) -> i32 {
    node.value * 2_i32
}

#[inline]
pub fn test_vec_ref_then_deref() {
    let nodes = vec![Node { value: 10_i32 }, Node { value: 20_i32 }, Node { value: 30_i32 }];
    let node_ref = &nodes[1];
    let result = process_node(*node_ref);
    assert_eq!(result, 40_i32);
}

#[inline]
pub fn test_vec_ref_deref_in_call() {
    let nodes = vec![Node { value: 5_i32 }, Node { value: 15_i32 }];
    let result = process_node(*&nodes[0]);
    assert_eq!(result, 10_i32);
}

