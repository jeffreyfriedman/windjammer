//! # Windjammer Rust SDK
//!
//! Zero-cost native bindings for the Windjammer Game Engine.
//!
//! This SDK provides idiomatic Rust access to all Windjammer features with
//! zero overhead compared to using the framework directly.
//!
//! ## Quick Start
//!
//! ```no_run
//! use windjammer_sdk::prelude::*;
//!
//! fn main() {
//!     let mut app = App::new();
//!     app.add_system(hello_system);
//!     app.run();
//! }
//!
//! fn hello_system() {
//!     println!("Hello, Windjammer!");
//! }
//! ```

// Re-export the entire framework
pub use windjammer_game_framework::*;

// Re-export commonly used types
pub use glam::{Vec2, Vec3, Vec4, Mat4, Quat};
pub use serde::{Serialize, Deserialize};

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::*;
    
    // Core types
    pub use windjammer_game_framework::prelude::*;
    
    // Math types
    pub use glam::{Vec2, Vec3, Vec4, Mat4, Quat};
    
    // Common traits
    pub use serde::{Serialize, Deserialize};
}

/// SDK version information
pub mod version {
    /// SDK version
    pub const VERSION: &str = env!("CARGO_PKG_VERSION");
    
    /// SDK name
    pub const NAME: &str = env!("CARGO_PKG_NAME");
    
    /// Framework version
    pub const FRAMEWORK_VERSION: &str = "0.34.0";
    
    /// Get version string
    pub fn version_string() -> String {
        format!("{} v{} (framework v{})", NAME, VERSION, FRAMEWORK_VERSION)
    }
}

/// Utility functions for common tasks
pub mod utils {
    use super::*;
    
    /// Create a new 2D game application
    pub fn new_2d_app() -> App {
        let mut app = App::new();
        // Add default 2D plugins
        app
    }
    
    /// Create a new 3D game application
    #[cfg(feature = "3d")]
    pub fn new_3d_app() -> App {
        let mut app = App::new();
        // Add default 3D plugins
        app
    }
    
    /// Load an asset asynchronously
    pub fn load_asset<T>(path: &str) -> T
    where
        T: Default,
    {
        // TODO: Implement actual asset loading
        T::default()
    }
}

/// Helper macros for common patterns
#[macro_export]
macro_rules! system {
    ($name:ident) => {
        fn $name() {
            // System implementation
        }
    };
}

/// Component bundle macro for convenience
#[macro_export]
macro_rules! bundle {
    ($name:ident { $($field:ident: $ty:ty),* $(,)? }) => {
        #[derive(Debug, Clone)]
        pub struct $name {
            $(pub $field: $ty,)*
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        let version = version::version_string();
        assert!(version.contains("windjammer-sdk"));
    }

    #[test]
    fn test_prelude_imports() {
        use prelude::*;
        
        let _v2 = Vec2::new(1.0, 2.0);
        let _v3 = Vec3::new(1.0, 2.0, 3.0);
        let _v4 = Vec4::new(1.0, 2.0, 3.0, 4.0);
    }

    #[test]
    fn test_2d_app_creation() {
        let _app = utils::new_2d_app();
    }

    #[test]
    #[cfg(feature = "3d")]
    fn test_3d_app_creation() {
        let _app = utils::new_3d_app();
    }
}

