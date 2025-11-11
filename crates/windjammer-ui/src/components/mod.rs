//! Windjammer UI Component Library
//!
//! Production-ready components for building web, desktop, and mobile applications.

pub mod alert;
pub mod button;
pub mod card;
pub mod code_editor;
pub mod container;
pub mod file_tree;
pub mod flex;
pub mod grid;
pub mod input;
pub mod panel;
pub mod tabs;
pub mod text;
pub mod toolbar;

// Re-export all components and their types
pub use alert::{Alert, AlertVariant};
pub use button::{Button, ButtonSize, ButtonVariant};
pub use card::Card;
pub use code_editor::CodeEditor;
pub use container::Container;
pub use file_tree::{FileNode, FileTree};
pub use flex::{Flex, FlexDirection};
pub use grid::Grid;
pub use input::Input;
pub use panel::Panel;
pub use tabs::{Tab, Tabs};
pub use text::{Text, TextSize};
pub use toolbar::Toolbar;
