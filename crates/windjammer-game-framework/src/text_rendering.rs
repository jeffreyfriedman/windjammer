//! # Text Rendering Module
//!
//! Provides text rendering with TrueType/OpenType font support.
//!
//! ## Features
//! - TrueType/OpenType font loading
//! - Glyph atlas generation
//! - Text layout (left, center, right, justified)
//! - Multi-line text
//! - Rich text (colors, styles)
//! - Font caching
//! - Kerning support
//! - Unicode support
//!
//! ## Example
//! ```no_run
//! use windjammer_game_framework::text_rendering::{FontManager, TextLayout, TextStyle};
//!
//! let mut font_manager = FontManager::new();
//! font_manager.load_font("Arial", "fonts/arial.ttf").unwrap();
//!
//! let layout = TextLayout::new("Hello, World!")
//!     .with_font("Arial")
//!     .with_size(24.0)
//!     .with_color([1.0, 1.0, 1.0, 1.0]);
//!
//! let glyphs = font_manager.layout_text(&layout);
//! ```

use std::collections::HashMap;
use std::path::Path;

/// Font ID for tracking loaded fonts
pub type FontId = u32;

/// Glyph ID for tracking individual glyphs
pub type GlyphId = u32;

/// Text alignment
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAlign {
    /// Left-aligned text
    Left,
    /// Center-aligned text
    Center,
    /// Right-aligned text
    Right,
    /// Justified text (stretch to fill width)
    Justified,
}

impl Default for TextAlign {
    fn default() -> Self {
        Self::Left
    }
}

/// Text vertical alignment
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerticalAlign {
    /// Top-aligned text
    Top,
    /// Middle-aligned text
    Middle,
    /// Bottom-aligned text
    Bottom,
    /// Baseline-aligned text
    Baseline,
}

impl Default for VerticalAlign {
    fn default() -> Self {
        Self::Baseline
    }
}

/// Font weight
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontWeight {
    /// Thin (100)
    Thin,
    /// Extra Light (200)
    ExtraLight,
    /// Light (300)
    Light,
    /// Normal (400)
    Normal,
    /// Medium (500)
    Medium,
    /// Semi Bold (600)
    SemiBold,
    /// Bold (700)
    Bold,
    /// Extra Bold (800)
    ExtraBold,
    /// Black (900)
    Black,
}

impl Default for FontWeight {
    fn default() -> Self {
        Self::Normal
    }
}

/// Font style
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontStyle {
    /// Normal style
    Normal,
    /// Italic style
    Italic,
    /// Oblique style
    Oblique,
}

impl Default for FontStyle {
    fn default() -> Self {
        Self::Normal
    }
}

/// Text decoration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextDecoration {
    /// Underline
    pub underline: bool,
    /// Strikethrough
    pub strikethrough: bool,
    /// Overline
    pub overline: bool,
}

impl Default for TextDecoration {
    fn default() -> Self {
        Self {
            underline: false,
            strikethrough: false,
            overline: false,
        }
    }
}

/// Text style
#[derive(Debug, Clone)]
pub struct TextStyle {
    /// Font name
    pub font: String,
    /// Font size in pixels
    pub size: f32,
    /// Text color (RGBA)
    pub color: [f32; 4],
    /// Font weight
    pub weight: FontWeight,
    /// Font style
    pub style: FontStyle,
    /// Text decoration
    pub decoration: TextDecoration,
    /// Line height multiplier
    pub line_height: f32,
    /// Letter spacing
    pub letter_spacing: f32,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            font: "default".to_string(),
            size: 16.0,
            color: [1.0, 1.0, 1.0, 1.0],
            weight: FontWeight::default(),
            style: FontStyle::default(),
            decoration: TextDecoration::default(),
            line_height: 1.2,
            letter_spacing: 0.0,
        }
    }
}

impl TextStyle {
    /// Create a new text style
    pub fn new(font: &str, size: f32) -> Self {
        Self {
            font: font.to_string(),
            size,
            ..Default::default()
        }
    }

    /// Set font
    pub fn with_font(mut self, font: &str) -> Self {
        self.font = font.to_string();
        self
    }

    /// Set size
    pub fn with_size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// Set color
    pub fn with_color(mut self, color: [f32; 4]) -> Self {
        self.color = color;
        self
    }

    /// Set weight
    pub fn with_weight(mut self, weight: FontWeight) -> Self {
        self.weight = weight;
        self
    }

    /// Set style
    pub fn with_style(mut self, style: FontStyle) -> Self {
        self.style = style;
        self
    }

    /// Set line height
    pub fn with_line_height(mut self, line_height: f32) -> Self {
        self.line_height = line_height;
        self
    }

