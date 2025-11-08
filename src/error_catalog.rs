/// Error Catalog: Generate comprehensive error documentation
///
/// This module generates a searchable error catalog with examples,
/// explanations, and solutions for all Windjammer errors.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Error catalog entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorEntry {
    /// Error code (e.g., "E0425", "WJ0001")
    pub code: String,
    /// Short error title
    pub title: String,
    /// Detailed explanation
    pub explanation: String,
    /// Common causes
    pub causes: Vec<String>,
    /// Solutions
    pub solutions: Vec<String>,
    /// Code examples that trigger this error
    pub examples: Vec<ErrorExample>,
    /// Related errors
    pub related: Vec<String>,
    /// Severity (error, warning, note)
    pub severity: String,
    /// Category (type, ownership, syntax, etc.)
    pub category: String,
}

/// Code example for an error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorExample {
    /// Example title
    pub title: String,
    /// Code that triggers the error
    pub bad_code: String,
    /// Fixed code
    pub good_code: String,
    /// Explanation of the fix
    pub explanation: String,
}

/// Error catalog
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorCatalog {
    /// All error entries indexed by code
    pub errors: HashMap<String, ErrorEntry>,
    /// Categories
    pub categories: Vec<String>,
    /// Version
    pub version: String,
}

impl ErrorCatalog {
    /// Create a new error catalog with common errors
    pub fn new() -> Self {
        let mut catalog = Self {
            errors: HashMap::new(),
            categories: vec![
                "Type Errors".to_string(),
                "Ownership Errors".to_string(),
                "Syntax Errors".to_string(),
                "Module Errors".to_string(),
                "Trait Errors".to_string(),
            ],
            version: "0.1.0".to_string(),
        };

        // Add common Rust errors with Windjammer translations
        catalog.add_common_errors();
        catalog
    }

    /// Add common errors to the catalog
    fn add_common_errors(&mut self) {
        // E0425: Variable not found
        self.add_error(ErrorEntry {
            code: "E0425".to_string(),
            title: "Variable not found".to_string(),
            explanation: "The compiler cannot find a variable, function, or constant with this name in the current scope.".to_string(),
            causes: vec![
                "Typo in the variable name".to_string(),
                "Variable not declared before use".to_string(),
                "Variable is out of scope".to_string(),
                "Module not imported".to_string(),
            ],
            solutions: vec![
                "Check the spelling of the variable name".to_string(),
                "Declare the variable before using it: let x = 42".to_string(),
                "Import the module: use std::collections::HashMap".to_string(),
                "Check variable scope (declared inside a block?)".to_string(),
            ],
            examples: vec![
                ErrorExample {
                    title: "Typo in variable name".to_string(),
                    bad_code: r#"let count = 10
println!("{}", cont)  // Typo: 'cont' instead of 'count'"#.to_string(),
                    good_code: r#"let count = 10
println!("{}", count)  // Fixed!"#.to_string(),
                    explanation: "Fixed the typo in the variable name".to_string(),
                },
                ErrorExample {
                    title: "Variable not declared".to_string(),
                    bad_code: r#"println!("{}", total)  // 'total' not declared"#.to_string(),
                    good_code: r#"let total = 100
println!("{}", total)  // Declared first!"#.to_string(),
                    explanation: "Declared the variable before using it".to_string(),
                },
            ],
            related: vec!["E0412".to_string(), "E0433".to_string()],
            severity: "error".to_string(),
            category: "Type Errors".to_string(),
        });

        // E0308: Type mismatch
        self.add_error(ErrorEntry {
            code: "E0308".to_string(),
            title: "Type mismatch".to_string(),
            explanation: "The compiler expected one type but found another. Types must match exactly in Windjammer.".to_string(),
            causes: vec![
                "Passing wrong type to function".to_string(),
                "Assigning wrong type to variable".to_string(),
                "Returning wrong type from function".to_string(),
                "String vs integer confusion".to_string(),
            ],
            solutions: vec![
                "Use .parse() to convert string to number: \"42\".parse()".to_string(),
                "Use .to_string() to convert number to string: 42.to_string()".to_string(),
                "Check function signature for expected types".to_string(),
                "Use type annotations if needed: let x: int = 42".to_string(),
            ],
            examples: vec![
                ErrorExample {
                    title: "String to int conversion".to_string(),
                    bad_code: r#"let x: int = "42"  // String, not int"#.to_string(),
                    good_code: r#"let x: int = "42".parse()  // Convert to int"#.to_string(),
                    explanation: "Used .parse() to convert string to integer".to_string(),
                },
                ErrorExample {
                    title: "Int to string conversion".to_string(),
                    bad_code: r#"let s: string = 42  // Int, not string"#.to_string(),
                    good_code: r#"let s: string = 42.to_string()  // Convert to string"#.to_string(),
                    explanation: "Used .to_string() to convert integer to string".to_string(),
                },
            ],
            related: vec!["E0277".to_string()],
            severity: "error".to_string(),
            category: "Type Errors".to_string(),
        });

        // E0384: Immutability error
        self.add_error(ErrorEntry {
            code: "E0384".to_string(),
            title: "Cannot modify immutable variable".to_string(),
            explanation: "Variables in Windjammer are immutable by default. To modify a variable, declare it as mutable with 'let mut'.".to_string(),
            causes: vec![
                "Trying to modify immutable variable".to_string(),
                "Forgot 'mut' keyword".to_string(),
            ],
            solutions: vec![
                "Declare variable as mutable: let mut x = 42".to_string(),
                "Create a new variable instead of modifying".to_string(),
            ],
            examples: vec![
                ErrorExample {
                    title: "Modifying immutable variable".to_string(),
                    bad_code: r#"let x = 10
x = 20  // Error: x is immutable"#.to_string(),
                    good_code: r#"let mut x = 10
x = 20  // Works! x is mutable"#.to_string(),
                    explanation: "Added 'mut' keyword to make variable mutable".to_string(),
                },
            ],
            related: vec!["E0596".to_string()],
            severity: "error".to_string(),
            category: "Ownership Errors".to_string(),
        });
    }

