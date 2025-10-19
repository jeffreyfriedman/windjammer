//! Integration tests for Windjammer UI framework

use windjammer_ui::vdom::{diff, VElement, VNode, VText};

mod vdom_tests {
    use super::*;

    #[test]
    fn test_simple_element_creation() {
        let node = VElement::new("div")
            .attr("class", "container")
            .child(VNode::Text(VText::new("Hello")))
            .into();

        if let VNode::Element(el) = node {
            assert_eq!(el.tag, "div");
            assert_eq!(el.attrs.get("class"), Some(&"container".to_string()));
            assert_eq!(el.children.len(), 1);
        } else {
            panic!("Expected Element node");
        }
    }

    #[test]
    fn test_nested_elements() {
        let node = VElement::new("div")
            .child(VNode::Element(
                VElement::new("span").child(VNode::Text(VText::new("Nested"))),
            ))
            .into();

        if let VNode::Element(el) = node {
            assert_eq!(el.children.len(), 1);
            if let VNode::Element(child) = &el.children[0] {
                assert_eq!(child.tag, "span");
            } else {
                panic!("Expected nested Element");
            }
        }
    }

    #[test]
    fn test_multiple_attributes() {
        let node = VElement::new("input")
            .attr("type", "text")
            .attr("placeholder", "Enter name")
            .attr("class", "form-control")
            .into();

        if let VNode::Element(el) = node {
            assert_eq!(el.attrs.len(), 3);
            assert_eq!(el.attrs.get("type"), Some(&"text".to_string()));
            assert_eq!(
                el.attrs.get("placeholder"),
                Some(&"Enter name".to_string())
            );
            assert_eq!(el.attrs.get("class"), Some(&"form-control".to_string()));
        }
    }

    #[test]
    fn test_multiple_children() {
        let node = VElement::new("ul")
            .child(VNode::Element(
                VElement::new("li").child(VNode::Text(VText::new("Item 1"))),
            ))
            .child(VNode::Element(
                VElement::new("li").child(VNode::Text(VText::new("Item 2"))),
            ))
            .child(VNode::Element(
                VElement::new("li").child(VNode::Text(VText::new("Item 3"))),
            ))
            .into();

        if let VNode::Element(el) = node {
            assert_eq!(el.children.len(), 3);
        }
    }
}

mod diff_tests {
    use super::*;
    use windjammer_ui::vdom::Patch;

    #[test]
    fn test_no_changes() {
        let old = VNode::Text(VText::new("Hello"));
        let new = VNode::Text(VText::new("Hello"));

        let patches = diff(&old, &new);
        assert_eq!(patches.len(), 0, "Identical nodes should produce no patches");
    }

    #[test]
    fn test_text_change() {
        let old = VNode::Text(VText::new("Hello"));
        let new = VNode::Text(VText::new("World"));

        let patches = diff(&old, &new);
        assert_eq!(patches.len(), 1);
        assert!(
            matches!(patches[0], Patch::UpdateText { .. }),
            "Should generate UpdateText patch"
        );
    }

    #[test]
    fn test_attribute_change() {
        let old = VElement::new("div").attr("class", "old").into();
        let new = VElement::new("div").attr("class", "new").into();

        let patches = diff(&old, &new);
        assert_eq!(patches.len(), 1);
        assert!(
            matches!(patches[0], Patch::SetAttribute { .. }),
            "Should generate SetAttribute patch"
        );
    }

    #[test]
    fn test_attribute_addition() {
        let old = VElement::new("div").into();
        let new = VElement::new("div").attr("class", "added").into();

        let patches = diff(&old, &new);
        assert_eq!(patches.len(), 1);
        assert!(matches!(patches[0], Patch::SetAttribute { .. }));
    }

    #[test]
    fn test_child_append() {
        let old = VElement::new("div").into();
        let new = VElement::new("div")
            .child(VNode::Text(VText::new("New child")))
            .into();

        let patches = diff(&old, &new);
        assert!(patches.len() >= 1, "Should have at least one patch");
        assert!(
            patches.iter().any(|p| matches!(p, Patch::Append { .. })),
            "Should generate Append patch"
        );
    }

