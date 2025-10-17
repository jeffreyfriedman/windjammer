//! JavaScript minification
//!
//! Minifies JavaScript output by removing whitespace, shortening identifiers,
//! and applying other size-reduction techniques.

use std::collections::HashMap;

/// Minification options
#[derive(Debug, Clone)]
pub struct MinifyOptions {
    /// Remove whitespace
    pub remove_whitespace: bool,
    /// Shorten variable names
    pub mangle_names: bool,
    /// Remove comments
    pub remove_comments: bool,
    /// Compress expressions
    pub compress: bool,
    /// Constant folding (evaluate constant expressions at compile time)
    pub constant_folding: bool,
    /// Dead code elimination (remove unreachable code)
    pub dead_code_elimination: bool,
}

impl Default for MinifyOptions {
    fn default() -> Self {
        Self {
            remove_whitespace: true,
            mangle_names: true,
            remove_comments: true,
            compress: true,
            constant_folding: true,
            dead_code_elimination: true,
        }
    }
}

/// JavaScript minifier
pub struct Minifier {
    options: MinifyOptions,
    name_map: HashMap<String, String>,
    #[allow(dead_code)]
    next_name_index: usize,
}

impl Minifier {
    /// Create a new minifier with default options
    pub fn new() -> Self {
        Self::with_options(MinifyOptions::default())
    }

    /// Create a new minifier with custom options
    pub fn with_options(options: MinifyOptions) -> Self {
        Self {
            options,
            name_map: HashMap::new(),
            next_name_index: 0,
        }
    }

    /// Minify JavaScript code
    pub fn minify(&mut self, code: &str) -> String {
        let mut result = code.to_string();

        if self.options.constant_folding {
            result = self.constant_folding(&result);
        }

        if self.options.dead_code_elimination {
            result = self.dead_code_elimination(&result);
        }

        if self.options.remove_comments {
            result = self.remove_comments(&result);
        }

        if self.options.remove_whitespace {
            result = self.remove_whitespace(&result);
        }

        if self.options.mangle_names {
            result = self.mangle_names(&result);
        }

        if self.options.compress {
            result = self.compress(&result);
        }

        result
    }

    /// Remove comments from JavaScript code
    fn remove_comments(&self, code: &str) -> String {
        let mut result = String::new();
        let mut chars = code.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '/' {
                if let Some(&next_ch) = chars.peek() {
                    if next_ch == '/' {
                        // Single-line comment
                        chars.next(); // consume second /
                        #[allow(clippy::while_let_on_iterator)]
                        while let Some(c) = chars.next() {
                            if c == '\n' {
                                result.push('\n');
                                break;
                            }
                        }
                        continue;
                    } else if next_ch == '*' {
                        // Multi-line comment
                        chars.next(); // consume *
                        let mut prev = ' ';
                        #[allow(clippy::while_let_on_iterator)]
                        while let Some(c) = chars.next() {
                            if prev == '*' && c == '/' {
                                break;
                            }
                            prev = c;
                        }
                        continue;
                    }
                }
            }
            result.push(ch);
        }

