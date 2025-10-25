//! Windjammer UI Framework
//!
//! A cross-platform UI framework for building web, desktop, and mobile applications.
//!
//! # Features
//!
//! - **Reactive State Management** - Signal, Computed, Effect
//! - **Virtual DOM** - Efficient diffing and patching
//! - **Component Model** - Clean, composable components
//! - **Cross-Platform** - Web (WASM), Desktop (Tauri), Mobile
//! - **Server-Side Rendering** - SSR with hydration
//! - **Routing** - Client-side navigation
//!
//! # Example
//!
//! ```rust,no_run
//! use windjammer_ui::prelude::*;
//! use windjammer_ui::vdom::{VElement, VNode, VText};
//! use windjammer_ui::reactivity::Signal;
//!
//! struct Counter {
//!     count: Signal<i32>,
//! }
//!
//! impl Counter {
//!     fn new() -> Self {
//!         Self { count: Signal::new(0) }
//!     }
//!     
//!     fn increment(&self) {
//!         self.count.update(|c| *c += 1);
//!     }
//! }
//! ```

#![allow(clippy::module_inception)]

// Re-export the proc macro
pub use windjammer_ui_macro::component;
pub use windjammer_ui_macro::Props;

pub mod component;
pub mod component_runtime;
pub mod events;
pub mod platform;
pub mod reactivity;
pub mod renderer;
pub mod routing;
pub mod runtime;
pub mod simple_renderer;
pub mod simple_vnode;
pub mod ssr;
pub mod vdom;

#[cfg(target_arch = "wasm32")]
pub mod examples_wasm;

#[cfg(test)]
mod reactivity_tests;

/// Prelude module with commonly used types and traits
pub mod prelude {
    pub use crate::component::{Component, ComponentProps};
    pub use crate::component_runtime;
    pub use crate::events::{Event, EventHandler};
    pub use crate::platform::{Platform, PlatformType};
    pub use crate::reactivity::{Computed, Effect, Signal};
    pub use crate::renderer::{Renderer, WebRenderer};
    pub use crate::routing::{Route, Router};
    pub use crate::simple_vnode::{VAttr, VNode};
    pub use crate::vdom::{VElement, VText};

    // Re-export the component macro
    pub use crate::component;
}

/// Mount a component to the DOM (WASM only)
#[cfg(target_arch = "wasm32")]
pub use renderer::mount;

#[cfg(test)]
mod tests {
    #[test]
    fn test_prelude_imports() {
        // Just verify prelude compiles
        use crate::prelude::*;
        let _ = Signal::new(42);
    }
}
