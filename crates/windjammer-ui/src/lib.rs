//! # Windjammer UI Framework
//!
//! A cross-platform UI framework for building applications that run on:
//! - **Web**: JavaScript (Virtual DOM) or WebAssembly (web-sys)
//! - **Desktop**: Native apps with Tauri (macOS, Windows, Linux)
//! - **Mobile**: Native apps (iOS, Android)
//!
//! ## Philosophy
//!
//! - **Svelte-inspired**: Simple, compiler-driven, minimal runtime
//! - **Type-safe**: Full Rust type checking
//! - **Cross-platform**: Write once, run everywhere
//! - **Native performance**: Use platform's native capabilities
//!
//! ## Example
//!
//! ```ignore
//! use windjammer_ui::prelude::*;
//!
//! #[component]
//! struct Counter {
//!     state count: i32 = 0,
//!     
//!     fn render() -> Html {
//!         html! {
//!             <div>
//!                 <h1>"Count: {count}"</h1>
//!                 <button onclick={|| count += 1}>"Increment"</button>
//!             </div>
//!         }
//!     }
//! }
//! ```

pub mod component;
pub mod events;
pub mod platform;
pub mod reactivity;
pub mod renderer;
pub mod vdom;

/// Prelude module with commonly used types and traits
pub mod prelude {
    pub use crate::component::{Component, ComponentProps};
    pub use crate::events::{Event, EventHandler};
    pub use crate::platform::{Platform, PlatformType};
    pub use crate::reactivity::{Computed, Effect, Signal};
    pub use crate::vdom::{VElement, VNode, VText};
}

/// Get the current platform type
pub fn current_platform() -> platform::PlatformType {
    platform::detect_platform()
}

/// Mount a component to the target
///
/// # Web
/// ```ignore
/// windjammer_ui::mount("#app", App::new());
/// ```
///
/// # Desktop
/// ```ignore
/// windjammer_ui::mount_desktop("My App", App::new());
/// ```
pub fn mount<C: component::Component>(selector: &str, component: C) -> Result<(), String> {
    renderer::mount(selector, component)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_detection() {
        let platform = current_platform();
        // Platform detection should always succeed
        assert!(matches!(
            platform,
            platform::PlatformType::Web
                | platform::PlatformType::Desktop
                | platform::PlatformType::Mobile
        ));
    }
}
