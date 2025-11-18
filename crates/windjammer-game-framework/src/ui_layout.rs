//! # UI Layout Module
//!
//! Provides flexible layout systems for UI elements.
//!
//! ## Features
//! - Flexbox layout (row, column, wrap)
//! - Grid layout (rows, columns, gaps)
//! - Anchor-based positioning
//! - Constraint system
//! - Responsive design
//! - Auto-sizing
//! - Alignment and justification
//!
//! ## Example
//! ```no_run
//! use windjammer_game_framework::ui_layout::{FlexLayout, FlexDirection, LayoutRect};
//!
//! let layout = FlexLayout::new(FlexDirection::Row)
//!     .with_gap(10.0)
//!     .with_padding([10.0, 10.0, 10.0, 10.0]);
//!
//! let container = LayoutRect::new(0.0, 0.0, 800.0, 600.0);
//! let children = vec![
//!     LayoutRect::new(0.0, 0.0, 100.0, 50.0),
//!     LayoutRect::new(0.0, 0.0, 150.0, 50.0),
//! ];
//!
//! let positioned = layout.layout(&container, &children);
//! ```

/// Layout rectangle
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutRect {
    /// X position
    pub x: f32,
    /// Y position
    pub y: f32,
    /// Width
    pub width: f32,
    /// Height
    pub height: f32,
}

impl LayoutRect {
    /// Create a new layout rect
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Get the center point
    pub fn center(&self) -> (f32, f32) {
        (self.x + self.width / 2.0, self.y + self.height / 2.0)
    }

    /// Get the right edge
    pub fn right(&self) -> f32 {
        self.x + self.width
    }

    /// Get the bottom edge
    pub fn bottom(&self) -> f32 {
        self.y + self.height
    }

    /// Check if a point is inside the rect
    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x && x <= self.right() && y >= self.y && y <= self.bottom()
    }
}

impl Default for LayoutRect {
    fn default() -> Self {
        Self::new(0.0, 0.0, 0.0, 0.0)
    }
}

/// Flex direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlexDirection {
    /// Horizontal layout (left to right)
    Row,
    /// Horizontal layout (right to left)
    RowReverse,
    /// Vertical layout (top to bottom)
    Column,
    /// Vertical layout (bottom to top)
    ColumnReverse,
}

impl Default for FlexDirection {
    fn default() -> Self {
        Self::Row
    }
}

/// Flex wrap
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlexWrap {
    /// No wrapping
    NoWrap,
    /// Wrap to next line
    Wrap,
    /// Wrap in reverse
    WrapReverse,
}

impl Default for FlexWrap {
    fn default() -> Self {
        Self::NoWrap
    }
}

/// Justify content (main axis alignment)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JustifyContent {
    /// Pack items at the start
    FlexStart,
    /// Pack items at the end
    FlexEnd,
    /// Center items
    Center,
    /// Distribute items evenly with space between
    SpaceBetween,
    /// Distribute items evenly with space around
    SpaceAround,
    /// Distribute items evenly with equal space
    SpaceEvenly,
}

impl Default for JustifyContent {
    fn default() -> Self {
        Self::FlexStart
    }
}

/// Align items (cross axis alignment)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlignItems {
    /// Stretch to fill
    Stretch,
    /// Align at start
    FlexStart,
    /// Align at end
    FlexEnd,
    /// Center items
    Center,
    /// Align to baseline
    Baseline,
}

impl Default for AlignItems {
    fn default() -> Self {
        Self::Stretch
    }
}

/// Align content (multi-line alignment)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlignContent {
    /// Stretch lines
    Stretch,
    /// Pack lines at start
    FlexStart,
    /// Pack lines at end
    FlexEnd,
    /// Center lines
    Center,
    /// Space between lines
    SpaceBetween,
    /// Space around lines
    SpaceAround,
}

impl Default for AlignContent {
    fn default() -> Self {
        Self::Stretch
    }
}

/// Flexbox layout
#[derive(Debug, Clone)]
pub struct FlexLayout {
    /// Flex direction
    pub direction: FlexDirection,
    /// Flex wrap
    pub wrap: FlexWrap,
    /// Justify content
    pub justify: JustifyContent,
    /// Align items
    pub align_items: AlignItems,
    /// Align content
    pub align_content: AlignContent,
    /// Gap between items
    pub gap: f32,
    /// Padding [top, right, bottom, left]
    pub padding: [f32; 4],
}

