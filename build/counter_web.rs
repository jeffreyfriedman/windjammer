


use windjammer_ui::prelude::*;

use windjammer_ui::vdom::{VElement, VNode, VText};


#[component]
struct Counter {
    count: i64,
}

impl Component for Counter {
#[inline]
fn render(&self) -> VNode {
        VElement::new("div").attr("class", "counter").child(VNode::Element(VElement::new("h1").attr("style", "font-family: Arial, sans-serif; color: #333;").child(VNode::Text(VText::new(format!("Count: {}", self.count)))))).child(VNode::Element(VElement::new("button").attr("onclick", "increment").attr("style", "padding: 10px 20px; font-size: 16px; cursor: pointer; background: #4CAF50; color: white; border: none; border-radius: 4px;").child(VNode::Text(VText::new("Increment"))))).into()
}
}

fn main() {
    let counter = Counter::new();
    let result = mount("#app", counter);
    println!("Counter mounted!")
}

