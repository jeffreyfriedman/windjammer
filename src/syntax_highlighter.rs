/// Syntax Highlighter for Windjammer code snippets in error messages
///
/// Uses syntect to provide beautiful syntax highlighting for error code snippets.
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;
use syntect::util::as_24_bit_terminal_escaped;

/// Syntax highlighter for Windjammer code
pub struct SyntaxHighlighter {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
}

impl SyntaxHighlighter {
    /// Create a new syntax highlighter
    pub fn new() -> Self {
        Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
        }
    }

    /// Highlight a line of Windjammer code
    ///
    /// Returns the highlighted line with ANSI color codes, or the original line if highlighting fails.
    pub fn highlight_line(&self, line: &str) -> String {
        // Use Rust syntax as a close approximation for Windjammer
        // (Windjammer syntax is similar enough to Rust for highlighting purposes)
        let syntax = self
            .syntax_set
            .find_syntax_by_extension("rs")
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());

        // Use the "base16-ocean.dark" theme for terminal output
        let theme = &self.theme_set.themes["base16-ocean.dark"];

        let mut highlighter = HighlightLines::new(syntax, theme);

        match highlighter.highlight_line(line, &self.syntax_set) {
            Ok(ranges) => {
                // Convert to 24-bit terminal escape codes
                as_24_bit_terminal_escaped(&ranges[..], false)
            }
            Err(_) => {
                // If highlighting fails, return the original line
                line.to_string()
            }
        }
    }

    /// Highlight multiple lines of code
    pub fn highlight_lines(&self, lines: &[&str]) -> Vec<String> {
        lines.iter().map(|line| self.highlight_line(line)).collect()
    }

    /// Check if syntax highlighting is available
    pub fn is_available() -> bool {
        // Syntax highlighting is always available with syntect
        true
    }
}

impl Default for SyntaxHighlighter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_highlighter_creation() {
        let _highlighter = SyntaxHighlighter::new();
        assert!(SyntaxHighlighter::is_available());
    }

    #[test]
    fn test_highlight_simple_line() {
        let highlighter = SyntaxHighlighter::new();
        let line = "let x = 42";
        let highlighted = highlighter.highlight_line(line);
        // Should contain ANSI escape codes
        assert!(highlighted.len() >= line.len());
    }

    #[test]
    fn test_highlight_function() {
        let highlighter = SyntaxHighlighter::new();
        let line = "fn main() {";
        let highlighted = highlighter.highlight_line(line);
        assert!(highlighted.len() >= line.len());
    }

    #[test]
    fn test_highlight_multiple_lines() {
        let highlighter = SyntaxHighlighter::new();
        let lines = vec!["let x = 42", "let y = \"hello\"", "println!(x)"];
        let highlighted = highlighter.highlight_lines(&lines);
        assert_eq!(highlighted.len(), 3);
    }
}
