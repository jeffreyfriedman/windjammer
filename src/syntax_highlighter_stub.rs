/// Fallback syntax highlighter when the `highlighting` feature is disabled.
/// Returns unmodified text (no ANSI color codes).
pub struct SyntaxHighlighter;

impl SyntaxHighlighter {
    pub fn new() -> Self {
        Self
    }

    pub fn highlight_line(&self, line: &str) -> String {
        line.to_string()
    }

    pub fn highlight_lines(&self, lines: &[&str]) -> Vec<String> {
        lines.iter().map(|line| line.to_string()).collect()
    }

    pub fn is_available() -> bool {
        false
    }
}
