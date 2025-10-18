#[component]
struct Counter {
    count: i64,
}

impl Counter {
#[inline]
fn render(&self) -> VNode {
        VElement::new("div").attr("class", "counter").child(VNode::Element(VElement::new("h1").child(VNode::Text(VText::new(format!("Count: {}", self.count)))))).child(VNode::Element(VElement::new("button").attr("onclick", "increment").child(VNode::Text(VText::new("Increment"))))).into()
}
}

fn main() {
    let counter = Counter::new();
    println!("Counter component created");
    let counter_with_state = Counter::with_state(42);
    println!("Counter with state 42 created");
    let vnode = counter.render();
    println!("Rendered VNode: {vnode:?}");
    let vnode_with_state = counter_with_state.render();
    println!("Rendered VNode with state: {vnode_with_state:?}");
    println!("
✅ Counter example completed successfully!")
}

