//! Component model and traits

use crate::vdom::VNode;

/// Trait for components
///
/// Note: The `render()` method should be implemented as a regular method,
/// not as part of this trait. The `#[component]` macro will ensure the
/// struct implements this trait.
pub trait Component: Send + Sync {
    /// Initialize the component
    fn init(&mut self) {}

    /// Update the component with new props
    fn update(&mut self) {}

    /// Cleanup when component is unmounted
    fn cleanup(&mut self) {}

    /// Render the component to a virtual DOM node
    /// This is provided as a trait method for type safety
    fn render(&self) -> VNode;
}

/// Props trait for component properties
pub trait ComponentProps: Clone + Send + Sync {}

/// Implement ComponentProps for unit (components with no props)
impl ComponentProps for () {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vdom::VText;

    struct TestComponent;

    impl Component for TestComponent {
        fn render(&self) -> VNode {
            VNode::Text(VText {
                content: "Hello, World!".to_string(),
            })
        }
    }

    #[test]
    fn test_component_render() {
        let component = TestComponent;
        let vnode = component.render();
        assert!(matches!(vnode, VNode::Text(_)));
    }
}
