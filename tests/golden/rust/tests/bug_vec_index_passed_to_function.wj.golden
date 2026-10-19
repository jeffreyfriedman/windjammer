#[derive(Debug, Clone, PartialEq, Eq, Default)]

struct Node {
    value: i32,
    data: Vec<i32>,
}

#[inline]
fn process_node(node: &Node) -> i32 {
    node.value * 2_i32
}

#[inline]
pub fn test_vec_index_then_call() {
    let nodes = vec![Node { value: 10_i32 }, Node { value: 20_i32 }, Node { value: 30_i32 }];
    let result = recursive_process(&nodes, 0_i32);
    assert_eq!(result, 120_i32);
}

#[inline]
pub fn test_vec_index_direct_call() {
    let nodes = vec![Node { value: 5_i32 }];
    let node = nodes[0].clone();
    let result = process_node(&node);
    assert_eq!(result, 10_i32);
}

#[inline]
fn recursive_process(nodes: &Vec<Node>, idx: i32) -> i32 {
    if idx >= nodes.len() as i32 {
        return 0_i32;
    }
    let node = nodes[idx as usize].clone();
    let result = process_node(&node);
    (result + recursive_process(nodes, idx + 1_i32)) as i32
}

