// Windjammer Game Editor Library
// Shared code for desktop and browser editors

pub mod panels;
pub mod editor_integration;

// Re-export commonly used types
pub use panels::*;

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
pub use editor_integration::GameEditorPanels;

