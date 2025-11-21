/// Error Codes: Windjammer-specific error codes and explanations
///
/// This module defines Windjammer error codes (WJ0001, etc.) and provides
/// detailed explanations for each error.
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Windjammer error code
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WjErrorCode {
    /// Code (e.g., "WJ0001")
    pub code: String,
    /// Short title
    pub title: String,
    /// Detailed explanation
    pub explanation: String,
    /// Common causes
    pub causes: Vec<String>,
    /// Solutions
    pub solutions: Vec<String>,
    /// Example code
    pub example: Option<String>,
    /// Related Rust error codes
    pub rust_codes: Vec<String>,
}

/// Error code registry
pub struct ErrorCodeRegistry {
    /// All Windjammer error codes
    codes: HashMap<String, WjErrorCode>,
}

impl ErrorCodeRegistry {
    /// Create a new error code registry with all Windjammer error codes
    pub fn new() -> Self {
        let mut registry = Self {
            codes: HashMap::new(),
        };
        registry.register_all_codes();
        registry
    }

    /// Register all Windjammer error codes
    fn register_all_codes(&mut self) {
        // WJ0001: Variable not found
        self.register(WjErrorCode {
            code: "WJ0001".to_string(),
            title: "Variable not found".to_string(),
            explanation: "The compiler cannot find a variable with this name in the current scope. This usually means the variable hasn't been declared yet, or it's out of scope.".to_string(),
            causes: vec![
                "Typo in the variable name".to_string(),
                "Variable not declared before use".to_string(),
                "Variable is out of scope (declared in a different block)".to_string(),
            ],
            solutions: vec![
                "Check the spelling of the variable name".to_string(),
                "Declare the variable before using it: let x = 42".to_string(),
                "Make sure the variable is in scope".to_string(),
            ],
            example: Some(r#"// Wrong:
println!("{}", total)  // 'total' not declared

// Correct:
let total = 100
println!("{}", total)"#.to_string()),
            rust_codes: vec!["E0425".to_string()],
        });

        // WJ0002: Function not found
        self.register(WjErrorCode {
            code: "WJ0002".to_string(),
            title: "Function not found".to_string(),
            explanation: "The compiler cannot find a function with this name. This might be because the function hasn't been defined, or the module containing it hasn't been imported.".to_string(),
            causes: vec![
                "Typo in the function name".to_string(),
                "Function not defined".to_string(),
                "Module not imported".to_string(),
            ],
            solutions: vec![
                "Check the spelling of the function name".to_string(),
                "Define the function before calling it".to_string(),
                "Import the module: use std::collections::HashMap".to_string(),
            ],
            example: Some(r#"// Wrong:
proces_data(items)  // Typo: 'proces' instead of 'process'

// Correct:
process_data(items)"#.to_string()),
            rust_codes: vec!["E0425".to_string()],
        });

        // WJ0003: Type mismatch
        self.register(WjErrorCode {
            code: "WJ0003".to_string(),
            title: "Type mismatch".to_string(),
            explanation: "The compiler expected one type but found another. In Windjammer, types must match exactly. You may need to convert between types explicitly.".to_string(),
            causes: vec![
                "Passing wrong type to function".to_string(),
                "Assigning wrong type to variable".to_string(),
                "Returning wrong type from function".to_string(),
            ],
            solutions: vec![
                "Use .parse() to convert string to number: \"42\".parse()".to_string(),
                "Use .to_string() to convert number to string: 42.to_string()".to_string(),
                "Check the function signature for expected types".to_string(),
            ],
            example: Some(r#"// Wrong:
let x: int = "42"  // String, not int

// Correct:
let x: int = "42".parse()  // Convert to int"#.to_string()),
            rust_codes: vec!["E0308".to_string()],
        });

        // WJ0004: Immutable variable
        self.register(WjErrorCode {
            code: "WJ0004".to_string(),
            title: "Cannot modify immutable variable".to_string(),
            explanation: "Variables in Windjammer are immutable by default. To modify a variable, you must declare it as mutable using 'let mut'.".to_string(),
            causes: vec![
                "Trying to modify an immutable variable".to_string(),
                "Forgot 'mut' keyword".to_string(),
            ],
            solutions: vec![
                "Declare the variable as mutable: let mut x = 42".to_string(),
                "Create a new variable instead of modifying the existing one".to_string(),
            ],
            example: Some(r#"// Wrong:
let x = 10
x = 20  // Error: x is immutable

// Correct:
let mut x = 10
x = 20  // Works!"#.to_string()),
            rust_codes: vec!["E0384".to_string(), "E0596".to_string()],
        });

        // WJ0005: Type not found
        self.register(WjErrorCode {
            code: "WJ0005".to_string(),
            title: "Type not found".to_string(),
            explanation: "The compiler cannot find a type with this name. This might be a typo, or the type might be defined in a module that hasn't been imported.".to_string(),
            causes: vec![
                "Typo in the type name".to_string(),
                "Type not defined".to_string(),
                "Module not imported".to_string(),
            ],
            solutions: vec![
                "Check the spelling of the type name".to_string(),
                "Define the type (struct, enum, etc.)".to_string(),
                "Import the module containing the type".to_string(),
            ],
            example: Some(r#"// Wrong:
let map: HashMapp<string, int> = ...  // Typo

// Correct:
let map: HashMap<string, int> = ..."#.to_string()),
            rust_codes: vec!["E0412".to_string()],
        });

        // WJ0006: Module not found
        self.register(WjErrorCode {
            code: "WJ0006".to_string(),
            title: "Module not found".to_string(),
            explanation: "The compiler cannot find a module with this name. Check that the module exists and is in the correct location.".to_string(),
            causes: vec![
                "Typo in the module name".to_string(),
                "Module file doesn't exist".to_string(),
                "Module not in the correct directory".to_string(),
            ],
            solutions: vec![
                "Check the spelling of the module name".to_string(),
                "Create the module file".to_string(),
                "Check the module is in the correct location".to_string(),
            ],
            example: Some(r#"// Wrong:
use std::colections::HashMap  // Typo

// Correct:
use std::collections::HashMap"#.to_string()),
            rust_codes: vec!["E0433".to_string()],
        });

        // WJ0007: Ownership error
        self.register(WjErrorCode {
            code: "WJ0007".to_string(),
            title: "Value was moved".to_string(),
            explanation: "This value was moved to another location and can no longer be used here. In Windjammer, most values can only be used once unless they implement Copy. However, the auto-clone system should handle most cases automatically.".to_string(),
            causes: vec![
                "Value passed to function that takes ownership".to_string(),
                "Value assigned to another variable".to_string(),
                "Auto-clone system couldn't insert clone automatically".to_string(),
            ],
            solutions: vec![
                "Use a reference (&) instead: process(&data)".to_string(),
                "Clone explicitly: process(data.clone())".to_string(),
                "Restructure code to avoid multiple uses".to_string(),
            ],
            example: Some(r#"// Usually handled automatically, but if not:
let data = vec![1, 2, 3]
process(data.clone())  // Explicit clone
println!("{}", data.len())"#.to_string()),
            rust_codes: vec!["E0382".to_string()],
        });

        // WJ0008: Borrow error
        self.register(WjErrorCode {
            code: "WJ0008".to_string(),
            title: "Cannot borrow as mutable".to_string(),
            explanation: "You're trying to borrow a value as mutable, but it's already borrowed as immutable, or vice versa. Windjammer enforces Rust's borrowing rules to ensure memory safety.".to_string(),
            causes: vec![
                "Multiple mutable borrows".to_string(),
                "Mutable borrow while immutable borrow exists".to_string(),
            ],
            solutions: vec![
                "Ensure borrows don't overlap in scope".to_string(),
                "Use only one mutable borrow at a time".to_string(),
                "Restructure code to avoid conflicting borrows".to_string(),
            ],
            example: Some(r#"// Wrong:
let mut x = 5
let y = &x
let z = &mut x  // Error: already borrowed

// Correct:
let mut x = 5
let y = &x
// Use y first, then borrow mutably
let z = &mut x"#.to_string()),
            rust_codes: vec!["E0502".to_string(), "E0503".to_string()],
        });

        // WJ0009: Missing field
        self.register(WjErrorCode {
            code: "WJ0009".to_string(),
            title: "Missing field in struct".to_string(),
            explanation: "When creating a struct, you must provide values for all fields."
                .to_string(),
            causes: vec![
                "Forgot to initialize a field".to_string(),
                "Typo in field name".to_string(),
            ],
            solutions: vec![
                "Add the missing field".to_string(),
                "Check field names match struct definition".to_string(),
            ],
            example: Some(
                r#"// Wrong:
let user = User {
    name: "Alice"
    // Missing 'age' field
}

// Correct:
let user = User {
    name: "Alice",
    age: 30
}"#
                .to_string(),
            ),
            rust_codes: vec!["E0063".to_string()],
        });

        // WJ0010: Pattern match not exhaustive
        self.register(WjErrorCode {
            code: "WJ0010".to_string(),
            title: "Pattern match not exhaustive".to_string(),
            explanation: "Your match expression doesn't cover all possible cases. Windjammer requires all patterns to be handled for safety.".to_string(),
            causes: vec![
                "Missing match arms".to_string(),
                "Forgot wildcard pattern".to_string(),
            ],
            solutions: vec![
                "Add missing match arms".to_string(),
                "Add a wildcard pattern: _ => ...".to_string(),
            ],
            example: Some(r#"// Wrong:
match value {
    Some(x) => println!("{}", x)
    // Missing None case
}

// Correct:
match value {
    Some(x) => println!("{}", x),
    None => println!("No value")
}"#.to_string()),
            rust_codes: vec!["E0004".to_string()],
        });
    }

    /// Register an error code
    fn register(&mut self, code: WjErrorCode) {
        self.codes.insert(code.code.clone(), code);
    }

    /// Get an error code by code string
    pub fn get(&self, code: &str) -> Option<&WjErrorCode> {
        self.codes.get(code)
    }

    /// Get all error codes
    pub fn all_codes(&self) -> Vec<&WjErrorCode> {
        self.codes.values().collect()
    }

    /// Map Rust error code to Windjammer error code
    pub fn map_rust_code(&self, rust_code: &str) -> Option<&WjErrorCode> {
        self.codes
            .values()
            .find(|wj_code| wj_code.rust_codes.contains(&rust_code.to_string()))
    }

    /// Format error explanation for display
    pub fn format_explanation(&self, code: &str) -> String {
        use colored::*;

        if let Some(error) = self.get(code) {
            let mut output = String::new();

            output.push_str(&format!(
                "\n{} {}\n",
                error.code.red().bold(),
                error.title.bold()
            ));
            output.push_str(&format!("{}\n\n", "=".repeat(60)));

            output.push_str(&format!("{}\n", "Explanation:".yellow().bold()));
            output.push_str(&format!("{}\n\n", error.explanation));

            if !error.causes.is_empty() {
                output.push_str(&format!("{}\n", "Common Causes:".yellow().bold()));
                for (i, cause) in error.causes.iter().enumerate() {
                    output.push_str(&format!("  {}. {}\n", i + 1, cause));
                }
                output.push('\n');
            }

            if !error.solutions.is_empty() {
                output.push_str(&format!("{}\n", "Solutions:".green().bold()));
                for (i, solution) in error.solutions.iter().enumerate() {
                    output.push_str(&format!("  {}. {}\n", i + 1, solution));
                }
                output.push('\n');
            }

            if let Some(ref example) = error.example {
                output.push_str(&format!("{}\n", "Example:".cyan().bold()));
                output.push_str(&format!("{}\n\n", example));
            }

            if !error.rust_codes.is_empty() {
                output.push_str(&format!("{}\n", "Related Rust Errors:".dimmed()));
                output.push_str(&format!("  {}\n", error.rust_codes.join(", ").dimmed()));
            }

            output
        } else {
            format!("Error code {} not found", code)
        }
    }
}

impl Default for ErrorCodeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Get the global error code registry
pub fn get_registry() -> &'static ErrorCodeRegistry {
    use std::sync::OnceLock;
    static REGISTRY: OnceLock<ErrorCodeRegistry> = OnceLock::new();
    REGISTRY.get_or_init(ErrorCodeRegistry::new)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_creation() {
        let registry = ErrorCodeRegistry::new();
        assert!(!registry.codes.is_empty());
        assert!(registry.get("WJ0001").is_some());
    }

    #[test]
    fn test_get_error_code() {
        let registry = ErrorCodeRegistry::new();
        let code = registry.get("WJ0001").unwrap();
        assert_eq!(code.code, "WJ0001");
        assert_eq!(code.title, "Variable not found");
    }

    #[test]
    fn test_map_rust_code() {
        let registry = ErrorCodeRegistry::new();
        let wj_code = registry.map_rust_code("E0425").unwrap();
        assert!(wj_code.code == "WJ0001" || wj_code.code == "WJ0002");
    }

    #[test]
    fn test_format_explanation() {
        let registry = ErrorCodeRegistry::new();
        let explanation = registry.format_explanation("WJ0001");
        assert!(explanation.contains("WJ0001"));
        assert!(explanation.contains("Variable not found"));
    }

    #[test]
    fn test_all_codes() {
        let registry = ErrorCodeRegistry::new();
        let codes = registry.all_codes();
        assert!(codes.len() >= 10);
    }
}
