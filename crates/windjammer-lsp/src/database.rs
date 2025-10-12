//! Salsa database for incremental computation
//!
//! This module provides the core incremental computation infrastructure using Salsa.
//! It tracks source files, parses them incrementally, and manages dependencies between files.

use salsa::Database;
use std::sync::Arc;
use tower_lsp::lsp_types::{Diagnostic, Url};
use windjammer::parser::Program;

// ============================================================================
// Database Definition
// ============================================================================

/// The main Salsa database that stores all memoized computations
#[salsa::database(LanguageStorage)]
#[derive(Default)]
pub struct WindjammerDatabase {
    storage: salsa::Storage<Self>,
}

impl salsa::Database for WindjammerDatabase {}

// ============================================================================
// Language Queries
// ============================================================================

/// Core language analysis queries
#[salsa::query_group(LanguageStorage)]
pub trait LanguageQueries: salsa::Database {
    /// Input: The source text for a file
    ///
    /// This is the root of the dependency graph. When source text changes,
    /// all dependent queries are invalidated.
    #[salsa::input]
    fn source_text(&self, uri: Url) -> Arc<String>;

    /// Derived: Parse the source text into an AST
    ///
    /// This query is memoized - it only recomputes if source_text changes.
    fn parse(&self, uri: Url) -> Arc<Program>;

    /// Derived: Extract imports from a file
    ///
    /// Returns list of URIs that this file imports (direct dependencies).
    fn imports(&self, uri: Url) -> Arc<Vec<Url>>;

    /// Derived: Get all files in the workspace
    ///
    /// This is used for cross-file analysis.
    fn all_files(&self) -> Arc<Vec<Url>>;

    /// Derived: Find all files that depend on this file
    ///
    /// Returns URIs of files that import this file (directly or transitively).
    fn dependents(&self, uri: Url) -> Arc<Vec<Url>>;
}

// ============================================================================
// Query Implementations
// ============================================================================

/// Parse query: Convert source text to AST
fn parse(db: &dyn LanguageQueries, uri: Url) -> Arc<Program> {
    tracing::debug!("Salsa: Parsing {}", uri);
    
    let source = db.source_text(uri);
    let program = windjammer::parser::parse(&source);
    
    Arc::new(program)
}

/// Imports query: Extract import statements
fn imports(db: &dyn LanguageQueries, uri: Url) -> Arc<Vec<Url>> {
    tracing::debug!("Salsa: Extracting imports from {}", uri);
    
    let program = db.parse(uri.clone());
    let mut import_uris = Vec::new();
    
    // TODO: Extract actual imports from the AST
    // For now, return empty list until we have import syntax
    // In the future, this will look like:
    //
    // for item in &program.items {
    //     if let windjammer::parser::Item::Import(import) = item {
    //         if let Some(resolved_uri) = resolve_import(&uri, &import.path) {
    //             import_uris.push(resolved_uri);
    //         }
    //     }
    // }
    
    Arc::new(import_uris)
}

/// All files query: Return all known files
///
/// Note: This is a bit tricky with Salsa - we need to track which files
/// have been added. For now, we'll use an approach where files are registered
/// when their source_text is first set.
fn all_files(_db: &dyn LanguageQueries) -> Arc<Vec<Url>> {
    // TODO: Implement proper file tracking
    // This will require either:
    // 1. A separate registry of files (outside Salsa)
    // 2. Using Salsa's intern system for file IDs
    // 3. Tracking files when source_text is set
    
    tracing::debug!("Salsa: Getting all files (stub)");
    Arc::new(Vec::new())
}

/// Dependents query: Find files that import this file
fn dependents(db: &dyn LanguageQueries, uri: Url) -> Arc<Vec<Url>> {
    tracing::debug!("Salsa: Finding dependents of {}", uri);
    
    let mut dependent_files = Vec::new();
    let all_files = db.all_files();
    
    // Check each file to see if it imports this file
    for file_uri in all_files.iter() {
        let imports = db.imports(file_uri.clone());
        if imports.contains(&uri) {
            dependent_files.push(file_uri.clone());
            
            // Recursively add transitive dependents
            let transitive = db.dependents(file_uri.clone());
            dependent_files.extend(transitive.iter().cloned());
        }
    }
    
    // Remove duplicates
    dependent_files.sort();
    dependent_files.dedup();
    
    Arc::new(dependent_files)
}

// ============================================================================
// Database Utilities
// ============================================================================

impl WindjammerDatabase {
    /// Create a new database
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Set the source text for a file
    ///
    /// This invalidates all queries that depend on this file.
    pub fn set_source(&mut self, uri: Url, text: String) {
        self.set_source_text(uri, Arc::new(text));
    }
    
    /// Get the parse tree for a file
    pub fn get_program(&self, uri: &Url) -> Option<Arc<Program>> {
        // Check if source exists for this URI
        // Note: Salsa doesn't provide a way to check if an input exists,
        // so we catch the panic and return None
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.parse(uri.clone())
        })) {
            Ok(program) => Some(program),
            Err(_) => None,
        }
    }
    
    /// Run garbage collection
    ///
    /// This removes memoized data for queries that are no longer needed.
    pub fn gc(&mut self) {
        tracing::debug!("Running Salsa GC");
        self.sweep_all(salsa::SweepStrategy::default());
    }
    
    /// Get memory usage statistics
    pub fn memory_stats(&self) -> MemoryStats {
        // TODO: Implement actual memory tracking
        MemoryStats {
            total_bytes: 0,
            cached_programs: 0,
        }
    }
}

/// Memory usage statistics
#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub total_bytes: usize,
    pub cached_programs: usize,
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Resolve an import path to a URI
///
/// This will eventually handle relative imports, absolute imports, etc.
#[allow(dead_code)]
fn resolve_import(base_uri: &Url, import_path: &str) -> Option<Url> {
    // TODO: Implement proper import resolution
    // For now, just return None
    tracing::debug!("Resolving import '{}' from {}", import_path, base_uri);
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_basic_parse() {
        let mut db = WindjammerDatabase::new();
        let uri = Url::parse("file:///test.wj").unwrap();
        
        db.set_source(uri.clone(), "fn main() {}".to_string());
        
        let program = db.get_program(&uri).expect("Program should exist");
        assert_eq!(program.items.len(), 1);
    }
    
    #[test]
    fn test_incremental_update() {
        let mut db = WindjammerDatabase::new();
        let uri = Url::parse("file:///test.wj").unwrap();
        
        // Initial parse
        db.set_source(uri.clone(), "fn foo() {}".to_string());
        let program1 = db.get_program(&uri).unwrap();
        assert_eq!(program1.items.len(), 1);
        
        // Update source
        db.set_source(uri.clone(), "fn foo() {}\nfn bar() {}".to_string());
        let program2 = db.get_program(&uri).unwrap();
        assert_eq!(program2.items.len(), 2);
    }
    
    #[test]
    fn test_memoization() {
        let mut db = WindjammerDatabase::new();
        let uri = Url::parse("file:///test.wj").unwrap();
        
        db.set_source(uri.clone(), "fn main() {}".to_string());
        
        // First parse
        let program1 = db.parse(uri.clone());
        
        // Second parse (should be memoized)
        let program2 = db.parse(uri.clone());
        
        // Should be the same Arc (pointer equality)
        assert!(Arc::ptr_eq(&program1, &program2));
    }
}

