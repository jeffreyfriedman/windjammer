//! Doc test extraction and execution utilities
//!
//! Provides runtime support for extracting and running tests from documentation.

use std::collections::HashMap;

/// Represents a doc test extracted from documentation
#[derive(Debug, Clone)]
pub struct DocTest {
    pub name: String,
    pub code: String,
    pub line: usize,
    pub should_panic: bool,
    pub ignore: bool,
}

impl DocTest {
    pub fn new(name: String, code: String, line: usize) -> Self {
        Self {
            name,
            code,
            line,
            should_panic: false,
            ignore: false,
        }
    }

    pub fn with_should_panic(mut self) -> Self {
        self.should_panic = true;
        self
    }

    pub fn with_ignore(mut self) -> Self {
        self.ignore = true;
        self
    }
}

/// Registry for doc tests
#[derive(Debug, Default)]
pub struct DocTestRegistry {
    tests: HashMap<String, Vec<DocTest>>,
}

impl DocTestRegistry {
    pub fn new() -> Self {
        Self {
            tests: HashMap::new(),
        }
    }

    /// Register a doc test
    pub fn register(&mut self, module: &str, test: DocTest) {
        self.tests.entry(module.to_string()).or_default().push(test);
    }

    /// Get all tests for a module
    pub fn get_tests(&self, module: &str) -> Option<&Vec<DocTest>> {
        self.tests.get(module)
    }

    /// Get all modules with doc tests
    pub fn modules(&self) -> Vec<&String> {
        self.tests.keys().collect()
    }

    /// Total number of doc tests
    pub fn total_tests(&self) -> usize {
        self.tests.values().map(|v| v.len()).sum()
    }
}

/// Extract doc tests from a doc comment
///
/// Looks for ` ```test ` or ` ``` ` code blocks and extracts them as tests.
///
/// # Example
/// ```
/// use windjammer_runtime::doc_test::extract_doc_tests;
///
/// let doc = r#"
/// /// This function adds two numbers.
/// ///
/// /// # Example
/// /// ```
/// /// let result = add(2, 3);
/// /// assert_eq!(result, 5);
/// /// ```
/// "#;
///
/// let tests = extract_doc_tests("my_module", doc);
/// assert_eq!(tests.len(), 1);
/// ```
pub fn extract_doc_tests(module: &str, doc_comment: &str) -> Vec<DocTest> {
    let mut tests = Vec::new();
    let mut in_code_block = false;
    let mut current_code = String::new();
    let mut start_line = 0;
    let mut should_panic = false;
    let mut ignore = false;

    for (line_num, line) in doc_comment.lines().enumerate() {
        let trimmed = line.trim_start_matches("///").trim();

        if trimmed.starts_with("```") {
            if in_code_block {
                // End of code block
                if !current_code.is_empty() {
                    let mut test = DocTest::new(
                        format!("{}_doctest_{}", module, tests.len()),
                        current_code.clone(),
                        start_line,
                    );

                    if should_panic {
                        test = test.with_should_panic();
                    }
                    if ignore {
                        test = test.with_ignore();
                    }

                    tests.push(test);
                }
                in_code_block = false;
                current_code.clear();
                should_panic = false;
                ignore = false;
            } else {
                // Start of code block
                in_code_block = true;
                start_line = line_num;

                // Check for attributes
                let block_type = trimmed.trim_start_matches("```");
                should_panic = block_type.contains("should_panic");
                ignore = block_type.contains("ignore");
            }
        } else if in_code_block {
            current_code.push_str(trimmed);
            current_code.push('\n');
        }
    }

    tests
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_doc_test_creation() {
        let test = DocTest::new(
            "test_add".to_string(),
            "assert_eq!(1 + 1, 2);".to_string(),
            10,
        );

        assert_eq!(test.name, "test_add");
        assert_eq!(test.code, "assert_eq!(1 + 1, 2);");
        assert_eq!(test.line, 10);
        assert!(!test.should_panic);
        assert!(!test.ignore);
    }

    #[test]
    fn test_doc_test_with_attributes() {
        let test = DocTest::new("test".to_string(), "code".to_string(), 0)
            .with_should_panic()
            .with_ignore();

        assert!(test.should_panic);
        assert!(test.ignore);
    }

    #[test]
    fn test_registry() {
        let mut registry = DocTestRegistry::new();

        let test1 = DocTest::new("test1".to_string(), "code1".to_string(), 0);
        let test2 = DocTest::new("test2".to_string(), "code2".to_string(), 5);

        registry.register("module_a", test1);
        registry.register("module_a", test2.clone());
        registry.register("module_b", test2);

        assert_eq!(registry.total_tests(), 3);
        assert_eq!(registry.modules().len(), 2);
        assert_eq!(registry.get_tests("module_a").unwrap().len(), 2);
        assert_eq!(registry.get_tests("module_b").unwrap().len(), 1);
    }

    #[test]
    fn test_extract_simple_doc_test() {
        let doc = r#"
/// This function adds two numbers.
///
/// # Example
/// ```
/// let result = add(2, 3);
/// assert_eq!(result, 5);
/// ```
"#;

        let tests = extract_doc_tests("test_module", doc);
        assert_eq!(tests.len(), 1);
        assert!(tests[0].code.contains("let result = add(2, 3);"));
        assert!(tests[0].code.contains("assert_eq!(result, 5);"));
    }

    #[test]
    fn test_extract_multiple_doc_tests() {
        let doc = r#"
/// Function with multiple examples
///
/// # Example 1
/// ```
/// assert_eq!(1 + 1, 2);
/// ```
///
/// # Example 2
/// ```
/// assert_eq!(2 + 2, 4);
/// ```
"#;

        let tests = extract_doc_tests("test_module", doc);
        assert_eq!(tests.len(), 2);
    }

    #[test]
    fn test_extract_should_panic() {
        let doc = r#"
/// This should panic
///
/// ```should_panic
/// panic!("expected");
/// ```
"#;

        let tests = extract_doc_tests("test_module", doc);
        assert_eq!(tests.len(), 1);
        assert!(tests[0].should_panic);
    }

    #[test]
    fn test_extract_ignore() {
        let doc = r#"
/// This is ignored
///
/// ```ignore
/// expensive_test();
/// ```
"#;

        let tests = extract_doc_tests("test_module", doc);
        assert_eq!(tests.len(), 1);
        assert!(tests[0].ignore);
    }

    #[test]
    fn test_extract_no_tests() {
        let doc = r#"
/// Just documentation, no code blocks
"#;

        let tests = extract_doc_tests("test_module", doc);
        assert_eq!(tests.len(), 0);
    }
}

