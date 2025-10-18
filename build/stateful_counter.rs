



use windjammer_ui::prelude::*;

use windjammer_ui::vdom::{VElement, VNode, VText}::*;

use windjammer_ui::reactivity::Signal::*;


#[component]
struct Counter {
    count: Signal<i64>,
}

impl Component for Counter {
#[inline]
fn render(&self) -> VNode {
        let current_count = self.count.get();
        VElement::new("div").attr("class", "counter-app").child(VNode::Element(VElement::new("h1").child(VNode::Text(VText::new("Reactive Counter"))))).child(VNode::Element(VElement::new("div").attr("class", "count-display").child(VNode::Text(VText::new(format!("Count: {}", current_count)))))).child(VNode::Element(VElement::new("div").attr("class", "button-group").child(VNode::Element(VElement::new("button").attr("onclick", "decrement").child(VNode::Text(VText::new("-"))))).child(VNode::Element(VElement::new("button").attr("onclick", "increment").child(VNode::Text(VText::new("+"))))).child(VNode::Element(VElement::new("button").attr("onclick", "reset").child(VNode::Text(VText::new("Reset"))))))).into()
}
}

impl Counter {
#[inline]
fn new(&self) -> Self {
        Self { count: Signal::new(0) }
}
#[inline]
fn increment(&self) {
        self.count.update(|c| {
            *c = *c + 1;
        })
}
#[inline]
fn decrement(&self) {
        self.count.update(|c| {
            *c = *c - 1;
        })
}
#[inline]
fn reset(&self) {
        self.count.set(0)
}
}

fn main() {
    let counter = Counter::new();
    counter.count::subscribe(|value| {
        println!(format!("Count changed to: {}", value));
    });
    mount("#app", counter)
}

