


use windjammer_ui::prelude::*;

use windjammer_ui::vdom::{VElement, VNode, VText};


#[component]
struct Counter {
    count: i64,
}

impl Counter {
#[inline]
fn render() -> VNode {
        VElement::new("div").attr("class", "counter").child(VNode::Element(VElement::new("h1").child(VNode::Text(VText::new("Count: {count}"))))).child(VNode::Element(VElement::new("button").attr("onclick", "increment").child(VNode::Text(VText::new("Increment"))))).into()
}
}

fn main() {
    let counter = Counter::new();
    print("Counter component created");
    let counter_with_state = Counter::with_state(42);
    print("Counter with state 42 created");
    let vnode = counter.render();
    print("Rendered VNode: {vnode:?}");
    let vnode_with_state = counter_with_state.render();
    print("Rendered VNode with state: {vnode_with_state:?}");
    print("
✅ Counter example completed successfully!")
}

