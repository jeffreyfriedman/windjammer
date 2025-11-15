/// World-class Entity Component System
/// 
/// Inspired by Unity DOTS, Bevy, and EnTT
/// Designed for:
/// - Cache-friendly performance
/// - Parallel execution
/// - Zero-cost abstractions
/// - Pure Windjammer API

pub mod entity;
pub mod component;
pub mod world;
pub mod query;
pub mod system;
pub mod storage;
pub mod archetype;

pub use entity::*;
pub use component::*;
pub use world::*;
pub use query::*;
pub use system::*;
pub use storage::*;
pub use archetype::*;

/// Prelude for convenient imports
pub mod prelude {
    pub use super::entity::*;
    pub use super::component::*;
    pub use super::world::*;
    pub use super::query::*;
    pub use super::system::*;
}

