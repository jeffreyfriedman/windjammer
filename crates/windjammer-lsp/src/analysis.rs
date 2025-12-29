use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::RwLock;
use std::time::SystemTime;
use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range, Url};
use windjammer::{
    analyzer::{AnalyzedFunction, Analyzer},
    lexer::Lexer,
    parser::{Parser, Program},
};

use crate::symbol_table::SymbolTable;

/// Analysis database for incremental compilation
///
/// This will eventually use Salsa for query-based incremental compilation,
/// but for now we'll use a simple approach to get the LSP working.
pub struct AnalysisDatabase {
    /// Cache of parsed files
    cache: RwLock<HashMap<Url, FileAnalysis>>,
}

/// Analysis results for a single file
#[derive(Clone)]
#[allow(dead_code)] // Some fields reserved for future incremental parsing features
struct FileAnalysis {
    /// Hash of source code for change detection
    source_hash: u64,
    /// Source code (kept for future use in incremental parsing)
    source: String,
    /// Parsed AST
    program: Option<Program<'static>>,
    /// Analyzed functions with ownership inference
    analyzed_functions: Vec<AnalyzedFunction<'static>>,
    /// Symbol table for go-to-definition
    symbol_table: SymbolTable,
    /// Analysis diagnostics
    diagnostics: Vec<Diagnostic>,
    /// Timestamp of last analysis (for cache invalidation)
    timestamp: SystemTime,
}

impl AnalysisDatabase {
    pub fn new() -> Self {
        Self {
            cache: RwLock::new(HashMap::new()),
        }
    }

    /// Analyze a file and return diagnostics
    ///
    /// Uses incremental compilation: skips re-analysis if content hasn't changed
    pub fn analyze_file(&self, uri: &Url, content: &str) -> Vec<Diagnostic> {
        // Calculate hash of new content
        let new_hash = Self::calculate_hash(content);

        // Check if we have cached analysis with the same hash
        {
            let cache = self.cache.read().unwrap();
            if let Some(cached) = cache.get(uri) {
                if cached.source_hash == new_hash {
                    tracing::debug!("File {} unchanged (hash match), using cached analysis", uri);
                    return cached.diagnostics.clone();
                }
            }
        }

        tracing::debug!("File {} changed or first analysis, re-analyzing", uri);

        let (diagnostics, program, analyzed_functions) = self.full_analysis(content);

        // Build symbol table
        let mut symbol_table = SymbolTable::new();
        if let Some(ref prog) = program {
            symbol_table.build_from_program(prog, uri);
            // Build references from source code
            symbol_table.build_references_from_source(content, uri);
        }

        // Cache the results
        let analysis = FileAnalysis {
            source_hash: new_hash,
            source: content.to_string(),
            program,
            analyzed_functions,
            symbol_table,
            diagnostics: diagnostics.clone(),
            timestamp: SystemTime::now(),
        };

        self.cache.write().unwrap().insert(uri.clone(), analysis);

        diagnostics
    }

    /// Calculate hash of source code for change detection
    fn calculate_hash(content: &str) -> u64 {
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        hasher.finish()
    }

    /// Full analysis: lex, parse, and analyze with ownership inference
    fn full_analysis(
        &self,
        content: &str,
    ) -> (Vec<Diagnostic>, Option<Program>, Vec<AnalyzedFunction>) {
        let mut diagnostics = Vec::new();
        let mut program_result = None;
        let mut analyzed_functions = Vec::new();

        // Lex the file
        let mut lexer = Lexer::new(content);
        let tokens = lexer.tokenize_with_locations();

        // Parse the file
        let mut parser = Parser::new(tokens);
        match parser.parse() {
            Ok(program) => {
                tracing::debug!("File parsed successfully");

                // Run ownership inference analysis
                let mut analyzer = Analyzer::new();
                match analyzer.analyze_program(&program) {
                    Ok((functions, _registry, _analyzed_trait_methods)) => {
                        tracing::debug!(
                            "Ownership analysis complete: {} functions",
                            functions.len()
                        );
                        analyzed_functions = functions;
                    }
                    Err(error) => {
                        tracing::warn!("Ownership analysis error: {}", error);
                        // Don't fail completely, just log the error
                    }
                }

                program_result = Some(program);
            }
            Err(error) => {
                // Parse error - convert to diagnostic
                tracing::debug!("Parse error: {}", error);

                // For now, create a diagnostic for the whole file
                // TODO: Extract line/column info from error message
                diagnostics.push(Diagnostic {
                    range: Range {
                        start: Position {
                            line: 0,
                            character: 0,
                        },
                        end: Position {
                            line: 0,
                            character: 100,
                        },
                    },
                    severity: Some(DiagnosticSeverity::ERROR),
                    code: None,
                    code_description: None,
                    source: Some("windjammer".to_string()),
                    message: format!("Parse error: {}", error),
                    related_information: None,
                    tags: None,
                    data: None,
                });
            }
        }

        (diagnostics, program_result, analyzed_functions)
    }

    /// Get cached analysis for a file (planned for hover improvements)
    #[allow(dead_code)]
    pub fn get_analysis(&self, uri: &Url) -> Option<Vec<Diagnostic>> {
        self.cache
            .read()
            .unwrap()
            .get(uri)
            .map(|analysis| analysis.diagnostics.clone())
    }

    /// Get cached program for a file
    pub fn get_program(&self, uri: &Url) -> Option<Program> {
        self.cache
            .read()
            .unwrap()
            .get(uri)
            .and_then(|analysis| analysis.program.clone())
    }

    /// Get cached analyzed functions for a file
    pub fn get_analyzed_functions(&self, uri: &Url) -> Vec<AnalyzedFunction> {
        self.cache
            .read()
            .unwrap()
            .get(uri)
            .map(|analysis| analysis.analyzed_functions.clone())
            .unwrap_or_default()
    }

    /// Get cached symbol table for a file
    pub fn get_symbol_table(&self, uri: &Url) -> Option<SymbolTable> {
        self.cache
            .read()
            .unwrap()
            .get(uri)
            .map(|analysis| analysis.symbol_table.clone())
    }
}

impl Default for AnalysisDatabase {
    fn default() -> Self {
        Self::new()
    }
}

// Incremental Compilation Status
//
// ✅ IMPLEMENTED (v0.19.0):
// - Hash-based change detection (skips re-analysis if unchanged)
// - Timestamp tracking for analysis freshness
// - Efficient caching with RwLock for concurrent access
// - Fine-grained invalidation per file
//
// Future enhancements (Salsa integration):
// - Query-based incremental computation
// - Cross-file dependency tracking
// - Fine-grained invalidation (per-function, per-symbol)
// - Parallel analysis of multiple files
//
// Current approach is simple and effective for LSP use:
// - Fast: O(1) cache lookup with hash comparison
// - Correct: Re-analyzes only when content changes
// - Scalable: Works well for typical project sizes

// Example Salsa database structure (for future reference):
/*
#[salsa::query_group(AnalysisStorage)]
pub trait AnalysisDatabase: salsa::Database {
    #[salsa::input]
    fn file_text(&self, file_id: FileId) -> Arc<String>;

    fn parse_file(&self, file_id: FileId) -> Arc<AST>;

    fn analyze_file(&self, file_id: FileId) -> Arc<AnalysisResult>;

    fn infer_ownership(&self, file_id: FileId, fn_id: FunctionId) -> Arc<OwnershipMap>;
}
*/
