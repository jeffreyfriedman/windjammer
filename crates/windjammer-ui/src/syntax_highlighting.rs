/// Syntax highlighting for Windjammer code using syntect
#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
use syntect::easy::HighlightLines;
#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
use syntect::highlighting::{Style, ThemeSet};
#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
use syntect::parsing::SyntaxSet;
#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
use syntect::util::LinesWithEndings;

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
pub struct SyntaxHighlighter {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
impl SyntaxHighlighter {
    pub fn new() -> Self {
        Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
        }
    }

    pub fn highlight_code(&self, code: &str, language: &str) -> Vec<(Style, String)> {
        let syntax = self
            .syntax_set
            .find_syntax_by_extension(language)
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());

        let theme = &self.theme_set.themes["base16-ocean.dark"];
        let mut highlighter = HighlightLines::new(syntax, theme);

        let mut result = Vec::new();
        for line in LinesWithEndings::from(code) {
            let ranges = highlighter
                .highlight_line(line, &self.syntax_set)
                .unwrap_or_default();
            for (style, text) in ranges {
                result.push((style, text.to_string()));
            }
        }
        result
    }

    pub fn highlight_windjammer(&self, code: &str) -> Vec<(Style, String)> {
        // For now, use Rust syntax as it's similar to Windjammer
        // TODO: Create custom Windjammer syntax definition
        self.highlight_code(code, "rs")
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
impl Default for SyntaxHighlighter {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert syntect Style to egui Color32
#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
pub fn style_to_color(style: Style) -> egui::Color32 {
    egui::Color32::from_rgb(style.foreground.r, style.foreground.g, style.foreground.b)
}

/// Render highlighted code in egui
#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
pub fn render_highlighted_code(
    ui: &mut egui::Ui,
    highlighted: &[(Style, String)],
    font_id: egui::FontId,
) {
    use egui::text::{LayoutJob, TextFormat};

    let mut job = LayoutJob::default();
    for (style, text) in highlighted {
        job.append(
            text,
            0.0,
            TextFormat {
                font_id: font_id.clone(),
                color: style_to_color(*style),
                ..Default::default()
            },
        );
    }

    ui.label(job);
}
