use std::collections::HashMap;
use windjammer_ui::simple_renderer::SimpleRenderer;
use windjammer_ui::vdom::{VElement, VNode, VText};

fn main() {
    println!("=== Windjammer UI Counter Demo ===\n");

    // Simulate a counter component
    let mut count = 0;
    let mut renderer = SimpleRenderer::new();

    // Initial render
    let vnode = VElement {
        tag: "div".to_string(),
        attrs: HashMap::new(),
        children: vec![
            VNode::Text(VText::new(format!("Count: {}", count))),
            VNode::Element(VElement {
                tag: "button".to_string(),
                attrs: {
                    let mut attrs = HashMap::new();
                    attrs.insert("onclick".to_string(), "increment".to_string());
                    attrs
                },
                children: vec![VNode::Text(VText::new("Increment"))],
            }),
        ],
    };

    renderer.render(&VNode::Element(vnode));
    println!("Initial Render:\n{}\n", renderer.get_output());

    // Simulate 3 button clicks
    for i in 1..=3 {
        count += 1;

        let updated_vnode = VElement {
            tag: "div".to_string(),
            attrs: HashMap::new(),
            children: vec![
                VNode::Text(VText::new(format!("Count: {}", count))),
                VNode::Element(VElement {
                    tag: "button".to_string(),
                    attrs: {
                        let mut attrs = HashMap::new();
                        attrs.insert("onclick".to_string(), "increment".to_string());
                        attrs
                    },
                    children: vec![VNode::Text(VText::new("Increment"))],
                }),
            ],
        };

        renderer.render(&VNode::Element(updated_vnode));
        println!("After Click #{i}:\n{}\n", renderer.get_output());
    }

    println!("✅ Counter demo complete!");
}