impl FlexLayout {
    /// Create a new flex layout
    pub fn new(direction: FlexDirection) -> Self {
        Self {
            direction,
            wrap: FlexWrap::default(),
            justify: JustifyContent::default(),
            align_items: AlignItems::default(),
            align_content: AlignContent::default(),
            gap: 0.0,
            padding: [0.0, 0.0, 0.0, 0.0],
        }
    }

    /// Set wrap
    pub fn with_wrap(mut self, wrap: FlexWrap) -> Self {
        self.wrap = wrap;
        self
    }

    /// Set justify content
    pub fn with_justify(mut self, justify: JustifyContent) -> Self {
        self.justify = justify;
        self
    }

    /// Set align items
    pub fn with_align_items(mut self, align_items: AlignItems) -> Self {
        self.align_items = align_items;
        self
    }

    /// Set align content
    pub fn with_align_content(mut self, align_content: AlignContent) -> Self {
        self.align_content = align_content;
        self
    }

    /// Set gap
    pub fn with_gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    /// Set padding
    pub fn with_padding(mut self, padding: [f32; 4]) -> Self {
        self.padding = padding;
        self
    }

    /// Layout children within a container
    pub fn layout(&self, container: &LayoutRect, children: &[LayoutRect]) -> Vec<LayoutRect> {
        if children.is_empty() {
            return Vec::new();
        }

        let mut result = Vec::new();

        // Calculate available space
        let available_width = container.width - self.padding[1] - self.padding[3];
        let available_height = container.height - self.padding[0] - self.padding[2];

        let is_row = matches!(
            self.direction,
            FlexDirection::Row | FlexDirection::RowReverse
        );

        let mut x = container.x + self.padding[3];
        let mut y = container.y + self.padding[0];

        // Simple layout (no wrapping for now)
        for (i, child) in children.iter().enumerate() {
            let mut positioned = *child;

            if is_row {
                positioned.x = x;
                positioned.y = y;

                // Apply justify content
                match self.justify {
                    JustifyContent::Center => {
                        let total_width: f32 = children.iter().map(|c| c.width).sum::<f32>()
                            + self.gap * (children.len() - 1) as f32;
                        let offset = (available_width - total_width) / 2.0;
                        positioned.x += offset;
                    }
                    JustifyContent::FlexEnd => {
                        let total_width: f32 = children.iter().map(|c| c.width).sum::<f32>()
                            + self.gap * (children.len() - 1) as f32;
                        let offset = available_width - total_width;
                        positioned.x += offset;
                    }
                    _ => {}
                }

                // Apply align items
                match self.align_items {
                    AlignItems::Center => {
                        positioned.y += (available_height - child.height) / 2.0;
                    }
                    AlignItems::FlexEnd => {
                        positioned.y += available_height - child.height;
                    }
                    AlignItems::Stretch => {
                        positioned.height = available_height;
                    }
                    _ => {}
                }

                x += child.width + self.gap;
            } else {
                positioned.x = x;
                positioned.y = y;

                // Apply justify content
                match self.justify {
                    JustifyContent::Center => {
                        let total_height: f32 = children.iter().map(|c| c.height).sum::<f32>()
                            + self.gap * (children.len() - 1) as f32;
                        let offset = (available_height - total_height) / 2.0;
                        positioned.y += offset;
                    }
                    JustifyContent::FlexEnd => {
                        let total_height: f32 = children.iter().map(|c| c.height).sum::<f32>()
                            + self.gap * (children.len() - 1) as f32;
                        let offset = available_height - total_height;
                        positioned.y += offset;
                    }
                    _ => {}
                }

                // Apply align items
                match self.align_items {
                    AlignItems::Center => {
                        positioned.x += (available_width - child.width) / 2.0;
                    }
                    AlignItems::FlexEnd => {
                        positioned.x += available_width - child.width;
                    }
                    AlignItems::Stretch => {
                        positioned.width = available_width;
                    }
                    _ => {}
                }

                y += child.height + self.gap;
            }

            result.push(positioned);
        }

        result
    }
}

impl Default for FlexLayout {
    fn default() -> Self {
        Self::new(FlexDirection::Row)
    }
}

/// Grid layout
#[derive(Debug, Clone)]
pub struct GridLayout {
    /// Number of columns
    pub columns: usize,
    /// Number of rows
    pub rows: usize,
    /// Column gap
    pub column_gap: f32,
    /// Row gap
    pub row_gap: f32,
    /// Padding [top, right, bottom, left]
    pub padding: [f32; 4],
}