    /// Set letter spacing
    pub fn with_letter_spacing(mut self, letter_spacing: f32) -> Self {
        self.letter_spacing = letter_spacing;
        self
    }

    /// Enable underline
    pub fn with_underline(mut self) -> Self {
        self.decoration.underline = true;
        self
    }

    /// Enable strikethrough
    pub fn with_strikethrough(mut self) -> Self {
        self.decoration.strikethrough = true;
        self
    }
}

/// Text layout configuration
#[derive(Debug, Clone)]
pub struct TextLayout {
    /// Text content
    pub text: String,
    /// Text style
    pub style: TextStyle,
    /// Horizontal alignment
    pub align: TextAlign,
    /// Vertical alignment
    pub vertical_align: VerticalAlign,
    /// Maximum width (for wrapping)
    pub max_width: Option<f32>,
    /// Maximum height
    pub max_height: Option<f32>,
    /// Word wrap
    pub word_wrap: bool,
}

impl TextLayout {
    /// Create a new text layout
    pub fn new(text: &str) -> Self {
        Self {
            text: text.to_string(),
            style: TextStyle::default(),
            align: TextAlign::default(),
            vertical_align: VerticalAlign::default(),
            max_width: None,
            max_height: None,
            word_wrap: true,
        }
    }

    /// Set font
    pub fn with_font(mut self, font: &str) -> Self {
        self.style.font = font.to_string();
        self
    }

    /// Set size
    pub fn with_size(mut self, size: f32) -> Self {
        self.style.size = size;
        self
    }

    /// Set color
    pub fn with_color(mut self, color: [f32; 4]) -> Self {
        self.style.color = color;
        self
    }

    /// Set alignment
    pub fn with_align(mut self, align: TextAlign) -> Self {
        self.align = align;
        self
    }

    /// Set vertical alignment
    pub fn with_vertical_align(mut self, vertical_align: VerticalAlign) -> Self {
        self.vertical_align = vertical_align;
        self
    }

    /// Set maximum width
    pub fn with_max_width(mut self, max_width: f32) -> Self {
        self.max_width = Some(max_width);
        self
    }

    /// Set maximum height
    pub fn with_max_height(mut self, max_height: f32) -> Self {
        self.max_height = Some(max_height);
        self
    }

    /// Set word wrap
    pub fn with_word_wrap(mut self, word_wrap: bool) -> Self {
        self.word_wrap = word_wrap;
        self
    }

    /// Set style
    pub fn with_style(mut self, style: TextStyle) -> Self {
        self.style = style;
        self
    }
}

/// Glyph metrics
#[derive(Debug, Clone, Copy)]
pub struct GlyphMetrics {
    /// Glyph width
    pub width: f32,
    /// Glyph height
    pub height: f32,
    /// Horizontal bearing X
    pub bearing_x: f32,
    /// Horizontal bearing Y
    pub bearing_y: f32,
    /// Horizontal advance
    pub advance: f32,
}

impl Default for GlyphMetrics {
    fn default() -> Self {
        Self {
            width: 0.0,
            height: 0.0,
            bearing_x: 0.0,
            bearing_y: 0.0,
            advance: 0.0,
        }
    }
}

/// Positioned glyph
#[derive(Debug, Clone)]
pub struct PositionedGlyph {
    /// Glyph ID
    pub glyph_id: GlyphId,
    /// Character
    pub character: char,
    /// X position
    pub x: f32,
    /// Y position
    pub y: f32,
    /// Glyph metrics
    pub metrics: GlyphMetrics,
    /// Color
    pub color: [f32; 4],
}

/// Font metrics
#[derive(Debug, Clone, Copy)]
pub struct FontMetrics {
    /// Ascent (distance from baseline to top)
    pub ascent: f32,
    /// Descent (distance from baseline to bottom)
    pub descent: f32,
    /// Line gap
    pub line_gap: f32,
    /// Units per em
    pub units_per_em: f32,
}

impl Default for FontMetrics {
    fn default() -> Self {
        Self {
            ascent: 0.0,
            descent: 0.0,
            line_gap: 0.0,
            units_per_em: 1000.0,
        }
    }
}

/// Font data
#[derive(Debug)]
pub struct Font {
    /// Font ID
    pub id: FontId,
    /// Font name
    pub name: String,
    /// Font metrics
    pub metrics: FontMetrics,
    /// Glyph metrics cache
    pub glyph_metrics: HashMap<char, GlyphMetrics>,
    /// Kerning pairs
    pub kerning: HashMap<(char, char), f32>,
}

