//! Formatter configuration (port). Pure data plus a `wj.toml [fmt]` parser.

/// User-tunable formatting options.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatOptions {
    /// Soft maximum line width; bracketed lists exceeding it are broken one element per line.
    pub max_width: usize,
    /// Number of spaces per indentation level.
    pub indent: usize,
}

impl Default for FormatOptions {
    fn default() -> Self {
        FormatOptions {
            max_width: 100,
            indent: 4,
        }
    }
}

impl FormatOptions {
    /// Read the `[fmt]` table of a `wj.toml` document. Missing or invalid keys keep defaults.
    pub fn from_toml_str(toml_text: &str) -> FormatOptions {
        let mut opts = FormatOptions::default();
        let Ok(doc) = toml_text.parse::<toml::Table>() else {
            return opts;
        };
        let Some(fmt) = doc.get("fmt").and_then(|v| v.as_table()) else {
            return opts;
        };
        if let Some(w) = positive(fmt.get("max_width")) {
            opts.max_width = w;
        }
        if let Some(i) = positive(fmt.get("indent")) {
            opts.indent = i;
        }
        opts
    }
}

fn positive(v: Option<&toml::Value>) -> Option<usize> {
    v.and_then(|v| v.as_integer())
        .filter(|n| *n > 0 && *n <= 1000)
        .map(|n| n as usize)
}
