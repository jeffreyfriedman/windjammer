//! Windjammer UI Component Library
//!
//! Production-ready components for building web, desktop, and mobile applications.

pub mod advanced_code_editor;
pub mod alert;
pub mod badge;
pub mod button;
pub mod card;
pub mod checkbox;
pub mod code_editor;
pub mod container;
pub mod dialog;
pub mod file_tree;
pub mod flex;
pub mod grid;
pub mod input;
pub mod panel;
pub mod progress;
pub mod radio;
pub mod select;
pub mod slider;
pub mod spinner;
pub mod split_panel;
pub mod switch;
pub mod tab_panel;
pub mod tabs;
pub mod text;
pub mod toolbar;
pub mod tooltip;
pub mod tree_view;

// Re-export all components and their types
pub use advanced_code_editor::AdvancedCodeEditor;
pub use alert::{Alert, AlertVariant};
pub use badge::{Badge, BadgeSize, BadgeVariant};
pub use button::{Button, ButtonSize, ButtonVariant};
pub use card::Card;
pub use checkbox::{Checkbox, CheckboxSize};
pub use code_editor::CodeEditor;
pub use container::Container;
pub use dialog::Dialog;
pub use file_tree::{FileNode, FileTree};
pub use flex::{Flex, FlexDirection};
pub use grid::Grid;
pub use input::Input;
pub use panel::Panel;
pub use progress::{Progress, ProgressVariant};
pub use radio::{RadioGroup, RadioOption};
pub use select::{Select, SelectOption};
pub use slider::Slider;
pub use spinner::{Spinner, SpinnerSize};
pub use split_panel::{SplitDirection, SplitPanel};
pub use switch::{Switch, SwitchSize};
pub use tab_panel::{Tab as TabPanelTab, TabPanel};
pub use tabs::{Tab, Tabs};
pub use text::{Text, TextSize};
pub use toolbar::Toolbar;
pub use tooltip::{Tooltip, TooltipPosition};
pub use tree_view::{TreeItem, TreeView};