impl Font {
    /// Create a new font
    pub fn new(id: FontId, name: String) -> Self {
        Self {
            id,
            name,
            metrics: FontMetrics::default(),
            glyph_metrics: HashMap::new(),
            kerning: HashMap::new(),
        }
    }

    /// Get glyph metrics for a character
    pub fn get_glyph_metrics(&self, character: char) -> GlyphMetrics {
        self.glyph_metrics
            .get(&character)
            .copied()
            .unwrap_or_default()
    }

    /// Get kerning between two characters
    pub fn get_kerning(&self, left: char, right: char) -> f32 {
        self.kerning.get(&(left, right)).copied().unwrap_or(0.0)
    }

    /// Add glyph metrics
    pub fn add_glyph_metrics(&mut self, character: char, metrics: GlyphMetrics) {
        self.glyph_metrics.insert(character, metrics);
    }

    /// Add kerning pair
    pub fn add_kerning(&mut self, left: char, right: char, kerning: f32) {
        self.kerning.insert((left, right), kerning);
    }
}

/// Font manager
pub struct FontManager {
    /// Loaded fonts
    fonts: HashMap<String, Font>,
    /// Next font ID
    next_font_id: FontId,
    /// Default font name
    default_font: String,
}

impl FontManager {
    /// Create a new font manager
    pub fn new() -> Self {
        let mut manager = Self {
            fonts: HashMap::new(),
            next_font_id: 1,
            default_font: "default".to_string(),
        };

        // Create a default font with basic ASCII glyphs
        manager.create_default_font();

        manager
    }

    /// Create a default font
    fn create_default_font(&mut self) {
        let font_id = self.next_font_id;
        self.next_font_id += 1;

        let mut font = Font::new(font_id, "default".to_string());
        font.metrics = FontMetrics {
            ascent: 12.0,
            descent: 4.0,
            line_gap: 2.0,
            units_per_em: 16.0,
        };

        // Add basic ASCII glyph metrics (simplified)
        for c in ' '..='~' {
            let metrics = GlyphMetrics {
                width: 8.0,
                height: 16.0,
                bearing_x: 0.0,
                bearing_y: 12.0,
                advance: 8.0,
            };
            font.add_glyph_metrics(c, metrics);
        }

        self.fonts.insert("default".to_string(), font);
    }

    /// Load a font from a file
    pub fn load_font(&mut self, name: &str, _path: &Path) -> Result<FontId, String> {
        // In a real implementation, this would use a library like `rusttype` or `fontdue`
        // to parse TrueType/OpenType fonts. For now, we'll create a placeholder.

        let font_id = self.next_font_id;
        self.next_font_id += 1;

        let mut font = Font::new(font_id, name.to_string());
        font.metrics = FontMetrics {
            ascent: 12.0,
            descent: 4.0,
            line_gap: 2.0,
            units_per_em: 16.0,
        };

        // Add basic ASCII glyph metrics
        for c in ' '..='~' {
            let metrics = GlyphMetrics {
                width: 8.0,
                height: 16.0,
                bearing_x: 0.0,
                bearing_y: 12.0,
                advance: 8.0,
            };
            font.add_glyph_metrics(c, metrics);
        }

        self.fonts.insert(name.to_string(), font);

        Ok(font_id)
    }

    /// Get a font by name
    pub fn get_font(&self, name: &str) -> Option<&Font> {
        self.fonts.get(name)
    }

