#[derive(Debug, Clone, Default)]

struct Node {
    value: i32,
    children: Vec<Node>,
}

impl Node {
#[inline]
pub fn new(value: i32) -> Node {
        Node { value, children: Vec::new() }
}
#[inline]
pub fn add_child(&mut self, child: Node) {
        self.children.push(child);
}
#[inline]
pub fn get_child(&self, index: usize) -> Node {
        (&self.children[index]).clone()
}
#[inline]
pub fn recursive_sum(node: &Node) -> i32 {
        let mut sum = node.value;
        for i in 0_usize..node.children.len() {
            let child = node.children[i].clone();
            sum += (Self::recursive_sum(&child)) as i32;
        }
        sum
}
}

#[inline]
pub fn test_vector_indexing() {
    let mut parent = Node::new(10_i32);
    parent.add_child(Node::new(5_i32));
    parent.add_child(Node::new(3_i32));
    let child = parent.get_child(0_usize);
    assert_eq!(child.value, 5);
    let sum = Node::recursive_sum(&parent);
    assert_eq!(sum, 18_i32);
}