    #[test]
    fn test_child_remove() {
        let old = VElement::new("div")
            .child(VNode::Text(VText::new("Will be removed")))
            .into();
        let new = VElement::new("div").into();

        let patches = diff(&old, &new);
        assert!(patches.len() >= 1);
        assert!(patches.iter().any(|p| matches!(p, Patch::Remove { .. })));
    }

    #[test]
    fn test_element_replacement() {
        let old = VElement::new("div").into();
        let new = VElement::new("span").into();

        let patches = diff(&old, &new);
        assert_eq!(patches.len(), 1);
        assert!(matches!(patches[0], Patch::Replace { .. }));
    }

    #[test]
    fn test_complex_diff() {
        let old = VElement::new("div")
            .attr("class", "old")
            .child(VNode::Text(VText::new("Old text")))
            .child(VNode::Element(VElement::new("span")))
            .into();

        let new = VElement::new("div")
            .attr("class", "new")
            .attr("id", "test")
            .child(VNode::Text(VText::new("New text")))
            .into();

        let patches = diff(&old, &new);
        assert!(patches.len() >= 3, "Complex diff should generate multiple patches");
    }
}

mod component_tests {
    use super::*;
    use windjammer_ui::component::Component;

    struct Counter {
        count: i32,
    }

    impl Component for Counter {
        fn render(&self) -> VNode {
            VElement::new("div")
                .attr("class", "counter")
                .child(VNode::Element(
                    VElement::new("span").child(VNode::Text(VText::new(format!("Count: {}", self.count)))),
                ))
                .child(VNode::Element(
                    VElement::new("button")
                        .attr("onclick", "increment")
                        .child(VNode::Text(VText::new("+")))
                ))
                .into()
        }
    }

    #[test]
    fn test_component_render() {
        let counter = Counter { count: 0 };
        let vnode = counter.render();

        if let VNode::Element(el) = vnode {
            assert_eq!(el.tag, "div");
            assert_eq!(el.attrs.get("class"), Some(&"counter".to_string()));
            assert_eq!(el.children.len(), 2);
        } else {
            panic!("Expected Element node");
        }
    }

    #[test]
    fn test_component_state_change() {
        let counter1 = Counter { count: 0 };
        let counter2 = Counter { count: 1 };

        let vnode1 = counter1.render();
        let vnode2 = counter2.render();

        let patches = diff(&vnode1, &vnode2);
        assert!(patches.len() > 0, "State change should produce patches");
    }
}

mod reactivity_tests {
    use super::*;
    use windjammer_ui::reactivity::Signal;
    use std::rc::Rc;
    use std::cell::RefCell;

    #[test]
    fn test_signal_creation() {
        let signal = Signal::new(42);
        assert_eq!(signal.get(), 42);
    }

    #[test]
    fn test_signal_update() {
        let signal = Signal::new(0);
        signal.set(10);
        assert_eq!(signal.get(), 10);
    }

    #[test]
    fn test_signal_subscribers() {
        let signal = Signal::new(0);
        let notified = Rc::new(RefCell::new(false));
        let notified_clone = notified.clone();

        signal.subscribe(move |_value| {
            *notified_clone.borrow_mut() = true;
        });

        assert!(!*notified.borrow());
        signal.set(1);
        assert!(*notified.borrow(), "Subscribers should be notified on set");
    }

    #[test]
    fn test_multiple_subscribers() {
        let signal = Signal::new(0);
        let count1 = Rc::new(RefCell::new(0));
        let count2 = Rc::new(RefCell::new(0));

        let count1_clone = count1.clone();
        let count2_clone = count2.clone();

        signal.subscribe(move |_value| {
            *count1_clone.borrow_mut() += 1;
        });

        signal.subscribe(move |_value| {
            *count2_clone.borrow_mut() += 1;
        });

        signal.set(1);
        assert_eq!(*count1.borrow(), 1);
        assert_eq!(*count2.borrow(), 1);

        signal.set(2);
        assert_eq!(*count1.borrow(), 2);
        assert_eq!(*count2.borrow(), 2);
    }
}

