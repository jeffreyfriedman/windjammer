//! ToVNode trait for converting components to VNode
//!
//! This trait allows any UI component to be converted to a VNode,
//! enabling them to be used as children in other components.

use crate::simple_vnode::VNode;

/// Trait for types that can be converted to a VNode
pub trait ToVNode {
    /// Convert this component to a VNode
    fn to_vnode(self) -> VNode;
}

// Implement for VNode itself (identity conversion)
impl ToVNode for VNode {
    fn to_vnode(self) -> VNode {
        self
    }
}

// Implement for String (text node)
impl ToVNode for String {
    fn to_vnode(self) -> VNode {
        VNode::Text(self)
    }
}

// Implement for &str (text node)
impl ToVNode for &str {
    fn to_vnode(self) -> VNode {
        VNode::Text(self.to_string())
    }
}

// Implement for Vec<VNode> (fragment)
impl ToVNode for Vec<VNode> {
    fn to_vnode(self) -> VNode {
        // Wrap in a fragment div
        VNode::element("div", vec![], self)
    }
}
