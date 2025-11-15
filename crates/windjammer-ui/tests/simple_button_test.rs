use std::sync::{Arc, Mutex};
use windjammer_ui::components::Button;
/// Simple test to verify button click handlers work
use windjammer_ui::prelude::*;
use windjammer_ui::simple_vnode::{VAttr, VNode};

#[test]
fn test_button_vnode_has_handler() {
    // Create a counter
    let click_count = Arc::new(Mutex::new(0));
    let click_count_clone = click_count.clone();

    // Create a button with a click handler
    let button = Button::new("Test Button").on_click(move || {
        let mut count = click_count_clone.lock().unwrap();
        *count += 1;
        println!("Handler called! Count: {}", *count);
    });

    // Convert to VNode
    let vnode = button.to_vnode();

    // Verify the VNode has the correct structure
    match &vnode {
        VNode::Element {
            tag,
            attrs,
            children,
        } => {
            println!("✅ VNode is an Element");
            println!("   Tag: {}", tag);
            assert_eq!(tag, "button", "Tag should be 'button'");

            println!("   Attributes:");
            for (name, attr) in attrs {
                match attr {
                    VAttr::Static(value) => println!("     {}: {} (static)", name, value),
                    VAttr::Dynamic(value) => println!("     {}: {} (dynamic)", name, value),
                    VAttr::Event(_) => println!("     {}: <handler> (event)", name),
                }
            }

            // Find the on_click handler
            let has_handler = attrs
                .iter()
                .any(|(name, attr)| name == "on_click" && matches!(attr, VAttr::Event(_)));

            assert!(has_handler, "Button should have an on_click handler");
            println!("✅ Button has on_click handler");

            // Try to call the handler directly
            if let Some((_, VAttr::Event(handler))) =
                attrs.iter().find(|(name, _)| name == "on_click")
            {
                println!("🔘 Calling handler directly...");
                handler.borrow_mut()();

                let count = *click_count.lock().unwrap();
                println!("✅ Handler executed! Count: {}", count);
                assert_eq!(count, 1, "Handler should have incremented count to 1");
            }
        }
        _ => panic!("VNode should be an Element"),
    }
}

#[test]
fn test_multiple_button_clicks() {
    let click_count = Arc::new(Mutex::new(0));
    let click_count_clone = click_count.clone();

    let button = Button::new("Multi Click").on_click(move || {
        let mut count = click_count_clone.lock().unwrap();
        *count += 1;
    });

    let vnode = button.to_vnode();

    // Extract and call the handler multiple times
    if let VNode::Element { attrs, .. } = &vnode {
        if let Some((_, VAttr::Event(handler))) = attrs.iter().find(|(name, _)| name == "on_click")
        {
            // Call 5 times
            for i in 1..=5 {
                handler.borrow_mut()();
                let count = *click_count.lock().unwrap();
                assert_eq!(count, i, "Count should be {} after {} clicks", i, i);
            }
            println!("✅ Handler called 5 times successfully");
        } else {
            panic!("No on_click handler found");
        }
    } else {
        panic!("VNode should be an Element");
    }
}