mod performance_tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_diff_performance_large_tree() {
        // Create a large tree
        let mut old_children = Vec::new();
        let mut new_children = Vec::new();

        for i in 0..100 {
            old_children.push(VNode::Element(
                VElement::new("div")
                    .attr("id", &format!("item-{}", i))
                    .child(VNode::Text(VText::new(format!("Item {}", i)))),
            ));
            new_children.push(VNode::Element(
                VElement::new("div")
                    .attr("id", &format!("item-{}", i))
                    .child(VNode::Text(VText::new(format!("Item {}", i)))),
            ));
        }

        let old = VElement::new("div").children(old_children).into();
        let new = VElement::new("div").children(new_children).into();

        let start = Instant::now();
        let patches = diff(&old, &new);
        let elapsed = start.elapsed();

        assert_eq!(patches.len(), 0, "Identical large trees should have no patches");
        assert!(
            elapsed.as_millis() < 100,
            "Diff of 100 identical nodes should be fast"
        );
    }

    #[test]
    fn test_diff_performance_with_changes() {
        let mut old_children = Vec::new();
        let mut new_children = Vec::new();

        for i in 0..100 {
            old_children.push(VNode::Text(VText::new(format!("Old {}", i))));
            new_children.push(VNode::Text(VText::new(format!("New {}", i))));
        }

        let old = VElement::new("div").children(old_children).into();
        let new = VElement::new("div").children(new_children).into();

        let start = Instant::now();
        let patches = diff(&old, &new);
        let elapsed = start.elapsed();

        assert_eq!(patches.len(), 100, "Should have 100 text update patches");
        assert!(
            elapsed.as_millis() < 100,
            "Diff with 100 text changes should be fast"
        );
    }
}


use windjammer_ui::vdom::{diff, VElement, VNode, VText};

mod vdom_tests {
    use super::*;

    #[test]
    fn test_simple_element_creation() {
        let node = VElement::new("div")
            .attr("class", "container")
            .child(VNode::Text(VText::new("Hello")))
            .into();

        if let VNode::Element(el) = node {
            assert_eq!(el.tag, "div");
            assert_eq!(el.attrs.get("class"), Some(&"container".to_string()));
            assert_eq!(el.children.len(), 1);
        } else {
            panic!("Expected Element node");
        }
    }

    #[test]
    fn test_nested_elements() {
        let node = VElement::new("div")
            .child(VNode::Element(
                VElement::new("span").child(VNode::Text(VText::new("Nested"))),
            ))
            .into();

        if let VNode::Element(el) = node {
            assert_eq!(el.children.len(), 1);
            if let VNode::Element(child) = &el.children[0] {
                assert_eq!(child.tag, "span");
            } else {
                panic!("Expected nested Element");
            }
        }
    }

    #[test]
    fn test_multiple_attributes() {
        let node = VElement::new("input")
            .attr("type", "text")
            .attr("placeholder", "Enter name")
            .attr("class", "form-control")
            .into();

        if let VNode::Element(el) = node {
            assert_eq!(el.attrs.len(), 3);
            assert_eq!(el.attrs.get("type"), Some(&"text".to_string()));
            assert_eq!(
                el.attrs.get("placeholder"),
                Some(&"Enter name".to_string())
            );
            assert_eq!(el.attrs.get("class"), Some(&"form-control".to_string()));
        }
    }

    #[test]
    fn test_multiple_children() {
        let node = VElement::new("ul")
            .child(VNode::Element(
                VElement::new("li").child(VNode::Text(VText::new("Item 1"))),
            ))
            .child(VNode::Element(
                VElement::new("li").child(VNode::Text(VText::new("Item 2"))),
            ))
            .child(VNode::Element(
                VElement::new("li").child(VNode::Text(VText::new("Item 3"))),
            ))
            .into();

        if let VNode::Element(el) = node {
            assert_eq!(el.children.len(), 3);
        }
    }
}