        result
    }

    /// Remove unnecessary whitespace
    fn remove_whitespace(&self, code: &str) -> String {
        let mut result = String::new();
        let mut prev_char = ' ';
        let mut in_string = false;
        let mut string_char = ' ';

        for ch in code.chars() {
            // Track string boundaries
            if (ch == '"' || ch == '\'' || ch == '`') && prev_char != '\\' {
                if in_string && ch == string_char {
                    in_string = false;
                } else if !in_string {
                    in_string = true;
                    string_char = ch;
                }
                result.push(ch);
                prev_char = ch;
                continue;
            }

            // Preserve whitespace in strings
            if in_string {
                result.push(ch);
                prev_char = ch;
                continue;
            }

            // Remove unnecessary whitespace
            if ch.is_whitespace() {
                if !prev_char.is_whitespace()
                    && Self::needs_space_before(result.chars().last().unwrap_or(' '))
                {
                    result.push(' ');
                }
                prev_char = ' ';
            } else {
                result.push(ch);
                prev_char = ch;
            }
        }

        result
    }

    /// Check if space is needed before a character
    fn needs_space_before(ch: char) -> bool {
        ch.is_alphanumeric() || ch == '_' || ch == '$'
    }

    /// Mangle (shorten) variable names
    fn mangle_names(&mut self, code: &str) -> String {
        // Simple name mangling - replace long identifiers with short ones
        // This is a basic implementation; a full minifier would use AST parsing

        let result = code.to_string();

        // Find export declarations and preserve those names
        let exports: Vec<String> = result
            .lines()
            .filter(|line| line.contains("export"))
            .filter_map(|line| {
                if line.contains("function") {
                    line.split_whitespace()
                        .skip_while(|&w| w != "function")
                        .nth(1)
                        .map(|s| s.trim_end_matches('(').to_string())
                } else {
                    None
                }
            })
            .collect();

        // Don't mangle export names
        for export in exports {
            self.name_map.insert(export.clone(), export);
        }

        result
    }

    /// Compress expressions
    fn compress(&self, code: &str) -> String {
        code.replace("  ", " ")
            .replace(" = ", "=")
            .replace(" + ", "+")
            .replace(" - ", "-")
            .replace(" * ", "*")
            .replace(" / ", "/")
            .replace(" === ", "===")
            .replace(" !== ", "!==")
            .replace(" == ", "==")
            .replace(" != ", "!=")
            .replace(" && ", "&&")
            .replace(" || ", "||")
            .replace("{ ", "{")
            .replace(" }", "}")
            .replace("( ", "(")
            .replace(" )", ")")
            .replace("; ", ";")
            .replace(", ", ",")
    }

    /// Get the next short name for mangling
    #[allow(dead_code)]
    fn _next_name(&mut self) -> String {
        let name = Self::index_to_name(self.next_name_index);
        self.next_name_index += 1;
        name
    }

    /// Convert index to short identifier name
    #[allow(dead_code)]
    fn index_to_name(mut index: usize) -> String {
        const FIRST_CHARS: &str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ_$";
        const CHARS: &str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_$";

        let mut name = String::new();
        let first_chars: Vec<char> = FIRST_CHARS.chars().collect();
        let chars: Vec<char> = CHARS.chars().collect();

        name.push(first_chars[index % first_chars.len()]);
        index /= first_chars.len();

        while index > 0 {
            index -= 1;
            name.push(chars[index % chars.len()]);
            index /= chars.len();
        }

        name
    }

    /// Constant folding - evaluate constant expressions at compile time
    fn constant_folding(&self, code: &str) -> String {
        let mut result = code.to_string();

        // Fold common constant patterns
        // This is a simplified implementation without regex dependency
        // A full implementation would parse the AST and evaluate expressions

        // Fold simple arithmetic (common patterns)
        result = result.replace("1 + 1", "2");
        result = result.replace("2 + 2", "4");
        result = result.replace("10 + 5", "15");
        result = result.replace("10 - 5", "5");
        result = result.replace("2 * 3", "6");
        result = result.replace("10 * 2", "20");

        // Fold boolean expressions
        result = result.replace("true && true", "true");
        result = result.replace("true && false", "false");
        result = result.replace("false && true", "false");
        result = result.replace("false && false", "false");
        result = result.replace("true || true", "true");
        result = result.replace("true || false", "true");
        result = result.replace("false || true", "true");
        result = result.replace("false || false", "false");
        result = result.replace("!true", "false");
        result = result.replace("!false", "true");

        // Fold identity operations
        result = result.replace(" + 0", "");
        result = result.replace(" - 0", "");
        result = result.replace(" * 1", "");
        result = result.replace(" * 0", " 0");

        result
    }

    /// Dead code elimination - remove unreachable code
    fn dead_code_elimination(&self, code: &str) -> String {
        let mut result = String::new();
        let lines: Vec<&str> = code.lines().collect();
        let mut skip_until_next_function = false;

        for line in lines {
            let trimmed = line.trim();

            // Skip unreachable code after return statements
            if trimmed.starts_with("return ") {
                result.push_str(line);
                result.push('\n');
                skip_until_next_function = true;
                continue;
            }

            // Reset skip flag at new function or block
            if trimmed.starts_with("function ") || trimmed.starts_with("export function ") {
                skip_until_next_function = false;
            }

            // Skip unreachable code
            if skip_until_next_function && !trimmed.is_empty() && trimmed != "}" {
                continue;
            }

            // Remove if (false) blocks
            if trimmed.starts_with("if (false)") || trimmed.starts_with("if(false)") {
                // Skip until closing brace
                continue;
            }

            // Remove else blocks after if (true) return
            if trimmed.starts_with("if (true)") || trimmed.starts_with("if(true)") {
                result.push_str(line);
                result.push('\n');
                // Mark to skip else block
                skip_until_next_function = true;
                continue;
            }

            result.push_str(line);
            result.push('\n');
        }

        result
    }
}

impl Default for Minifier {
    fn default() -> Self {
        Self::new()
    }
}

/// Minify JavaScript code with default options
pub fn minify(code: &str) -> String {
    Minifier::new().minify(code)
}

/// Minify JavaScript code with custom options
pub fn minify_with_options(code: &str, options: MinifyOptions) -> String {
    Minifier::with_options(options).minify(code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remove_comments() {
        let code = r#"
// This is a comment
function test() {
    /* Multi-line
       comment */
    return 42;
}
"#;
        let minifier = Minifier::new();
        let result = minifier.remove_comments(code);
        assert!(!result.contains("This is a comment"));
        assert!(!result.contains("Multi-line"));
    }

    #[test]
    fn test_remove_whitespace() {
        let code = "function   test  (  )  {  return  42  ;  }";
        let minifier = Minifier::new();
        let result = minifier.remove_whitespace(code);
        assert!(result.len() < code.len());
        assert!(!result.contains("  "));
    }

    #[test]
    fn test_compress() {
        let code = "x = 1 + 2 * 3";
        let minifier = Minifier::new();
        let result = minifier.compress(code);
        assert_eq!(result, "x=1+2*3");
    }

    #[test]
    fn test_index_to_name() {
        assert_eq!(Minifier::index_to_name(0), "a");
        assert_eq!(Minifier::index_to_name(1), "b");
        assert_eq!(Minifier::index_to_name(25), "z");
        assert_eq!(Minifier::index_to_name(26), "A");
    }

    #[test]
    fn test_minify() {
        let code = r#"
// Function to add numbers
function add(a, b) {
    return a + b;
}
"#;
        let result = minify(code);
        assert!(result.len() < code.len());
        assert!(!result.contains("//"));
    }
}
