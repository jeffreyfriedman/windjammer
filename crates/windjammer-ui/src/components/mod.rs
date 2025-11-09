//! Windjammer UI Component Library
//!
//! Production-ready components for building web, desktop, and mobile applications.

pub mod button;
pub mod container;
pub mod input;
pub mod text;
pub mod flex;
pub mod grid;
pub mod card;
pub mod alert;
pub mod code_editor;
pub mod file_tree;
pub mod panel;
pub mod toolbar;
pub mod tabs;

// Re-export all components
pub use button::Button;
pub use container::Container;
pub use input::Input;
pub use text::Text;
pub use flex::Flex;
pub use grid::Grid;
pub use card::Card;
pub use alert::Alert;
pub use code_editor::CodeEditor;
pub use file_tree::FileTree;
pub use panel::Panel;
pub use toolbar::Toolbar;
pub use tabs::{Tab, Tabs};