mod diff_tests {
    use super::*;
    use windjammer_ui::vdom::Patch;

    #[test]
    fn test_no_changes() {
        let old = VNode::Text(VText::new("Hello"));
        let new = VNode::Text(VText::new("Hello"));

        let patches = diff(&old, &new);
        assert_eq!(patches.len(), 0, "Identical nodes should produce no patches");
    }

    #[test]
    fn test_text_change() {
        let old = VNode::Text(VText::new("Hello"));
        let new = VNode::Text(VText::new("World"));

        let patches = diff(&old, &new);
        assert_eq!(patches.len(), 1);
        assert!(
            matches!(patches[0], Patch::UpdateText { .. }),
            "Should generate UpdateText patch"
        );
    }

    #[test]
    fn test_attribute_change() {
        let old = VElement::new("div").attr("class", "old").into();
        let new = VElement::new("div").attr("class", "new").into();

        let patches = diff(&old, &new);
        assert_eq!(patches.len(), 1);
        assert!(
            matches!(patches[0], Patch::SetAttribute { .. }),
            "Should generate SetAttribute patch"
        );
    }

    #[test]
    fn test_attribute_addition() {
        let old = VElement::new("div").into();
        let new = VElement::new("div").attr("class", "added").into();

        let patches = diff(&old, &new);
        assert_eq!(patches.len(), 1);
        assert!(matches!(patches[0], Patch::SetAttribute { .. }));
    }

    #[test]
    fn test_child_append() {
        let old = VElement::new("div").into();
        let new = VElement::new("div")
            .child(VNode::Text(VText::new("New child")))
            .into();

        let patches = diff(&old, &new);
        assert!(patches.len() >= 1, "Should have at least one patch");
        assert!(
            patches.iter().any(|p| matches!(p, Patch::Append { .. })),
            "Should generate Append patch"
        );
    }

    #[test]
    fn test_child_remove() {
        let old = VElement::new("div")
            .child(VNode::Text(VText::new("Will be removed")))
            .into();
        let new = VElement::new("div").into();

        let patches = diff(&old, &new);
        assert!(patches.len() >= 1);
        assert!(patches.iter().any(|p| matches!(p, Patch::Remove { .. })));
    }

    #[test]
    fn test_element_replacement() {
        let old = VElement::new("div").into();
        let new = VElement::new("span").into();

        let patches = diff(&old, &new);
        assert_eq!(patches.len(), 1);
        assert!(matches!(patches[0], Patch::Replace { .. }));
    }

    #[test]
    fn test_complex_diff() {
        let old = VElement::new("div")
            .attr("class", "old")
            .child(VNode::Text(VText::new("Old text")))
            .child(VNode::Element(VElement::new("span")))
            .into();

        let new = VElement::new("div")
            .attr("class", "new")
            .attr("id", "test")
            .child(VNode::Text(VText::new("New text")))
            .into();

        let patches = diff(&old, &new);
        assert!(patches.len() >= 3, "Complex diff should generate multiple patches");
    }
}

mod component_tests {
    use super::*;
    use windjammer_ui::component::Component;

    struct Counter {
        count: i32,
    }

    impl Component for Counter {
        fn render(&self) -> VNode {
            VElement::new("div")
                .attr("class", "counter")
                .child(VNode::Element(
                    VElement::new("span").child(VNode::Text(VText::new(format!("Count: {}", self.count)))),
                ))
                .child(VNode::Element(
                    VElement::new("button")
                        .attr("onclick", "increment")
                        .child(VNode::Text(VText::new("+")))
                ))
                .into()
        }
    }