    /// Add an error to the catalog
    pub fn add_error(&mut self, error: ErrorEntry) {
        self.errors.insert(error.code.clone(), error);
    }

    /// Get an error by code
    pub fn get_error(&self, code: &str) -> Option<&ErrorEntry> {
        self.errors.get(code)
    }

    /// Search errors by keyword
    pub fn search(&self, keyword: &str) -> Vec<&ErrorEntry> {
        let keyword_lower = keyword.to_lowercase();
        self.errors
            .values()
            .filter(|error| {
                error.title.to_lowercase().contains(&keyword_lower)
                    || error.explanation.to_lowercase().contains(&keyword_lower)
                    || error.code.to_lowercase().contains(&keyword_lower)
            })
            .collect()
    }

    /// Get errors by category
    pub fn get_by_category(&self, category: &str) -> Vec<&ErrorEntry> {
        self.errors
            .values()
            .filter(|error| error.category == category)
            .collect()
    }

    /// Generate HTML documentation
    pub fn generate_html(&self) -> String {
        let mut html = String::new();

        // HTML header
        html.push_str("<!DOCTYPE html>\n");
        html.push_str("<html lang=\"en\">\n");
        html.push_str("<head>\n");
        html.push_str("  <meta charset=\"UTF-8\">\n");
        html.push_str("  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n");
        html.push_str("  <title>Windjammer Error Catalog</title>\n");
        html.push_str("  <style>\n");
        html.push_str(Self::get_default_css());
        html.push_str("  </style>\n");
        html.push_str("</head>\n");
        html.push_str("<body>\n");

        // Header
        html.push_str("  <header>\n");
        html.push_str("    <h1>Windjammer Error Catalog</h1>\n");
        html.push_str(&format!("    <p>Version {}</p>\n", self.version));
        html.push_str("  </header>\n");

        // Navigation
        html.push_str("  <nav>\n");
        html.push_str("    <h2>Categories</h2>\n");
        html.push_str("    <ul>\n");
        for category in &self.categories {
            let anchor = category.replace(" ", "-").to_lowercase();
            html.push_str(&format!("      <li><a href=\"#{}\">{}</a></li>\n", anchor, category));
        }
        html.push_str("    </ul>\n");
        html.push_str("  </nav>\n");

        // Main content
        html.push_str("  <main>\n");

        for category in &self.categories {
            let anchor = category.replace(" ", "-").to_lowercase();
            html.push_str(&format!("    <section id=\"{}\">\n", anchor));
            html.push_str(&format!("      <h2>{}</h2>\n", category));

            let errors = self.get_by_category(category);
            for error in errors {
                html.push_str(&self.generate_error_html(error));
            }

            html.push_str("    </section>\n");
        }

        html.push_str("  </main>\n");
        html.push_str("</body>\n");
        html.push_str("</html>\n");

        html
    }