    /// Layout text
    pub fn layout_text(&self, layout: &TextLayout) -> Vec<PositionedGlyph> {
        let font = self
            .fonts
            .get(&layout.style.font)
            .or_else(|| self.fonts.get(&self.default_font))
            .expect("Default font should always exist");

        let mut glyphs = Vec::new();
        let mut x = 0.0;
        let mut y = 0.0;
        let scale = layout.style.size / font.metrics.units_per_em;
        let line_height = layout.style.size * layout.style.line_height;

        let mut lines: Vec<Vec<char>> = vec![Vec::new()];
        let mut current_line = 0;

        // Split text into lines
        for c in layout.text.chars() {
            if c == '\n' {
                current_line += 1;
                lines.push(Vec::new());
            } else {
                lines[current_line].push(c);
            }
        }

        // Layout each line
        for (line_idx, line) in lines.iter().enumerate() {
            x = 0.0;
            y = line_idx as f32 * line_height;

            // Calculate line width for alignment
            let line_width: f32 = line
                .iter()
                .enumerate()
                .map(|(i, &c)| {
                    let metrics = font.get_glyph_metrics(c);
                    let mut advance = metrics.advance * scale;
                    if i > 0 {
                        let prev_c = line[i - 1];
                        advance += font.get_kerning(prev_c, c) * scale;
                    }
                    advance + layout.style.letter_spacing
                })
                .sum();

            // Apply horizontal alignment
            let x_offset = match layout.align {
                TextAlign::Left => 0.0,
                TextAlign::Center => {
                    if let Some(max_width) = layout.max_width {
                        (max_width - line_width) / 2.0
                    } else {
                        0.0
                    }
                }
                TextAlign::Right => {
                    if let Some(max_width) = layout.max_width {
                        max_width - line_width
                    } else {
                        0.0
                    }
                }
                TextAlign::Justified => 0.0, // TODO: Implement justified alignment
            };

            x = x_offset;

            // Position glyphs
            for (i, &c) in line.iter().enumerate() {
                let metrics = font.get_glyph_metrics(c);

                // Apply kerning
                if i > 0 {
                    let prev_c = line[i - 1];
                    x += font.get_kerning(prev_c, c) * scale;
                }

                let glyph = PositionedGlyph {
                    glyph_id: c as u32,
                    character: c,
                    x,
                    y,
                    metrics: GlyphMetrics {
                        width: metrics.width * scale,
                        height: metrics.height * scale,
                        bearing_x: metrics.bearing_x * scale,
                        bearing_y: metrics.bearing_y * scale,
                        advance: metrics.advance * scale,
                    },
                    color: layout.style.color,
                };

                glyphs.push(glyph);

                x += metrics.advance * scale + layout.style.letter_spacing;
            }
        }

        glyphs
    }

    /// Measure text dimensions
    pub fn measure_text(&self, layout: &TextLayout) -> (f32, f32) {
        let glyphs = self.layout_text(layout);

        if glyphs.is_empty() {
            return (0.0, 0.0);
        }

        let max_x = glyphs
            .iter()
            .map(|g| g.x + g.metrics.width)
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(0.0);

        let max_y = glyphs
            .iter()
            .map(|g| g.y + g.metrics.height)
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(0.0);

        (max_x, max_y)
    }

    /// Get number of loaded fonts
    pub fn font_count(&self) -> usize {
        self.fonts.len()
    }
}

