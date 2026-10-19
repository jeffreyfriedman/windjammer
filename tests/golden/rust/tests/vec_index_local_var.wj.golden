#[derive(Debug, Clone, Default)]

struct Node {
    value: i32,
    children: Option<Vec<Node>>,
}

impl Node {
#[inline]
pub fn new(value: i32) -> Node {
        Node { value, children: None }
}
#[inline]
pub fn direct_access(mut self, index: usize) -> Node {
        let children = self.children.unwrap();
        (&children[index]).clone()
}
#[inline]
pub fn through_self(mut self, index: usize) -> Node {
        let children_ref = self.children.unwrap();
        (&children_ref[index]).clone()
}
}

#[inline]
pub fn test_local_var_indexing() {
    let mut node = Node::new(10_i32);
    node.children = Some(Vec::new());
    node.children.unwrap().push(Node::new(5_i32));
    let child = node.direct_access(0_usize);
    assert_eq!(child.value, 5);
}

