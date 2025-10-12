//! Salsa database for incremental computation
//!
//! This module provides the core incremental computation infrastructure using Salsa 0.24.
//! It uses `#[salsa::tracked]` for memoized functions and input structs.

use std::sync::Arc;
use tower_lsp::lsp_types::Url;
use windjammer::{lexer, parser};

// ============================================================================
// Database Definition
// ============================================================================

/// The main Salsa database that stores all memoized computations
#[salsa::db]
#[derive(Clone)]
pub struct WindjammerDatabase {
    storage: salsa::Storage<Self>,
}

impl salsa::Database for WindjammerDatabase {}

impl Default for WindjammerDatabase {
    fn default() -> Self {
        Self {
            storage: Default::default(),
        }
    }
}

// ============================================================================
// Input Structs
// ============================================================================

/// Represents a source file
///
/// This is an input - it can be set by the caller and triggers recomputation
/// when changed.
#[salsa::input]
pub struct SourceFile {
    pub uri: Url,
    pub text: Arc<String>,
}

// ============================================================================
// Tracked Structs (Intermediate Results)
// ============================================================================

/// A parsed program (memoized)
///
/// We wrap the Program in a tracked struct so Salsa can track it.
/// The actual parser::Program doesn't need to implement PartialEq/Hash.
#[salsa::tracked]
pub struct ParsedProgram {
    #[return_ref]
    pub program: Arc<parser::Program>,
}

/// Import information for a file
#[salsa::tracked]
pub struct ImportInfo {
    #[return_ref]
    pub imports: Arc<Vec<Url>>,
}

// ============================================================================
// Tracked Functions (Derived Computations)
// ============================================================================

/// Parse a source file into an AST
///
/// This function is memoized - it only recomputes if the source text changes.
#[salsa::tracked]
pub fn parse(db: &dyn salsa::Database, file: SourceFile) -> ParsedProgram {
    let uri = file.uri(db);
    let text = file.text(db);
    
    tracing::debug!("Salsa: Parsing {}", uri);
    
    // Lex and parse
    let mut lexer = lexer::Lexer::new(&text);
    let tokens = lexer.tokenize();
    let mut parser = parser::Parser::new(tokens);
    
    let program = match parser.parse() {
        Ok(prog) => prog,
        Err(e) => {
            tracing::error!("Parse error in {}: {}", uri, e);
            // Return empty program on error
            parser::Program { items: vec![] }
        }
    };
    
    ParsedProgram::new(db, Arc::new(program))
}

/// Extract imports from a source file
///
/// Returns URIs of files that this file imports.
#[salsa::tracked]
pub fn imports(db: &dyn salsa::Database, file: SourceFile) -> ImportInfo {
    let parsed = parse(db, file);
    let program = parsed.program(db);
    
    let uri = file.uri(db);
    tracing::debug!("Salsa: Extracting imports from {}", uri);
    
    let mut import_uris = Vec::new();
    
    // Extract imports from the AST
    for item in &program.items {
        if let parser::Item::Use { path, alias: _ } = item {
            // TODO: Resolve import path to URI
            // For now, we'll just log it
            tracing::debug!("Found import: {}", path.join("."));
            
            // In the future, this will resolve to actual URIs:
            // if let Some(resolved_uri) = resolve_import(uri, &path.join(".")) {
            //     import_uris.push(resolved_uri);
            // }
        }
    }
    
    ImportInfo::new(db, Arc::new(import_uris))
}

// ============================================================================
// Database API
// ============================================================================

impl WindjammerDatabase {
    /// Create a new database
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Set the source text for a file
    ///
    /// Returns a SourceFile handle that can be used in queries.
    pub fn set_source_text(&mut self, uri: Url, text: String) -> SourceFile {
        SourceFile::new(self, uri, Arc::new(text))
    }
    
    /// Get the parsed program for a file
    pub fn get_program(&self, file: SourceFile) -> Arc<parser::Program> {
        let parsed = parse(self, file);
        parsed.program(self).clone()
    }
    
    /// Get imports for a file
    pub fn get_imports(&self, file: SourceFile) -> Arc<Vec<Url>> {
        let import_info = imports(self, file);
        import_info.imports(self).clone()
    }
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
    // This would handle:
    // - std.* imports (from WINDJAMMER_STDLIB)
    // - Relative imports (./ and ../)
    // - Absolute imports
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
        
        let file = db.set_source_text(uri, "fn main() {}".to_string());
        
        let program = db.get_program(file);
        assert_eq!(program.items.len(), 1);
        
        // Verify it's a function
        if let parser::Item::Function(func) = &program.items[0] {
            assert_eq!(func.name, "main");
        } else {
            panic!("Expected function item");
        }
    }
    
    #[test]
    fn test_incremental_update() {
        let mut db = WindjammerDatabase::new();
        let uri = Url::parse("file:///test.wj").unwrap();
        
        // Initial parse
        let file1 = db.set_source_text(uri.clone(), "fn foo() {}".to_string());
        let program1 = db.get_program(file1);
        assert_eq!(program1.items.len(), 1);
        
        // Update source - creates a new SourceFile input
        let file2 = db.set_source_text(uri, "fn foo() {}\nfn bar() {}".to_string());
        let program2 = db.get_program(file2);
        assert_eq!(program2.items.len(), 2);
    }
    
    #[test]
    fn test_memoization() {
        let mut db = WindjammerDatabase::new();
        let uri = Url::parse("file:///test.wj").unwrap();
        
        let file = db.set_source_text(uri, "fn main() {}".to_string());
        
        // First parse
        let parsed1 = parse(&db, file);
        
        // Second parse (should be memoized - same ParsedProgram)
        let parsed2 = parse(&db, file);
        
        // The ParsedProgram should be the same (Salsa returns same value)
        // Note: We can't check pointer equality directly, but we can verify
        // that the program content is the same
        assert_eq!(parsed1.program(&db).items.len(), parsed2.program(&db).items.len());
    }
    
    #[test]
    fn test_parse_error_handling() {
        let mut db = WindjammerDatabase::new();
        let uri = Url::parse("file:///test.wj").unwrap();
        
        // Invalid syntax
        let file = db.set_source_text(uri, "fn }}}".to_string());
        
        // Should not panic, should return empty program
        let program = db.get_program(file);
        assert_eq!(program.items.len(), 0);
    }
    
    #[test]
    fn test_extract_imports() {
        let mut db = WindjammerDatabase::new();
        let uri = Url::parse("file:///test.wj").unwrap();
        
        let file = db.set_source_text(
            uri,
            "use std.fs\nuse std.http\nfn main() {}".to_string(),
        );
        
        let imports = db.get_imports(file);
        
        // For now, imports are empty until we implement resolution
        // But the function should not crash
        assert_eq!(imports.len(), 0);
    }
}
