/// Error Statistics: Track and analyze compilation error patterns
///
/// This module provides error tracking and statistics to help developers
/// understand common error patterns and improve their code quality.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Error statistics tracker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorStatistics {
    /// Total number of compilation attempts
    pub total_compilations: usize,
    /// Total number of errors encountered
    pub total_errors: usize,
    /// Error frequency by error code
    pub errors_by_code: HashMap<String, ErrorCodeStats>,
    /// Error frequency by file
    pub errors_by_file: HashMap<PathBuf, usize>,
    /// Error frequency by type (error, warning)
    pub errors_by_type: HashMap<String, usize>,
    /// Recent errors (last 100)
    pub recent_errors: Vec<ErrorRecord>,
    /// Statistics start time
    pub started_at: SystemTime,
    /// Last updated time
    pub updated_at: SystemTime,
}

/// Statistics for a specific error code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorCodeStats {
    /// Error code (e.g., "E0425", "WJ0001")
    pub code: String,
    /// Number of times this error occurred
    pub count: usize,
    /// Error message template
    pub message: String,
    /// First seen timestamp
    pub first_seen: SystemTime,
    /// Last seen timestamp
    pub last_seen: SystemTime,
    /// Files where this error occurred
    pub files: Vec<PathBuf>,
}

/// Record of a single error occurrence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorRecord {
    /// Error code
    pub code: Option<String>,
    /// Error message
    pub message: String,
    /// File where error occurred
    pub file: PathBuf,
    /// Line number
    pub line: usize,
    /// Timestamp
    pub timestamp: SystemTime,
    /// Was it fixed?
    pub fixed: bool,
}

impl ErrorStatistics {
    /// Create a new error statistics tracker
    pub fn new() -> Self {
        let now = SystemTime::now();
        Self {
            total_compilations: 0,
            total_errors: 0,
            errors_by_code: HashMap::new(),
            errors_by_file: HashMap::new(),
            errors_by_type: HashMap::new(),
            recent_errors: Vec::new(),
            started_at: now,
            updated_at: now,
        }
    }

    /// Load statistics from file
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let content = fs::read_to_string(path)?;
        let stats: ErrorStatistics = serde_json::from_str(&content)?;
        Ok(stats)
    }

    /// Save statistics to file
    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    /// Record a compilation attempt
    pub fn record_compilation(&mut self) {
        self.total_compilations += 1;
        self.updated_at = SystemTime::now();
    }

    /// Record an error
    pub fn record_error(
        &mut self,
        code: Option<String>,
        message: String,
        file: PathBuf,
        line: usize,
    ) {
        self.total_errors += 1;
        self.updated_at = SystemTime::now();

        // Update error by code
        if let Some(ref error_code) = code {
            let stats = self
                .errors_by_code
                .entry(error_code.clone())
                .or_insert_with(|| ErrorCodeStats {
                    code: error_code.clone(),
                    count: 0,
                    message: message.clone(),
                    first_seen: SystemTime::now(),
                    last_seen: SystemTime::now(),
                    files: Vec::new(),
                });

            stats.count += 1;
            stats.last_seen = SystemTime::now();
            if !stats.files.contains(&file) {
                stats.files.push(file.clone());
            }
        }

        // Update error by file
        *self.errors_by_file.entry(file.clone()).or_insert(0) += 1;

        // Update error by type
        let error_type = if code.is_some() { "error" } else { "warning" };
        *self.errors_by_type.entry(error_type.to_string()).or_insert(0) += 1;

        // Add to recent errors (keep last 100)
        self.recent_errors.push(ErrorRecord {
            code,
            message,
            file,
            line,
            timestamp: SystemTime::now(),
            fixed: false,
        });

        if self.recent_errors.len() > 100 {
            self.recent_errors.remove(0);
        }
    }

    /// Get the most common errors
    pub fn get_top_errors(&self, limit: usize) -> Vec<(&ErrorCodeStats, usize)> {
        let mut errors: Vec<_> = self
            .errors_by_code
            .values()
            .map(|stats| (stats, stats.count))
            .collect();

        errors.sort_by(|a, b| b.1.cmp(&a.1));
        errors.truncate(limit);
        errors
    }

    /// Get the most error-prone files
    pub fn get_top_files(&self, limit: usize) -> Vec<(&PathBuf, usize)> {
        let mut files: Vec<_> = self.errors_by_file.iter().map(|(k, v)| (k, *v)).collect();
        files.sort_by(|a, b| b.1.cmp(&a.1));
        files.truncate(limit);
        files
    }

    /// Get error rate (errors per compilation)
    pub fn get_error_rate(&self) -> f64 {
        if self.total_compilations == 0 {
            0.0
        } else {
            self.total_errors as f64 / self.total_compilations as f64
        }
    }

    /// Get statistics summary
    pub fn get_summary(&self) -> StatsSummary {
        let duration = self
            .updated_at
            .duration_since(self.started_at)
            .unwrap_or_default();

        StatsSummary {
            total_compilations: self.total_compilations,
            total_errors: self.total_errors,
            unique_error_codes: self.errors_by_code.len(),
            unique_files: self.errors_by_file.len(),
            error_rate: self.get_error_rate(),
            duration_days: duration.as_secs() / 86400,
            most_common_error: self
                .get_top_errors(1)
                .first()
                .map(|(stats, _)| stats.code.clone()),
        }
    }

    /// Format statistics as a human-readable string
    pub fn format(&self) -> String {
        use colored::*;

        let mut output = String::new();

        output.push_str(&format!(
            "\n{}\n",
            "📊 Windjammer Error Statistics".cyan().bold()
        ));
        output.push_str(&format!("{}\n\n", "=".repeat(50)));

        // Summary
        let summary = self.get_summary();
        output.push_str(&format!("{}\n", "Summary:".yellow().bold()));
        output.push_str(&format!(
            "  Total Compilations: {}\n",
            summary.total_compilations
        ));
        output.push_str(&format!("  Total Errors: {}\n", summary.total_errors));
        output.push_str(&format!(
            "  Error Rate: {:.2} errors/compilation\n",
            summary.error_rate
        ));
        output.push_str(&format!(
            "  Tracking Period: {} days\n\n",
            summary.duration_days
        ));

        // Top errors
        output.push_str(&format!("{}\n", "Top 5 Most Common Errors:".yellow().bold()));
        let top_errors = self.get_top_errors(5);
        if top_errors.is_empty() {
            output.push_str("  No errors recorded yet\n");
        } else {
            for (i, (stats, count)) in top_errors.iter().enumerate() {
                output.push_str(&format!(
                    "  {}. {} - {} occurrences\n",
                    i + 1,
                    stats.code.green(),
                    count
                ));
                output.push_str(&format!("     {}\n", stats.message.dimmed()));
            }
        }
        output.push_str("\n");

        // Top files
        output.push_str(&format!(
            "{}\n",
            "Top 5 Most Error-Prone Files:".yellow().bold()
        ));
        let top_files = self.get_top_files(5);
        if top_files.is_empty() {
            output.push_str("  No files recorded yet\n");
        } else {
            for (i, (file, count)) in top_files.iter().enumerate() {
                output.push_str(&format!(
                    "  {}. {} - {} errors\n",
                    i + 1,
                    file.display(),
                    count
                ));
            }
        }
        output.push_str("\n");

        // Recent errors
        output.push_str(&format!(
            "{}\n",
            "Recent Errors (last 5):".yellow().bold()
        ));
        let recent = self.recent_errors.iter().rev().take(5).collect::<Vec<_>>();
        if recent.is_empty() {
            output.push_str("  No recent errors\n");
        } else {
            for (i, record) in recent.iter().enumerate() {
                let code_str = record
                    .code
                    .as_ref()
                    .map(|c| format!("[{}]", c))
                    .unwrap_or_default();
                output.push_str(&format!(
                    "  {}. {} {} at {}:{}\n",
                    i + 1,
                    code_str.green(),
                    record.message,
                    record.file.display(),
                    record.line
                ));
            }
        }

        output
    }

    /// Clear all statistics
    pub fn clear(&mut self) {
        *self = Self::new();
    }
}