    /// Generate HTML for a single error
    fn generate_error_html(&self, error: &ErrorEntry) -> String {
        let mut html = String::new();

        html.push_str(&format!("      <article class=\"error\" id=\"{}\">\n", error.code));
        html.push_str(&format!("        <h3>{} - {}</h3>\n", error.code, error.title));
        html.push_str(&format!("        <p class=\"explanation\">{}</p>\n", error.explanation));

        // Causes
        if !error.causes.is_empty() {
            html.push_str("        <h4>Common Causes:</h4>\n");
            html.push_str("        <ul>\n");
            for cause in &error.causes {
                html.push_str(&format!("          <li>{}</li>\n", cause));
            }
            html.push_str("        </ul>\n");
        }

        // Solutions
        if !error.solutions.is_empty() {
            html.push_str("        <h4>Solutions:</h4>\n");
            html.push_str("        <ul>\n");
            for solution in &error.solutions {
                html.push_str(&format!("          <li>{}</li>\n", solution));
            }
            html.push_str("        </ul>\n");
        }

        // Examples
        if !error.examples.is_empty() {
            html.push_str("        <h4>Examples:</h4>\n");
            for example in &error.examples {
                html.push_str("        <div class=\"example\">\n");
                html.push_str(&format!("          <h5>{}</h5>\n", example.title));
                html.push_str("          <div class=\"code-comparison\">\n");
                html.push_str("            <div class=\"bad-code\">\n");
                html.push_str("              <h6>❌ Wrong:</h6>\n");
                html.push_str(&format!("              <pre><code>{}</code></pre>\n", example.bad_code));
                html.push_str("            </div>\n");
                html.push_str("            <div class=\"good-code\">\n");
                html.push_str("              <h6>✅ Correct:</h6>\n");
                html.push_str(&format!("              <pre><code>{}</code></pre>\n", example.good_code));
                html.push_str("            </div>\n");
                html.push_str("          </div>\n");
                html.push_str(&format!("          <p class=\"example-explanation\">{}</p>\n", example.explanation));
                html.push_str("        </div>\n");
            }
        }

        html.push_str("      </article>\n");
        html
    }

    /// Generate Markdown documentation
    pub fn generate_markdown(&self) -> String {
        let mut md = String::new();

        md.push_str("# Windjammer Error Catalog\n\n");
        md.push_str(&format!("Version: {}\n\n", self.version));

        for category in &self.categories {
            md.push_str(&format!("## {}\n\n", category));

            let errors = self.get_by_category(category);
            for error in errors {
                md.push_str(&self.generate_error_markdown(error));
            }
        }

        md
    }

    /// Generate Markdown for a single error
    fn generate_error_markdown(&self, error: &ErrorEntry) -> String {
        let mut md = String::new();

        md.push_str(&format!("### {} - {}\n\n", error.code, error.title));
        md.push_str(&format!("{}\n\n", error.explanation));

        if !error.causes.is_empty() {
            md.push_str("**Common Causes:**\n\n");
            for cause in &error.causes {
                md.push_str(&format!("- {}\n", cause));
            }
            md.push_str("\n");
        }

        if !error.solutions.is_empty() {
            md.push_str("**Solutions:**\n\n");
            for solution in &error.solutions {
                md.push_str(&format!("- {}\n", solution));
            }
            md.push_str("\n");
        }

        if !error.examples.is_empty() {
            md.push_str("**Examples:**\n\n");
            for example in &error.examples {
                md.push_str(&format!("#### {}\n\n", example.title));
                md.push_str("❌ Wrong:\n\n");
                md.push_str(&format!("```windjammer\n{}\n```\n\n", example.bad_code));
                md.push_str("✅ Correct:\n\n");
                md.push_str(&format!("```windjammer\n{}\n```\n\n", example.good_code));
                md.push_str(&format!("{}\n\n", example.explanation));
            }
        }

        md.push_str("---\n\n");
        md
    }

    /// Save catalog to JSON file
    pub fn save_json(&self, path: &Path) -> anyhow::Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }

    /// Load catalog from JSON file
    pub fn load_json(path: &Path) -> anyhow::Result<Self> {
        let json = fs::read_to_string(path)?;
        let catalog: ErrorCatalog = serde_json::from_str(&json)?;
        Ok(catalog)
    }

    /// Get default CSS for HTML generation
    fn get_default_css() -> &'static str {
        include_str!("../docs/error_catalog_style.css")
    }
}

impl Default for ErrorCatalog {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_catalog_creation() {
        let catalog = ErrorCatalog::new();
        assert!(!catalog.errors.is_empty());
        assert!(catalog.errors.contains_key("E0425"));
    }

    #[test]
    fn test_search() {
        let catalog = ErrorCatalog::new();
        let results = catalog.search("variable");
        assert!(!results.is_empty());
    }

    #[test]
    fn test_get_by_category() {
        let catalog = ErrorCatalog::new();
        let errors = catalog.get_by_category("Type Errors");
        assert!(!errors.is_empty());
    }

    #[test]
    fn test_generate_markdown() {
        let catalog = ErrorCatalog::new();
        let md = catalog.generate_markdown();
        assert!(md.contains("# Windjammer Error Catalog"));
        assert!(md.contains("E0425"));
    }

    #[test]
    fn test_generate_html() {
        let catalog = ErrorCatalog::new();
        let html = catalog.generate_html();
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("Windjammer Error Catalog"));
    }
}