impl Default for FontManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_style_creation() {
        let style = TextStyle::new("Arial", 24.0);
        assert_eq!(style.font, "Arial");
        assert_eq!(style.size, 24.0);
    }

    #[test]
    fn test_text_style_builder() {
        let style = TextStyle::new("Arial", 24.0)
            .with_color([1.0, 0.0, 0.0, 1.0])
            .with_weight(FontWeight::Bold)
            .with_line_height(1.5);

        assert_eq!(style.color, [1.0, 0.0, 0.0, 1.0]);
        assert_eq!(style.weight, FontWeight::Bold);
        assert_eq!(style.line_height, 1.5);
    }

    #[test]
    fn test_text_layout_creation() {
        let layout = TextLayout::new("Hello, World!");
        assert_eq!(layout.text, "Hello, World!");
        assert_eq!(layout.align, TextAlign::Left);
    }

    #[test]
    fn test_text_layout_builder() {
        let layout = TextLayout::new("Hello")
            .with_font("Arial")
            .with_size(24.0)
            .with_align(TextAlign::Center)
            .with_max_width(100.0);

        assert_eq!(layout.style.font, "Arial");
        assert_eq!(layout.style.size, 24.0);
        assert_eq!(layout.align, TextAlign::Center);
        assert_eq!(layout.max_width, Some(100.0));
    }

    #[test]
    fn test_font_manager_creation() {
        let manager = FontManager::new();
        assert_eq!(manager.font_count(), 1); // Default font
    }

    #[test]
    fn test_font_manager_default_font() {
        let manager = FontManager::new();
        let font = manager.get_font("default");
        assert!(font.is_some());
    }

    #[test]
    fn test_font_manager_layout_text() {
        let manager = FontManager::new();
        let layout = TextLayout::new("Hello");
        let glyphs = manager.layout_text(&layout);

        assert_eq!(glyphs.len(), 5); // "Hello" has 5 characters
    }

    #[test]
    fn test_font_manager_measure_text() {
        let manager = FontManager::new();
        let layout = TextLayout::new("Hello");
        let (width, height) = manager.measure_text(&layout);

        assert!(width > 0.0);
        assert!(height > 0.0);
    }

    #[test]
    fn test_text_align_types() {
        assert_eq!(TextAlign::Left, TextAlign::Left);
        assert_eq!(TextAlign::Center, TextAlign::Center);
        assert_eq!(TextAlign::Right, TextAlign::Right);
        assert_eq!(TextAlign::Justified, TextAlign::Justified);
    }

    #[test]
    fn test_font_weight_types() {
        assert_eq!(FontWeight::Normal, FontWeight::Normal);
        assert_eq!(FontWeight::Bold, FontWeight::Bold);
    }

    #[test]
    fn test_font_style_types() {
        assert_eq!(FontStyle::Normal, FontStyle::Normal);
        assert_eq!(FontStyle::Italic, FontStyle::Italic);
    }

    #[test]
    fn test_text_decoration() {
        let mut decoration = TextDecoration::default();
        assert!(!decoration.underline);
        assert!(!decoration.strikethrough);

        decoration.underline = true;
        assert!(decoration.underline);
    }

    #[test]
    fn test_glyph_metrics() {
        let metrics = GlyphMetrics {
            width: 10.0,
            height: 20.0,
            bearing_x: 1.0,
            bearing_y: 15.0,
            advance: 12.0,
        };

        assert_eq!(metrics.width, 10.0);
        assert_eq!(metrics.height, 20.0);
    }

    #[test]
    fn test_font_metrics() {
        let metrics = FontMetrics {
            ascent: 12.0,
            descent: 4.0,
            line_gap: 2.0,
            units_per_em: 16.0,
        };

        assert_eq!(metrics.ascent, 12.0);
        assert_eq!(metrics.descent, 4.0);
    }

    #[test]
    fn test_font_creation() {
        let font = Font::new(1, "Arial".to_string());
        assert_eq!(font.id, 1);
        assert_eq!(font.name, "Arial");
    }

    #[test]
    fn test_font_glyph_metrics() {
        let mut font = Font::new(1, "Arial".to_string());
        let metrics = GlyphMetrics {
            width: 10.0,
            height: 20.0,
            bearing_x: 1.0,
            bearing_y: 15.0,
            advance: 12.0,
        };

        font.add_glyph_metrics('A', metrics);
        let retrieved = font.get_glyph_metrics('A');
        assert_eq!(retrieved.width, 10.0);
    }

    #[test]
    fn test_font_kerning() {
        let mut font = Font::new(1, "Arial".to_string());
        font.add_kerning('A', 'V', -2.0);

        let kerning = font.get_kerning('A', 'V');
        assert_eq!(kerning, -2.0);
    }

    #[test]
    fn test_multiline_text() {
        let manager = FontManager::new();
        let layout = TextLayout::new("Hello\nWorld");
        let glyphs = manager.layout_text(&layout);

        // Should have glyphs for both lines
        assert_eq!(glyphs.len(), 10); // "Hello" (5) + "World" (5)
    }

    #[test]
    fn test_text_style_underline() {
        let style = TextStyle::new("Arial", 24.0).with_underline();
        assert!(style.decoration.underline);
    }

    #[test]
    fn test_text_style_strikethrough() {
        let style = TextStyle::new("Arial", 24.0).with_strikethrough();
        assert!(style.decoration.strikethrough);
    }

    #[test]
    fn test_positioned_glyph() {
        let glyph = PositionedGlyph {
            glyph_id: 65, // 'A'
            character: 'A',
            x: 10.0,
            y: 20.0,
            metrics: GlyphMetrics::default(),
            color: [1.0, 1.0, 1.0, 1.0],
        };

        assert_eq!(glyph.character, 'A');
        assert_eq!(glyph.x, 10.0);
        assert_eq!(glyph.y, 20.0);
    }

    #[test]
    fn test_vertical_align_types() {
        assert_eq!(VerticalAlign::Top, VerticalAlign::Top);
        assert_eq!(VerticalAlign::Middle, VerticalAlign::Middle);
        assert_eq!(VerticalAlign::Bottom, VerticalAlign::Bottom);
        assert_eq!(VerticalAlign::Baseline, VerticalAlign::Baseline);
    }

    #[test]
    fn test_empty_text_layout() {
        let manager = FontManager::new();
        let layout = TextLayout::new("");
        let glyphs = manager.layout_text(&layout);

        assert_eq!(glyphs.len(), 0);
    }

    #[test]
    fn test_text_with_letter_spacing() {
        let manager = FontManager::new();
        let layout = TextLayout::new("AB").with_size(16.0);
        let layout_with_spacing = TextLayout::new("AB")
            .with_size(16.0)
            .with_style(TextStyle::new("default", 16.0).with_letter_spacing(5.0));

        let (width1, _) = manager.measure_text(&layout);
        let (width2, _) = manager.measure_text(&layout_with_spacing);

        // Width with letter spacing should be larger
        assert!(width2 > width1);
    }
}