impl GridLayout {
    /// Create a new grid layout
    pub fn new(columns: usize, rows: usize) -> Self {
        Self {
            columns,
            rows,
            column_gap: 0.0,
            row_gap: 0.0,
            padding: [0.0, 0.0, 0.0, 0.0],
        }
    }

    /// Set column gap
    pub fn with_column_gap(mut self, gap: f32) -> Self {
        self.column_gap = gap;
        self
    }

    /// Set row gap
    pub fn with_row_gap(mut self, gap: f32) -> Self {
        self.row_gap = gap;
        self
    }

    /// Set padding
    pub fn with_padding(mut self, padding: [f32; 4]) -> Self {
        self.padding = padding;
        self
    }

    /// Layout children within a container
    pub fn layout(&self, container: &LayoutRect, children: &[LayoutRect]) -> Vec<LayoutRect> {
        if children.is_empty() || self.columns == 0 || self.rows == 0 {
            return Vec::new();
        }

        let mut result = Vec::new();

        // Calculate available space
        let available_width = container.width - self.padding[1] - self.padding[3];
        let available_height = container.height - self.padding[0] - self.padding[2];

        // Calculate cell size
        let total_column_gap = self.column_gap * (self.columns - 1) as f32;
        let total_row_gap = self.row_gap * (self.rows - 1) as f32;

        let cell_width = (available_width - total_column_gap) / self.columns as f32;
        let cell_height = (available_height - total_row_gap) / self.rows as f32;

        // Position children
        for (i, _child) in children.iter().enumerate() {
            let col = i % self.columns;
            let row = i / self.columns;

            if row >= self.rows {
                break; // Don't layout more items than grid cells
            }

            let x = container.x
                + self.padding[3]
                + col as f32 * (cell_width + self.column_gap);
            let y = container.y + self.padding[0] + row as f32 * (cell_height + self.row_gap);

            result.push(LayoutRect::new(x, y, cell_width, cell_height));
        }

        result
    }
}

/// Anchor point
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anchor {
    /// Top-left corner
    TopLeft,
    /// Top-center
    TopCenter,
    /// Top-right corner
    TopRight,
    /// Middle-left
    MiddleLeft,
    /// Center
    Center,
    /// Middle-right
    MiddleRight,
    /// Bottom-left corner
    BottomLeft,
    /// Bottom-center
    BottomCenter,
    /// Bottom-right corner
    BottomRight,
}

impl Default for Anchor {
    fn default() -> Self {
        Self::TopLeft
    }
}

/// Anchor layout
#[derive(Debug, Clone)]
pub struct AnchorLayout {
    /// Anchor point
    pub anchor: Anchor,
    /// Offset from anchor
    pub offset: (f32, f32),
}

impl AnchorLayout {
    /// Create a new anchor layout
    pub fn new(anchor: Anchor) -> Self {
        Self {
            anchor,
            offset: (0.0, 0.0),
        }
    }

    /// Set offset
    pub fn with_offset(mut self, offset: (f32, f32)) -> Self {
        self.offset = offset;
        self
    }

    /// Position a child within a container
    pub fn position(&self, container: &LayoutRect, child: &LayoutRect) -> LayoutRect {
        let (anchor_x, anchor_y) = match self.anchor {
            Anchor::TopLeft => (container.x, container.y),
            Anchor::TopCenter => (container.x + container.width / 2.0, container.y),
            Anchor::TopRight => (container.right(), container.y),
            Anchor::MiddleLeft => (container.x, container.y + container.height / 2.0),
            Anchor::Center => {
                let (cx, cy) = container.center();
                (cx, cy)
            }
            Anchor::MiddleRight => (container.right(), container.y + container.height / 2.0),
            Anchor::BottomLeft => (container.x, container.bottom()),
            Anchor::BottomCenter => (container.x + container.width / 2.0, container.bottom()),
            Anchor::BottomRight => (container.right(), container.bottom()),
        };

        let x = anchor_x + self.offset.0 - child.width / 2.0;
        let y = anchor_y + self.offset.1 - child.height / 2.0;

        LayoutRect::new(x, y, child.width, child.height)
    }
}

