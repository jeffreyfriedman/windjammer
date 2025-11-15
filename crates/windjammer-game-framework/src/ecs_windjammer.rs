///! Windjammer-friendly ECS API
///!
///! This module provides an ergonomic, idiomatic Windjammer API for the ECS system
///! while hiding Rust-specific implementation details.
///!
///! This is a thin wrapper around the core ECS (`crate::ecs`) that provides
///! a more Windjammer-friendly interface.

// Re-export core ECS types
pub use crate::ecs::{Entity, World, System, Component};

// For backwards compatibility, we also export under the old names
pub use crate::ecs::Entity as EntityWj;
pub use crate::ecs::World as WorldWj;
pub use crate::ecs::System as SystemWj;

// Note: The core ECS already provides a clean, ergonomic API.
// This module exists primarily for backwards compatibility and to provide
// a clear separation between the internal Rust API and the Windjammer-facing API.
