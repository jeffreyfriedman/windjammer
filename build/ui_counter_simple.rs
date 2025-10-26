



use windjammer_ui::reactivity::Signal;

use windjammer_ui::vdom::{VElement, VNode, VText};

use windjammer_ui::wasm_events::*;


struct CounterApp {
    count: Signal<i32>,
}

impl CounterApp {
#[inline]
fn new() -> CounterApp {
        CounterApp { count: Signal::new(0) }
}
#[inline]
fn render(self) -> VNode {
        let count_value = self.count.get();
        VNode::Element(VElement::new("div").attr("class", "counter").child(VNode::Element(VElement::new("h1").child(VNode::Text(VText::new(format!("Count: {}", count_value)))))).child(VNode::Element(VElement::new("button").attr("id", "increment").child(VNode::Text(VText::new("+"))))))
}
}

fn main() {
    let app = CounterApp::new();
    println!("Counter app created!")
}