impl Default for ErrorStatistics {
    fn default() -> Self {
        Self::new()
    }
}

/// Summary statistics
#[derive(Debug, Clone)]
pub struct StatsSummary {
    pub total_compilations: usize,
    pub total_errors: usize,
    pub unique_error_codes: usize,
    pub unique_files: usize,
    pub error_rate: f64,
    pub duration_days: u64,
    pub most_common_error: Option<String>,
}

/// Get the default statistics file path
pub fn get_stats_file_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".windjammer").join("stats.json")
}

/// Load or create statistics
pub fn load_or_create_stats() -> ErrorStatistics {
    let path = get_stats_file_path();

    // Ensure directory exists
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    ErrorStatistics::load(&path).unwrap_or_else(|_| ErrorStatistics::new())
}

/// Save statistics
pub fn save_stats(stats: &ErrorStatistics) -> anyhow::Result<()> {
    let path = get_stats_file_path();

    // Ensure directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    stats.save(&path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_statistics_creation() {
        let stats = ErrorStatistics::new();
        assert_eq!(stats.total_compilations, 0);
        assert_eq!(stats.total_errors, 0);
    }

    #[test]
    fn test_record_compilation() {
        let mut stats = ErrorStatistics::new();
        stats.record_compilation();
        assert_eq!(stats.total_compilations, 1);
    }

    #[test]
    fn test_record_error() {
        let mut stats = ErrorStatistics::new();
        stats.record_error(
            Some("E0425".to_string()),
            "Variable not found".to_string(),
            PathBuf::from("test.wj"),
            10,
        );

        assert_eq!(stats.total_errors, 1);
        assert_eq!(stats.errors_by_code.len(), 1);
        assert!(stats.errors_by_code.contains_key("E0425"));
    }

    #[test]
    fn test_top_errors() {
        let mut stats = ErrorStatistics::new();

        // Record multiple errors
        for _ in 0..5 {
            stats.record_error(
                Some("E0425".to_string()),
                "Variable not found".to_string(),
                PathBuf::from("test.wj"),
                10,
            );
        }

        for _ in 0..3 {
            stats.record_error(
                Some("E0308".to_string()),
                "Type mismatch".to_string(),
                PathBuf::from("test.wj"),
                20,
            );
        }

        let top = stats.get_top_errors(2);
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].0.code, "E0425");
        assert_eq!(top[0].1, 5);
    }

    #[test]
    fn test_error_rate() {
        let mut stats = ErrorStatistics::new();
        stats.record_compilation();
        stats.record_compilation();
        stats.record_error(
            Some("E0425".to_string()),
            "Variable not found".to_string(),
            PathBuf::from("test.wj"),
            10,
        );

        assert_eq!(stats.get_error_rate(), 0.5); // 1 error / 2 compilations
    }

    #[test]
    fn test_recent_errors_limit() {
        let mut stats = ErrorStatistics::new();

        // Record 150 errors (should keep only last 100)
        for i in 0..150 {
            stats.record_error(
                Some(format!("E{:04}", i)),
                "Test error".to_string(),
                PathBuf::from("test.wj"),
                i,
            );
        }

        assert_eq!(stats.recent_errors.len(), 100);
    }
}

