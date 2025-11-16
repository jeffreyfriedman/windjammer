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

pub mod app;
#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
pub mod app_docking;
#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
pub mod app_docking_v2;
#[cfg(target_arch = "wasm32")]
pub mod app_reactive;
#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
pub mod app_reactive_eframe;
#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
pub mod asset_browser;
#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
pub mod build_system;
#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
pub mod scene_gizmos;
pub mod undo_redo; // Available on all platforms

// Old app_reactive with manual winit+wgpu (deprecated, keeping for reference)
#[cfg(all(not(target_arch = "wasm32"), not(feature = "desktop")))]
mod app_reactive_old;
pub mod component;
pub mod component_runtime;
pub mod components; // Component library
pub mod events;
pub mod platform;
pub mod reactivity;

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
pub mod desktop_renderer;
#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
pub mod file_watcher;
pub mod reactivity_optimized;
pub mod renderer;
pub mod routing;
pub mod runtime;
#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
pub mod scene_manager;
#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
pub mod scene_renderer_3d;
pub mod simple_renderer;
pub mod simple_vnode;
pub mod ssr;
#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
pub mod syntax_highlighting;
pub mod to_vnode;
pub mod vdom;
#[cfg(target_arch = "wasm32")]
pub mod wasm_events;

#[cfg(target_arch = "wasm32")]
pub mod examples_wasm;

#[cfg(test)]
mod reactivity_tests;

/// Prelude module with commonly used types and traits
pub mod prelude {
    pub use crate::app::App;
    pub use crate::component::{Component, ComponentProps};
    pub use crate::component_runtime;
    pub use crate::events::{Event, EventHandler};
    pub use crate::platform::{Platform, PlatformType};
    pub use crate::reactivity::{Computed, Effect, Signal};
    pub use crate::renderer::{Renderer, WebRenderer};
    pub use crate::routing::{Route, Router};
    pub use crate::simple_vnode::{VAttr, VNode};
    pub use crate::to_vnode::ToVNode;

    pub use crate::vdom::{VElement, VText};

    // Reactive app (all platforms)
    #[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
    pub use crate::app_docking::DockingApp;
    #[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
    pub use crate::app_docking_v2::EditorApp;
    #[cfg(target_arch = "wasm32")]
    pub use crate::app_reactive::{trigger_rerender, ReactiveApp};
    #[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
    pub use crate::app_reactive_eframe::{trigger_rerender, ReactiveApp};

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