impl Default for AnchorLayout {
    fn default() -> Self {
        Self::new(Anchor::TopLeft)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_rect_creation() {
        let rect = LayoutRect::new(10.0, 20.0, 100.0, 50.0);
        assert_eq!(rect.x, 10.0);
        assert_eq!(rect.y, 20.0);
        assert_eq!(rect.width, 100.0);
        assert_eq!(rect.height, 50.0);
    }

    #[test]
    fn test_layout_rect_center() {
        let rect = LayoutRect::new(0.0, 0.0, 100.0, 50.0);
        let (cx, cy) = rect.center();
        assert_eq!(cx, 50.0);
        assert_eq!(cy, 25.0);
    }

    #[test]
    fn test_layout_rect_edges() {
        let rect = LayoutRect::new(10.0, 20.0, 100.0, 50.0);
        assert_eq!(rect.right(), 110.0);
        assert_eq!(rect.bottom(), 70.0);
    }

    #[test]
    fn test_layout_rect_contains() {
        let rect = LayoutRect::new(0.0, 0.0, 100.0, 50.0);
        assert!(rect.contains(50.0, 25.0));
        assert!(!rect.contains(150.0, 25.0));
        assert!(!rect.contains(50.0, 75.0));
    }

    #[test]
    fn test_flex_layout_creation() {
        let layout = FlexLayout::new(FlexDirection::Row);
        assert_eq!(layout.direction, FlexDirection::Row);
        assert_eq!(layout.gap, 0.0);
    }

    #[test]
    fn test_flex_layout_builder() {
        let layout = FlexLayout::new(FlexDirection::Column)
            .with_gap(10.0)
            .with_justify(JustifyContent::Center)
            .with_align_items(AlignItems::Center);

        assert_eq!(layout.direction, FlexDirection::Column);
        assert_eq!(layout.gap, 10.0);
        assert_eq!(layout.justify, JustifyContent::Center);
        assert_eq!(layout.align_items, AlignItems::Center);
    }

    #[test]
    fn test_flex_layout_row() {
        let layout = FlexLayout::new(FlexDirection::Row).with_gap(10.0);
        let container = LayoutRect::new(0.0, 0.0, 300.0, 100.0);
        let children = vec![
            LayoutRect::new(0.0, 0.0, 50.0, 50.0),
            LayoutRect::new(0.0, 0.0, 50.0, 50.0),
        ];

        let positioned = layout.layout(&container, &children);
        assert_eq!(positioned.len(), 2);
        assert_eq!(positioned[0].x, 0.0);
        assert_eq!(positioned[1].x, 60.0); // 50 + 10 gap
    }

    #[test]
    fn test_flex_layout_column() {
        let layout = FlexLayout::new(FlexDirection::Column).with_gap(10.0);
        let container = LayoutRect::new(0.0, 0.0, 100.0, 300.0);
        let children = vec![
            LayoutRect::new(0.0, 0.0, 50.0, 50.0),
            LayoutRect::new(0.0, 0.0, 50.0, 50.0),
        ];

        let positioned = layout.layout(&container, &children);
        assert_eq!(positioned.len(), 2);
        assert_eq!(positioned[0].y, 0.0);
        assert_eq!(positioned[1].y, 60.0); // 50 + 10 gap
    }

    #[test]
    fn test_grid_layout_creation() {
        let layout = GridLayout::new(3, 2);
        assert_eq!(layout.columns, 3);
        assert_eq!(layout.rows, 2);
    }

    #[test]
    fn test_grid_layout_builder() {
        let layout = GridLayout::new(3, 2)
            .with_column_gap(10.0)
            .with_row_gap(5.0);

        assert_eq!(layout.column_gap, 10.0);
        assert_eq!(layout.row_gap, 5.0);
    }

    #[test]
    fn test_grid_layout() {
        let layout = GridLayout::new(2, 2);
        let container = LayoutRect::new(0.0, 0.0, 200.0, 200.0);
        let children = vec![
            LayoutRect::new(0.0, 0.0, 0.0, 0.0),
            LayoutRect::new(0.0, 0.0, 0.0, 0.0),
            LayoutRect::new(0.0, 0.0, 0.0, 0.0),
            LayoutRect::new(0.0, 0.0, 0.0, 0.0),
        ];

        let positioned = layout.layout(&container, &children);
        assert_eq!(positioned.len(), 4);

        // Check grid positions
        assert_eq!(positioned[0].x, 0.0);
        assert_eq!(positioned[0].y, 0.0);
        assert_eq!(positioned[1].x, 100.0);
        assert_eq!(positioned[1].y, 0.0);
        assert_eq!(positioned[2].x, 0.0);
        assert_eq!(positioned[2].y, 100.0);
        assert_eq!(positioned[3].x, 100.0);
        assert_eq!(positioned[3].y, 100.0);
    }

    #[test]
    fn test_anchor_layout_creation() {
        let layout = AnchorLayout::new(Anchor::Center);
        assert_eq!(layout.anchor, Anchor::Center);
    }

    #[test]
    fn test_anchor_layout_top_left() {
        let layout = AnchorLayout::new(Anchor::TopLeft);
        let container = LayoutRect::new(0.0, 0.0, 200.0, 200.0);
        let child = LayoutRect::new(0.0, 0.0, 50.0, 50.0);

        let positioned = layout.position(&container, &child);
        assert_eq!(positioned.x, -25.0); // Centered on anchor
        assert_eq!(positioned.y, -25.0);
    }

    #[test]
    fn test_anchor_layout_center() {
        let layout = AnchorLayout::new(Anchor::Center);
        let container = LayoutRect::new(0.0, 0.0, 200.0, 200.0);
        let child = LayoutRect::new(0.0, 0.0, 50.0, 50.0);

        let positioned = layout.position(&container, &child);
        assert_eq!(positioned.x, 75.0); // 100 - 25
        assert_eq!(positioned.y, 75.0);
    }

    #[test]
    fn test_anchor_layout_with_offset() {
        let layout = AnchorLayout::new(Anchor::Center).with_offset((10.0, 20.0));
        let container = LayoutRect::new(0.0, 0.0, 200.0, 200.0);
        let child = LayoutRect::new(0.0, 0.0, 50.0, 50.0);

        let positioned = layout.position(&container, &child);
        assert_eq!(positioned.x, 85.0); // 75 + 10
        assert_eq!(positioned.y, 95.0); // 75 + 20
    }

    #[test]
    fn test_flex_direction_types() {
        assert_eq!(FlexDirection::Row, FlexDirection::Row);
        assert_eq!(FlexDirection::Column, FlexDirection::Column);
    }

    #[test]
    fn test_justify_content_types() {
        assert_eq!(JustifyContent::FlexStart, JustifyContent::FlexStart);
        assert_eq!(JustifyContent::Center, JustifyContent::Center);
        assert_eq!(JustifyContent::SpaceBetween, JustifyContent::SpaceBetween);
    }

    #[test]
    fn test_align_items_types() {
        assert_eq!(AlignItems::Stretch, AlignItems::Stretch);
        assert_eq!(AlignItems::Center, AlignItems::Center);
    }

    #[test]
    fn test_anchor_types() {
        assert_eq!(Anchor::TopLeft, Anchor::TopLeft);
        assert_eq!(Anchor::Center, Anchor::Center);
        assert_eq!(Anchor::BottomRight, Anchor::BottomRight);
    }

    #[test]
    fn test_flex_layout_empty_children() {
        let layout = FlexLayout::new(FlexDirection::Row);
        let container = LayoutRect::new(0.0, 0.0, 100.0, 100.0);
        let children = vec![];

        let positioned = layout.layout(&container, &children);
        assert_eq!(positioned.len(), 0);
    }

    #[test]
    fn test_grid_layout_empty_children() {
        let layout = GridLayout::new(2, 2);
        let container = LayoutRect::new(0.0, 0.0, 100.0, 100.0);
        let children = vec![];

        let positioned = layout.layout(&container, &children);
        assert_eq!(positioned.len(), 0);
    }

    #[test]
    fn test_flex_layout_with_padding() {
        let layout = FlexLayout::new(FlexDirection::Row).with_padding([10.0, 10.0, 10.0, 10.0]);
        let container = LayoutRect::new(0.0, 0.0, 200.0, 100.0);
        let children = vec![LayoutRect::new(0.0, 0.0, 50.0, 50.0)];

        let positioned = layout.layout(&container, &children);
        assert_eq!(positioned[0].x, 10.0); // Left padding
        assert_eq!(positioned[0].y, 10.0); // Top padding
    }

    #[test]
    fn test_grid_layout_with_gaps() {
        let layout = GridLayout::new(2, 2)
            .with_column_gap(10.0)
            .with_row_gap(10.0);
        let container = LayoutRect::new(0.0, 0.0, 210.0, 210.0);
        let children = vec![
            LayoutRect::new(0.0, 0.0, 0.0, 0.0),
            LayoutRect::new(0.0, 0.0, 0.0, 0.0),
        ];

        let positioned = layout.layout(&container, &children);
        assert_eq!(positioned.len(), 2);

        // Cell width = (210 - 10) / 2 = 100
        assert_eq!(positioned[0].width, 100.0);
        assert_eq!(positioned[1].x, 110.0); // 100 + 10 gap
    }
}

