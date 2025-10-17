//! Simple counter example using windjammer-ui

use windjammer_ui::prelude::*;
use windjammer_ui::{
    component,
    vdom::{VElement, VNode, VText},
};

#[component]
struct Counter {
    count: i32,
}

impl Counter {
    fn render(&self) -> VNode {
        VElement::new("div")
            .attr("class", "counter")
            .child(VNode::Element(VElement::new("h1").child(VNode::Text(
                VText::new(format!("Count: {}", self.count)),
            ))))
            .child(VNode::Element(
                VElement::new("button")
                    .attr("onclick", "increment")
                    .child(VNode::Text(VText::new("Increment"))),
            ))
            .into()
    }
}

impl Component for Counter {
    fn render(&self) -> VNode {
        self.render()
    }
}

fn main() {
    // Create a counter component
    let counter = Counter::new();
    println!("Counter component created");

    // Create with initial state
    let counter_with_state = Counter::with_state(42);
    println!("Counter with state 42 created");

    // Render the component
    let vnode = counter.render();
    println!("Rendered VNode: {:?}", vnode);

    let vnode_with_state = counter_with_state.render();
    println!("Rendered VNode with state: {:?}", vnode_with_state);

    println!("\n✅ Counter example completed successfully!");
}
