// Salsa-based incremental compilation database
use salsa::Database;
use std::sync::Arc;
use lsp_types::Url;

/// The main incremental compilation database
/// Uses Salsa to cache and recompute only what's needed
#[salsa::database(SourceDatabaseStorage, ParseDatabaseStorage, AnalysisDatabaseStorage)]
#[derive(Default)]
pub struct RootDatabase {
    storage: salsa::Storage<Self>,
}

impl salsa::Database for RootDatabase {}

/// Input queries - these are set by the LSP and trigger recomputation
#[salsa::query_group(SourceDatabaseStorage)]
pub trait SourceDatabase {
    /// Get the source text for a file
    #[salsa::input]
    fn source_text(&self, file: FileId) -> Arc<String>;
    
    /// Get all files in the workspace
    #[salsa::input]
    fn all_files(&self) -> Arc<Vec<FileId>>;
}

/// Parse queries - convert source to AST
#[salsa::query_group(ParseDatabaseStorage)]
pub trait ParseDatabase: SourceDatabase {
    /// Parse a file into tokens
    fn tokens(&self, file: FileId) -> Arc<Vec<windjammer::lexer::Token>>;
    
    /// Parse a file into an AST
    fn parse(&self, file: FileId) -> Arc<Result<windjammer::parser::Program, Vec<ParseError>>>;
    
    /// Get syntax errors for a file
    fn syntax_errors(&self, file: FileId) -> Arc<Vec<ParseError>>;
}

/// Analysis queries - semantic analysis, type checking, ownership inference
#[salsa::query_group(AnalysisDatabaseStorage)]
pub trait AnalysisDatabase: ParseDatabase {
    /// Analyze ownership for all functions in a file
    fn analyze_file(&self, file: FileId) -> Arc<Vec<windjammer::analyzer::AnalyzedFunction>>;
    
    /// Get semantic errors for a file
    fn semantic_errors(&self, file: FileId) -> Arc<Vec<SemanticError>>;
    
    /// Get all symbols in a file (for completion, go-to-definition)
    fn symbols(&self, file: FileId) -> Arc<Vec<Symbol>>;
    
    /// Infer type of an expression at a position
    fn type_at_position(&self, file: FileId, line: u32, col: u32) -> Option<Arc<String>>;
}

/// File identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileId(pub u32);

impl salsa::InternKey for FileId {
    fn from_intern_id(v: salsa::InternId) -> Self {
        FileId(v.as_u32())
    }

    fn as_intern_id(&self) -> salsa::InternId {
        salsa::InternId::from(self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
    pub line: u32,
    pub col: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticError {
    pub message: String,
    pub line: u32,
    pub col: u32,
}

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub line: u32,
    pub col: u32,
    pub doc: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SymbolKind {
    Function,
    Struct,
    Enum,
    Variable,
    Parameter,
    Field,
}

// Implement the query functions

fn tokens(db: &dyn ParseDatabase, file: FileId) -> Arc<Vec<windjammer::lexer::Token>> {
    let source = db.source_text(file);
    let mut lexer = windjammer::lexer::Lexer::new(&source);
    Arc::new(lexer.tokenize())
}

fn parse(db: &dyn ParseDatabase, file: FileId) -> Arc<Result<windjammer::parser::Program, Vec<ParseError>>> {
    let tokens = db.tokens(file);
    let mut parser = windjammer::parser::Parser::new((*tokens).clone());
    
    match parser.parse() {
        Ok(program) => Arc::new(Ok(program)),
        Err(e) => {
            let error = ParseError {
                message: e,
                line: 0,
                col: 0,
            };
            Arc::new(Err(vec![error]))
        }
    }
}

fn syntax_errors(db: &dyn ParseDatabase, file: FileId) -> Arc<Vec<ParseError>> {
    match &*db.parse(file) {
        Ok(_) => Arc::new(Vec::new()),
        Err(errors) => Arc::new(errors.clone()),
    }
}

fn analyze_file(db: &dyn AnalysisDatabase, file: FileId) -> Arc<Vec<windjammer::analyzer::AnalyzedFunction>> {
    match &*db.parse(file) {
        Ok(program) => {
            let mut analyzer = windjammer::analyzer::Analyzer::new();
            match analyzer.analyze_program(program) {
                Ok(analyzed) => Arc::new(analyzed),
                Err(_) => Arc::new(Vec::new()),
            }
        }
        Err(_) => Arc::new(Vec::new()),
    }
}

fn semantic_errors(db: &dyn AnalysisDatabase, file: FileId) -> Arc<Vec<SemanticError>> {
    // For now, just return empty
    // TODO: Implement semantic error collection
    Arc::new(Vec::new())
}

fn symbols(db: &dyn AnalysisDatabase, file: FileId) -> Arc<Vec<Symbol>> {
    match &*db.parse(file) {
        Ok(program) => {
            let mut symbols = Vec::new();
            
            for item in &program.items {
                match item {
                    windjammer::parser::Item::Function(func) => {
                        symbols.push(Symbol {
                            name: func.name.clone(),
                            kind: SymbolKind::Function,
                            line: 0, // TODO: Track line numbers in parser
                            col: 0,
                            doc: None,
                        });
                    }
                    windjammer::parser::Item::Struct(s) => {
                        symbols.push(Symbol {
                            name: s.name.clone(),
                            kind: SymbolKind::Struct,
                            line: 0,
                            col: 0,
                            doc: None,
                        });
                    }
                    windjammer::parser::Item::Enum(e) => {
                        symbols.push(Symbol {
                            name: e.name.clone(),
                            kind: SymbolKind::Enum,
                            line: 0,
                            col: 0,
                            doc: None,
                        });
                    }
                    _ => {}
                }
            }
            
            Arc::new(symbols)
        }
        Err(_) => Arc::new(Vec::new()),
    }
}

fn type_at_position(db: &dyn AnalysisDatabase, file: FileId, line: u32, col: u32) -> Option<Arc<String>> {
    // TODO: Implement type inference at position
    None
}

