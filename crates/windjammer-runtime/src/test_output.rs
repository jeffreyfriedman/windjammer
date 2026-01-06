//! Enhanced test output utilities
//!
//! Provides better formatting and diagnostics for test results.

use std::fmt;
use std::time::Duration;

/// Test result status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestStatus {
    Passed,
    Failed,
    Ignored,
}

/// Individual test result
#[derive(Debug, Clone)]
pub struct TestResult {
    pub name: String,
    pub status: TestStatus,
    pub duration: Duration,
    pub error: Option<String>,
}

impl TestResult {
    pub fn new(name: String, status: TestStatus, duration: Duration) -> Self {
        Self {
            name,
            status,
            duration,
            error: None,
        }
    }

    pub fn with_error(mut self, error: String) -> Self {
        self.error = Some(error);
        self
    }
}

/// Test suite summary
#[derive(Debug, Clone)]
pub struct TestSummary {
    pub results: Vec<TestResult>,
    pub total_duration: Duration,
}

impl TestSummary {
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
            total_duration: Duration::ZERO,
        }
    }

    pub fn add_result(&mut self, result: TestResult) {
        self.total_duration += result.duration;
        self.results.push(result);
    }

    pub fn passed_count(&self) -> usize {
        self.results
            .iter()
            .filter(|r| r.status == TestStatus::Passed)
            .count()
    }

    pub fn failed_count(&self) -> usize {
        self.results
            .iter()
            .filter(|r| r.status == TestStatus::Failed)
            .count()
    }

    pub fn ignored_count(&self) -> usize {
        self.results
            .iter()
            .filter(|r| r.status == TestStatus::Ignored)
            .count()
    }

    pub fn total_count(&self) -> usize {
        self.results.len()
    }

    /// Format test results in standard format
    pub fn format_standard(&self) -> String {
        let mut output = String::new();
        output.push_str(&format!("Running {} tests...\n\n", self.total_count()));

        for result in &self.results {
            let symbol = match result.status {
                TestStatus::Passed => "✓",
                TestStatus::Failed => "✗",
                TestStatus::Ignored => "⊘",
            };

            output.push_str(&format!(
                "{} {} ({:?})\n",
                symbol, result.name, result.duration
            ));

            if let Some(error) = &result.error {
                output.push_str(&format!("  {}\n\n", error));
            }
        }

        output.push_str("\nTest Results:\n");
        output.push_str(&format!(
            "  Passed: {}/{} ({:.1}%)\n",
            self.passed_count(),
            self.total_count(),
            (self.passed_count() as f64 / self.total_count() as f64) * 100.0
        ));
        output.push_str(&format!("  Failed: {}\n", self.failed_count()));
        output.push_str(&format!("  Ignored: {}\n", self.ignored_count()));
        output.push_str(&format!("  Total time: {:?}\n", self.total_duration));

        output
    }

    /// Format test results in verbose format
    pub fn format_verbose(&self) -> String {
        let mut output = String::new();
        output.push_str(&format!("Running {} tests...\n\n", self.total_count()));

        for result in &self.results {
            let status = match result.status {
                TestStatus::Passed => "[PASS]",
                TestStatus::Failed => "[FAIL]",
                TestStatus::Ignored => "[SKIP]",
            };

            output.push_str(&format!(
                "{} {} ({:?})\n",
                status, result.name, result.duration
            ));

            if let Some(error) = &result.error {
                output.push_str(&format!("  ↳ {}\n", error));
            }

            output.push('\n');
        }

        output.push_str("────────────────────────────────────────\n");
        output.push_str(&format!("Total: {} tests\n", self.total_count()));
        output.push_str(&format!(
            "Passed: {} ({:.1}%)\n",
            self.passed_count(),
            (self.passed_count() as f64 / self.total_count() as f64) * 100.0
        ));
        output.push_str(&format!("Failed: {}\n", self.failed_count()));
        output.push_str(&format!("Ignored: {}\n", self.ignored_count()));
        output.push_str(&format!("Duration: {:?}\n", self.total_duration));

        output
    }
}

impl Default for TestSummary {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for TestSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.format_standard())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_result_creation() {
        let result = TestResult::new(
            "test_foo".to_string(),
            TestStatus::Passed,
            Duration::from_millis(10),
        );

        assert_eq!(result.name, "test_foo");
        assert_eq!(result.status, TestStatus::Passed);
        assert_eq!(result.duration, Duration::from_millis(10));
        assert!(result.error.is_none());
    }

    #[test]
    fn test_result_with_error() {
        let result = TestResult::new(
            "test_foo".to_string(),
            TestStatus::Failed,
            Duration::from_millis(10),
        )
        .with_error("assertion failed".to_string());

        assert!(result.error.is_some());
        assert_eq!(result.error.unwrap(), "assertion failed");
    }

    #[test]
    fn test_summary_counts() {
        let mut summary = TestSummary::new();

        summary.add_result(TestResult::new(
            "test1".to_string(),
            TestStatus::Passed,
            Duration::from_millis(5),
        ));
        summary.add_result(TestResult::new(
            "test2".to_string(),
            TestStatus::Failed,
            Duration::from_millis(10),
        ));
        summary.add_result(TestResult::new(
            "test3".to_string(),
            TestStatus::Ignored,
            Duration::from_millis(0),
        ));

        assert_eq!(summary.total_count(), 3);
        assert_eq!(summary.passed_count(), 1);
        assert_eq!(summary.failed_count(), 1);
        assert_eq!(summary.ignored_count(), 1);
        assert_eq!(summary.total_duration, Duration::from_millis(15));
    }

    #[test]
    fn test_format_standard() {
        let mut summary = TestSummary::new();
        summary.add_result(TestResult::new(
            "test_foo".to_string(),
            TestStatus::Passed,
            Duration::from_millis(5),
        ));
        summary.add_result(
            TestResult::new(
                "test_bar".to_string(),
                TestStatus::Failed,
                Duration::from_millis(10),
            )
            .with_error("assertion failed".to_string()),
        );

        let output = summary.format_standard();
        assert!(output.contains("✓ test_foo"));
        assert!(output.contains("✗ test_bar"));
        assert!(output.contains("assertion failed"));
        assert!(output.contains("Passed: 1/2"));
        assert!(output.contains("Failed: 1"));
    }

    #[test]
    fn test_format_verbose() {
        let mut summary = TestSummary::new();
        summary.add_result(TestResult::new(
            "test_foo".to_string(),
            TestStatus::Passed,
            Duration::from_millis(5),
        ));

        let output = summary.format_verbose();
        assert!(output.contains("[PASS] test_foo"));
        assert!(output.contains("Total: 1 tests"));
    }
}