    #[test]
    fn test_component_render() {
        let counter = Counter { count: 0 };
        let vnode = counter.render();

        if let VNode::Element(el) = vnode {
            assert_eq!(el.tag, "div");
            assert_eq!(el.attrs.get("class"), Some(&"counter".to_string()));
            assert_eq!(el.children.len(), 2);
        } else {
            panic!("Expected Element node");
        }
    }

    #[test]
    fn test_component_state_change() {
        let counter1 = Counter { count: 0 };
        let counter2 = Counter { count: 1 };

        let vnode1 = counter1.render();
        let vnode2 = counter2.render();

        let patches = diff(&vnode1, &vnode2);
        assert!(patches.len() > 0, "State change should produce patches");
    }
}

mod reactivity_tests {
    use super::*;
    use windjammer_ui::reactivity::Signal;
    use std::rc::Rc;
    use std::cell::RefCell;

    #[test]
    fn test_signal_creation() {
        let signal = Signal::new(42);
        assert_eq!(signal.get(), 42);
    }

    #[test]
    fn test_signal_update() {
        let signal = Signal::new(0);
        signal.set(10);
        assert_eq!(signal.get(), 10);
    }

    #[test]
    fn test_signal_subscribers() {
        let signal = Signal::new(0);
        let notified = Rc::new(RefCell::new(false));
        let notified_clone = notified.clone();

        signal.subscribe(move |_value| {
            *notified_clone.borrow_mut() = true;
        });

        assert!(!*notified.borrow());
        signal.set(1);
        assert!(*notified.borrow(), "Subscribers should be notified on set");
    }

    #[test]
    fn test_multiple_subscribers() {
        let signal = Signal::new(0);
        let count1 = Rc::new(RefCell::new(0));
        let count2 = Rc::new(RefCell::new(0));

        let count1_clone = count1.clone();
        let count2_clone = count2.clone();

        signal.subscribe(move |_value| {
            *count1_clone.borrow_mut() += 1;
        });

        signal.subscribe(move |_value| {
            *count2_clone.borrow_mut() += 1;
        });

        signal.set(1);
        assert_eq!(*count1.borrow(), 1);
        assert_eq!(*count2.borrow(), 1);

        signal.set(2);
        assert_eq!(*count1.borrow(), 2);
        assert_eq!(*count2.borrow(), 2);
    }
}

mod performance_tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_diff_performance_large_tree() {
        // Create a large tree
        let mut old_children = Vec::new();
        let mut new_children = Vec::new();

        for i in 0..100 {
            old_children.push(VNode::Element(
                VElement::new("div")
                    .attr("id", &format!("item-{}", i))
                    .child(VNode::Text(VText::new(format!("Item {}", i)))),
            ));
            new_children.push(VNode::Element(
                VElement::new("div")
                    .attr("id", &format!("item-{}", i))
                    .child(VNode::Text(VText::new(format!("Item {}", i)))),
            ));
        }

        let old = VElement::new("div").children(old_children).into();
        let new = VElement::new("div").children(new_children).into();

        let start = Instant::now();
        let patches = diff(&old, &new);
        let elapsed = start.elapsed();

        assert_eq!(patches.len(), 0, "Identical large trees should have no patches");
        assert!(
            elapsed.as_millis() < 100,
            "Diff of 100 identical nodes should be fast"
        );
    }

    #[test]
    fn test_diff_performance_with_changes() {
        let mut old_children = Vec::new();
        let mut new_children = Vec::new();

        for i in 0..100 {
            old_children.push(VNode::Text(VText::new(format!("Old {}", i))));
            new_children.push(VNode::Text(VText::new(format!("New {}", i))));
        }

        let old = VElement::new("div").children(old_children).into();
        let new = VElement::new("div").children(new_children).into();

        let start = Instant::now();
        let patches = diff(&old, &new);
        let elapsed = start.elapsed();

        assert_eq!(patches.len(), 100, "Should have 100 text update patches");
        assert!(
            elapsed.as_millis() < 100,
            "Diff with 100 text changes should be fast"
        );
    }
}

