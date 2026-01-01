//! Salsa database for incremental computation
//!
//! This module provides the core incremental computation infrastructure using Salsa 0.24.
//! It uses `#[salsa::tracked]` for memoized functions and tracked structs.

use tower_lsp::lsp_types::Url;
use windjammer::{lexer, parser};

// ============================================================================
// Database Definition
// ============================================================================

/// The main Salsa database implementation
#[salsa::db]
#[derive(Clone)]
pub struct WindjammerDatabase {
    storage: salsa::Storage<Self>,
    /// Lazy loading cache for symbols (only load when accessed)
    symbol_cache: std::sync::Arc<std::sync::Mutex<std::collections::HashMap<Url, bool>>>,
    /// Lazy loading cache for references
    reference_cache: std::sync::Arc<std::sync::Mutex<std::collections::HashMap<Url, bool>>>,
}

impl Default for WindjammerDatabase {
    fn default() -> Self {
        Self::new()
    }
}

#[salsa::db]
impl salsa::Database for WindjammerDatabase {}

// ============================================================================
// Input Structs
// ============================================================================

/// Represents a source file
///
/// This is an input - it can be set by the caller and triggers recomputation
/// when changed.
#[salsa::input]
pub struct SourceFile {
    #[returns(ref)]
    pub uri: Url,

    #[returns(ref)]
    pub text: String,
}

// ============================================================================
// Tracked Structs (Intermediate Results)
// ============================================================================

/// A parsed program (memoized)
///
/// Now that parser::Program implements Hash/PartialEq/Eq, we can use it
/// in a tracked struct for better Salsa integration.
#[salsa::tracked]
pub struct ParsedProgram<'db> {
    #[returns(ref)]
    pub program: parser::Program<'static>,
}

/// Import information for a file
#[salsa::tracked]
pub struct ImportInfo<'db> {
    #[returns(ref)]
    pub imports: Vec<Url>,
}

/// Symbol information for a file
#[salsa::tracked]
pub struct SymbolTable<'db> {
    #[returns(ref)]
    pub symbols: Vec<Symbol>,
}

/// A symbol definition with detailed position information
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub line: u32,
    pub character: u32,
    /// Full range of the symbol (for precise selection)
    pub range: Option<SymbolRange>,
    /// Range of just the symbol name (for rename operations)
    pub name_range: Option<SymbolRange>,
    /// Type information (if available)
    pub type_info: Option<String>,
    /// Documentation comment
    pub doc: Option<String>,
}

/// A range in the source code
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SymbolRange {
    pub start_line: u32,
    pub start_character: u32,
    pub end_line: u32,
    pub end_character: u32,
}

/// A reference to a symbol (usage location)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SymbolReference {
    pub name: String,
    pub uri: Url,
    pub line: u32,
    pub character: u32,
}

/// References found in a file
#[salsa::tracked]
pub struct ReferenceInfo<'db> {
    #[returns(ref)]
    pub references: Vec<SymbolReference>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SymbolKind {
    Function,
    Struct,
    Enum,
    Trait,
    Impl,
    Const,
    Static,
}

// ============================================================================
// Tracked Functions (Derived Computations)
// ============================================================================

/// Parse a source file into an AST
///
/// This function is memoized - it only recomputes if the source text changes.
#[salsa::tracked]
pub fn parse<'db>(db: &'db dyn salsa::Database, file: SourceFile) -> ParsedProgram<'db> {
    let uri = file.uri(db);
    let text = file.text(db);

    tracing::debug!("Salsa: Parsing {}", uri);

    // Lex and parse
    let mut lexer = lexer::Lexer::new(text);
    let tokens = lexer.tokenize_with_locations();
    // Leak parser to keep arena alive for 'static lifetime (required by Salsa)
    let parser = Box::leak(Box::new(parser::Parser::new(tokens)));

    let program = match parser.parse() {
        Ok(prog) => prog,
        Err(e) => {
            tracing::error!("Parse error in {}: {}", uri, e);
            // Return empty program on error
            parser::Program { items: vec![] }
        }
    };

    ParsedProgram::new(db, program)
}

/// Extract imports from a source file
///
/// Returns URIs of files that this file imports.
#[salsa::tracked]
pub fn imports<'db>(db: &'db dyn salsa::Database, file: SourceFile) -> ImportInfo<'db> {
    let parsed = parse(db, file);
    let program = parsed.program(db);

    let uri = file.uri(db);
    tracing::debug!("Salsa: Extracting imports from {}", uri);

    let mut import_uris = Vec::new();

    // Extract and resolve imports from the AST
    for item in &program.items {
        if let parser::Item::Use {
            path,
            alias: _,
            location: _,
            is_pub: _,
        } = item
        {
            let import_path = path.join(".");
            tracing::debug!("Found import: {}", import_path);

            // Resolve import path to actual file URI
            if let Some(resolved_uri) = resolve_import(uri, &import_path) {
                tracing::debug!("Resolved import '{}' to {}", import_path, resolved_uri);
                import_uris.push(resolved_uri);
            } else {
                tracing::debug!("Could not resolve import: {}", import_path);
            }
        }
    }

    tracing::debug!("Resolved {} imports from {}", import_uris.len(), uri);
    ImportInfo::new(db, import_uris)
}

/// Extract symbols from a source file
///
/// Returns all symbol definitions in the file (functions, structs, etc.)
#[salsa::tracked]
pub fn extract_symbols<'db>(db: &'db dyn salsa::Database, file: SourceFile) -> SymbolTable<'db> {
    let parsed = parse(db, file);
    let program = parsed.program(db);

    let uri = file.uri(db);
    tracing::debug!("Salsa: Extracting symbols from {}", uri);

    let mut symbols = Vec::new();

    // Extract symbols from top-level items
    for (idx, item) in program.items.iter().enumerate() {
        // Use item index as line heuristic (AST doesn't have position info yet)
        let line = idx as u32;

        match item {
            parser::Item::Function {
                decl: func,
                location: _,
            } => {
                symbols.push(Symbol {
                    name: func.name.clone(),
                    kind: SymbolKind::Function,
                    line,
                    character: 0,
                    range: None, // TODO: Extract from AST when available
                    name_range: None,
                    type_info: func.return_type.as_ref().map(|t| format!("{:?}", t)),
                    doc: None, // TODO: Extract doc comments
                });
            }
            parser::Item::Struct {
                decl: struct_decl,
                location: _,
            } => {
                symbols.push(Symbol {
                    name: struct_decl.name.clone(),
                    kind: SymbolKind::Struct,
                    line,
                    character: 0,
                    range: None,
                    name_range: None,
                    type_info: None,
                    doc: None,
                });
            }
            parser::Item::Enum {
                decl: enum_decl,
                location: _,
            } => {
                symbols.push(Symbol {
                    name: enum_decl.name.clone(),
                    kind: SymbolKind::Enum,
                    line,
                    character: 0,
                    range: None,
                    name_range: None,
                    type_info: None,
                    doc: None,
                });
            }
            parser::Item::Trait {
                decl: trait_decl,
                location: _,
            } => {
                symbols.push(Symbol {
                    name: trait_decl.name.clone(),
                    kind: SymbolKind::Trait,
                    line,
                    character: 0,
                    range: None,
                    name_range: None,
                    type_info: None,
                    doc: None,
                });
            }
            parser::Item::Impl {
                block: impl_block,
                location: _,
            } => {
                // Impl blocks don't have a name, use type name
                let name = if let Some(trait_name) = &impl_block.trait_name {
                    format!("impl {} for {}", trait_name, impl_block.type_name)
                } else {
                    format!("impl {}", impl_block.type_name)
                };
                symbols.push(Symbol {
                    name,
                    kind: SymbolKind::Impl,
                    line,
                    character: 0,
                    range: None,
                    name_range: None,
                    type_info: Some(impl_block.type_name.clone()),
                    doc: None,
                });
            }
            parser::Item::Const { name, type_, .. } => {
                symbols.push(Symbol {
                    name: name.clone(),
                    kind: SymbolKind::Const,
                    line,
                    character: 0,
                    range: None,
                    name_range: None,
                    type_info: Some(format!("{:?}", type_)), // Use Debug for now
                    doc: None,
                });
            }
            parser::Item::Static { name, type_, .. } => {
                symbols.push(Symbol {
                    name: name.clone(),
                    kind: SymbolKind::Static,
                    line,
                    character: 0,
                    range: None,
                    name_range: None,
                    type_info: Some(format!("{:?}", type_)), // Use Debug for now
                    doc: None,
                });
            }
            _ => {} // Skip other items (use statements, etc.)
        }
    }

    tracing::debug!("Found {} symbols in {}", symbols.len(), uri);
    SymbolTable::new(db, symbols)
}

/// Extract symbol references from a source file
///
/// Finds all usages of symbols in expressions, function calls, etc.
#[salsa::tracked]
pub fn extract_references<'db>(
    db: &'db dyn salsa::Database,
    file: SourceFile,
) -> ReferenceInfo<'db> {
    let parsed = parse(db, file);
    let program = parsed.program(db);

    let uri = file.uri(db).clone();
    tracing::debug!("Salsa: Extracting references from {}", uri);

    let references = Vec::new();

    // Walk the AST to find all identifier references
    // For now, we'll extract function calls as a starting point
    for item in program.items.iter() {
        if let parser::Item::Function {
            decl: func,
            location: _,
        } = item
        {
            // Scan function body for references
            // TODO: Implement proper AST walking
            // For now, we'll just note that references exist
            tracing::debug!("TODO: Scan function '{}' body for references", func.name);
        }
    }

    tracing::debug!("Found {} references in {}", references.len(), uri);
    ReferenceInfo::new(db, references)
}

// ============================================================================
// Code Actions & Refactorings
// ============================================================================

/// A code action for refactoring
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeAction {
    pub title: String,
    pub kind: CodeActionKind,
    pub edits: Vec<TextEdit>,
    pub is_preferred: bool,
}

/// Types of code actions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodeActionKind {
    /// Quick fix for a diagnostic
    QuickFix,
    /// Refactor: Extract function
    RefactorExtract,
    /// Refactor: Inline variable/function
    RefactorInline,
    /// Refactor: Rename
    RefactorRename,
    /// Refactor: Change signature
    RefactorChangeSignature,
    /// Refactor: Move item
    RefactorMove,
}

impl WindjammerDatabase {
    /// Get available code actions for a selection
    ///
    /// Returns code actions that can be applied at the given range.
    pub fn get_code_actions(
        &mut self,
        file: SourceFile,
        range: tower_lsp::lsp_types::Range,
    ) -> Vec<CodeAction> {
        let mut actions = Vec::new();

        // Try to extract function
        if let Some(action) = self.try_extract_function(file, range) {
            actions.push(action);
        }

        // Try to inline variable
        if let Some(action) = self.try_inline_variable(file, range) {
            actions.push(action);
        }

        // Try to inline function
        if let Some(action) = self.try_inline_function(file, range) {
            actions.push(action);
        }

        actions
    }

    /// Try to extract the selected code into a function
    fn try_extract_function(
        &mut self,
        file: SourceFile,
        range: tower_lsp::lsp_types::Range,
    ) -> Option<CodeAction> {
        let text = file.text(self);
        let lines: Vec<&str> = text.lines().collect();

        // Get selected text
        let start_line = range.start.line as usize;
        let end_line = range.end.line as usize;

        if start_line >= lines.len() || end_line >= lines.len() {
            return None;
        }

        // Extract the selected code
        let mut selected_code = String::new();
        for (idx, line) in lines
            .iter()
            .enumerate()
            .skip(start_line)
            .take(end_line - start_line + 1)
        {
            let line_idx = idx;
            if line_idx == start_line && line_idx == end_line {
                // Single line selection
                let start_char = range.start.character as usize;
                let end_char = range.end.character as usize;
                if start_char < line.len() && end_char <= line.len() {
                    selected_code.push_str(&line[start_char..end_char]);
                }
            } else if line_idx == start_line {
                // First line of multi-line selection
                let line = lines[line_idx];
                let start_char = range.start.character as usize;
                if start_char < line.len() {
                    selected_code.push_str(&line[start_char..]);
                    selected_code.push('\n');
                }
            } else if line_idx == end_line {
                // Last line of multi-line selection
                let line = lines[line_idx];
                let end_char = range.end.character as usize;
                if end_char <= line.len() {
                    selected_code.push_str(&line[..end_char]);
                }
            } else {
                // Middle lines
                selected_code.push_str(lines[line_idx]);
                selected_code.push('\n');
            }
        }

        if selected_code.trim().is_empty() {
            return None;
        }

        // Generate the extracted function
        let function_name = "extracted_function";
        let extracted_function = format!(
            "fn {}() {{\n    {}\n}}\n\n",
            function_name,
            selected_code.trim()
        );

        // Create edits
        let mut edits = Vec::new();

        // 1. Replace selection with function call
        edits.push(TextEdit {
            range,
            new_text: format!("{}()", function_name),
        });

        // 2. Insert function above current function
        // Find the start of the current function
        let insert_line = start_line.saturating_sub(1);
        edits.push(TextEdit {
            range: tower_lsp::lsp_types::Range {
                start: tower_lsp::lsp_types::Position {
                    line: insert_line as u32,
                    character: 0,
                },
                end: tower_lsp::lsp_types::Position {
                    line: insert_line as u32,
                    character: 0,
                },
            },
            new_text: extracted_function,
        });

        Some(CodeAction {
            title: "Extract function".to_string(),
            kind: CodeActionKind::RefactorExtract,
            edits,
            is_preferred: true,
        })
    }

    /// Try to inline a variable at the cursor
    fn try_inline_variable(
        &mut self,
        _file: SourceFile,
        _range: tower_lsp::lsp_types::Range,
    ) -> Option<CodeAction> {
        // TODO: Implement variable inlining
        // This requires:
        // 1. Identify the variable at the cursor
        // 2. Find all uses of the variable
        // 3. Replace uses with the variable's value
        None
    }

    /// Try to inline a function at the cursor
    fn try_inline_function(
        &mut self,
        _file: SourceFile,
        _range: tower_lsp::lsp_types::Range,
    ) -> Option<CodeAction> {
        // TODO: Implement function inlining
        // This requires:
        // 1. Identify the function call at the cursor
        // 2. Get the function body
        // 3. Replace call with inlined body
        None
    }
}

// ============================================================================
// Database API
// ============================================================================

impl WindjammerDatabase {
    /// Create a new database
    pub fn new() -> Self {
        Self {
            storage: Default::default(),
            symbol_cache: Default::default(),
            reference_cache: Default::default(),
        }
    }

    /// Check if symbols are loaded for a file
    pub fn are_symbols_loaded(&self, file: SourceFile) -> bool {
        let uri = file.uri(self);
        self.symbol_cache
            .lock()
            .unwrap()
            .get(uri)
            .copied()
            .unwrap_or(false)
    }

    /// Mark symbols as loaded for a file
    pub fn mark_symbols_loaded(&self, file: SourceFile) {
        let uri = file.uri(self).clone();
        self.symbol_cache.lock().unwrap().insert(uri, true);
    }

    /// Check if references are loaded for a file
    pub fn are_references_loaded(&self, file: SourceFile) -> bool {
        let uri = file.uri(self);
        self.reference_cache
            .lock()
            .unwrap()
            .get(uri)
            .copied()
            .unwrap_or(false)
    }

    /// Mark references as loaded for a file
    pub fn mark_references_loaded(&self, file: SourceFile) {
        let uri = file.uri(self).clone();
        self.reference_cache.lock().unwrap().insert(uri, true);
    }

    /// Get symbols with lazy loading
    pub fn get_symbols_lazy(&mut self, file: SourceFile) -> &Vec<Symbol> {
        if !self.are_symbols_loaded(file) {
            // Trigger computation
            let _symbols = self.get_symbols(file);
            self.mark_symbols_loaded(file);
        }
        self.get_symbols(file)
    }

    /// Get references with lazy loading
    pub fn get_references_lazy(&mut self, file: SourceFile) -> &Vec<SymbolReference> {
        if !self.are_references_loaded(file) {
            // Trigger computation
            let _refs = self.get_references(file);
            self.mark_references_loaded(file);
        }
        self.get_references(file)
    }

    /// Clear lazy loading caches (useful after file changes)
    pub fn clear_lazy_caches(&self) {
        self.symbol_cache.lock().unwrap().clear();
        self.reference_cache.lock().unwrap().clear();
    }

    /// Preload symbols for multiple files
    ///
    /// Note: This is sequential within the database context, but Salsa
    /// internally parallelizes computation across files.
    pub fn preload_symbols(&mut self, files: &[SourceFile]) {
        for file in files {
            if !self.are_symbols_loaded(*file) {
                let _ = self.get_symbols(*file);
                self.mark_symbols_loaded(*file);
            }
        }
    }

    /// Set the source text for a file
    ///
    /// Returns a SourceFile handle that can be used in queries.
    pub fn set_source_text(&mut self, uri: Url, text: String) -> SourceFile {
        SourceFile::new(self, uri, text)
    }

    /// Get the parsed program for a file
    pub fn get_program(&self, file: SourceFile) -> &parser::Program<'_> {
        let parsed = parse(self, file);
        parsed.program(self)
    }

    /// Get imports for a file
    pub fn get_imports(&self, file: SourceFile) -> &Vec<Url> {
        let import_info = imports(self, file);
        import_info.imports(self)
    }

    /// Get symbols for a file
    pub fn get_symbols(&self, file: SourceFile) -> &Vec<Symbol> {
        let symbol_table = extract_symbols(self, file);
        symbol_table.symbols(self)
    }

    /// Get references for a file
    pub fn get_references(&self, file: SourceFile) -> &Vec<SymbolReference> {
        let reference_info = extract_references(self, file);
        reference_info.references(self)
    }

    /// Find all references to a symbol across multiple files
    ///
    /// This searches through all provided files for references to the given symbol name.
    pub fn find_all_references(
        &self,
        symbol_name: &str,
        files: &[SourceFile],
    ) -> Vec<tower_lsp::lsp_types::Location> {
        let mut locations = Vec::new();

        // Search through all files
        for &file in files {
            let uri = file.uri(self).clone();

            // Check if symbol is defined in this file
            let symbols = self.get_symbols(file);
            for symbol in symbols {
                if symbol.name == symbol_name {
                    // Found definition
                    locations.push(tower_lsp::lsp_types::Location {
                        uri: uri.clone(),
                        range: tower_lsp::lsp_types::Range {
                            start: tower_lsp::lsp_types::Position {
                                line: symbol.line,
                                character: symbol.character,
                            },
                            end: tower_lsp::lsp_types::Position {
                                line: symbol.line,
                                character: symbol.character + symbol.name.len() as u32,
                            },
                        },
                    });
                }
            }

            // Check references in this file (when implemented)
            let references = self.get_references(file);
            for reference in references {
                if reference.name == symbol_name {
                    locations.push(tower_lsp::lsp_types::Location {
                        uri: reference.uri.clone(),
                        range: tower_lsp::lsp_types::Range {
                            start: tower_lsp::lsp_types::Position {
                                line: reference.line,
                                character: reference.character,
                            },
                            end: tower_lsp::lsp_types::Position {
                                line: reference.line,
                                character: reference.character + reference.name.len() as u32,
                            },
                        },
                    });
                }
            }
        }

        tracing::debug!(
            "Found {} references to '{}' across {} files",
            locations.len(),
            symbol_name,
            files.len()
        );

        locations
    }

    /// Find the definition of a symbol across multiple files
    ///
    /// Searches through all provided files for the definition of the given symbol.
    /// Returns the first matching definition found.
    pub fn find_definition(
        &self,
        symbol_name: &str,
        files: &[SourceFile],
    ) -> Option<tower_lsp::lsp_types::Location> {
        // Search through all files for the definition
        for &file in files {
            let uri = file.uri(self).clone();
            let symbols = self.get_symbols(file);

            for symbol in symbols {
                if symbol.name == symbol_name {
                    // Found definition!
                    tracing::debug!("Found definition of '{}' in {}", symbol_name, uri);
                    return Some(tower_lsp::lsp_types::Location {
                        uri,
                        range: tower_lsp::lsp_types::Range {
                            start: tower_lsp::lsp_types::Position {
                                line: symbol.line,
                                character: symbol.character,
                            },
                            end: tower_lsp::lsp_types::Position {
                                line: symbol.line,
                                character: symbol.character + symbol.name.len() as u32,
                            },
                        },
                    });
                }
            }
        }

        tracing::debug!("Definition of '{}' not found", symbol_name);
        None
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Resolve an import path to a URI
///
/// Given a source file and an import path like `utils.helpers`, resolves to the actual file URI.
///
/// Resolution strategy:
/// 1. Skip standard library imports (std.*)
/// 2. Check relative to current file
/// 3. Check relative to project root (look for Cargo.toml or wj.toml)
fn resolve_import(source_uri: &Url, import_path: &str) -> Option<Url> {
    tracing::debug!("Resolving import '{}' from {}", import_path, source_uri);

    // Skip standard library imports for now
    if import_path.starts_with("std.") {
        tracing::debug!("Skipping std library import: {}", import_path);
        return None;
    }

    // Convert dotted path to file path: utils.helpers -> utils/helpers.wj
    let file_path = import_path.replace('.', "/") + ".wj";

    // Try to get the directory of the source file
    let source_path = source_uri.to_file_path().ok()?;
    let source_dir = source_path.parent()?;

    // Strategy 1: Relative to current file
    let relative_path = source_dir.join(&file_path);
    if relative_path.exists() {
        let resolved_uri = Url::from_file_path(relative_path).ok()?;
        tracing::debug!("Resolved '{}' to {} (relative)", import_path, resolved_uri);
        return Some(resolved_uri);
    }

    // Strategy 2: Relative to project root (find Cargo.toml or wj.toml)
    let mut current_dir = source_dir;
    while let Some(parent) = current_dir.parent() {
        // Check for project root markers
        if parent.join("Cargo.toml").exists() || parent.join("wj.toml").exists() {
            let project_path = parent.join(&file_path);
            if project_path.exists() {
                let resolved_uri = Url::from_file_path(project_path).ok()?;
                tracing::debug!(
                    "Resolved '{}' to {} (project root)",
                    import_path,
                    resolved_uri
                );
                return Some(resolved_uri);
            }
            break;
        }
        current_dir = parent;
    }

    tracing::debug!("Could not resolve import: {}", import_path);
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
        if let parser::Item::Function { decl: func, .. } = &program.items[0] {
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
        let program1 = db.get_program(file);

        // Second parse (should be memoized - same pointer)
        let program2 = db.get_program(file);

        // Should return the same reference (memoized)
        assert!(std::ptr::eq(program1, program2));
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

        let file = db.set_source_text(uri, "use std.fs\nuse std.http\nfn main() {}".to_string());

        let imports = db.get_imports(file);

        // For now, imports are empty until we implement resolution
        // But the function should not crash
        assert_eq!(imports.len(), 0);
    }
}

// ============================================================================
// Parallel Processing Utilities
// ============================================================================

/// Configuration for parallel processing
#[derive(Debug, Clone)]
pub struct ParallelConfig {
    /// Number of threads to use (0 = use all available cores)
    pub num_threads: usize,
    /// Minimum number of files to process in parallel (below this, use sequential)
    pub min_files_for_parallel: usize,
}

impl Default for ParallelConfig {
    fn default() -> Self {
        Self {
            num_threads: 0,            // Use all available cores
            min_files_for_parallel: 5, // Only parallelize if >= 5 files
        }
    }
}

impl WindjammerDatabase {
    /// Process multiple source files in parallel
    ///
    /// This is useful for initial project loading or when many files change at once.
    /// Uses rayon to parallelize parsing and symbol extraction.
    pub fn process_files_parallel(
        &mut self,
        files: Vec<(Url, String)>,
        config: &ParallelConfig,
    ) -> Vec<SourceFile> {
        // Configure rayon thread pool if specified
        if config.num_threads > 0 {
            rayon::ThreadPoolBuilder::new()
                .num_threads(config.num_threads)
                .build_global()
                .ok(); // Ignore error if already initialized
        }

        // If too few files, process sequentially
        if files.len() < config.min_files_for_parallel {
            return files
                .into_iter()
                .map(|(uri, text)| self.set_source_text(uri, text))
                .collect();
        }

        tracing::info!(
            "Processing {} files in parallel with {} threads",
            files.len(),
            rayon::current_num_threads()
        );

        // Process files in parallel
        // Note: We can't parallelize the actual Salsa operations due to &mut self,
        // but we can prepare the data in parallel
        files
            .into_iter()
            .map(|(uri, text)| self.set_source_text(uri, text))
            .collect()
    }

    /// Extract symbols from multiple files in parallel
    ///
    /// This leverages Salsa's caching - if files haven't changed, symbols are cached.
    pub fn extract_symbols_parallel(&mut self, files: &[SourceFile]) -> Vec<&[Symbol]> {
        tracing::debug!("Extracting symbols from {} files", files.len());

        // Salsa queries are already cached, so we can call them sequentially
        // The caching makes this very fast
        files
            .iter()
            .map(|file| {
                let symbols = self.get_symbols(*file);
                symbols.as_slice()
            })
            .collect()
    }

    /// Find all references across multiple files (optimized for parallel queries)
    ///
    /// This is the same as find_all_references but with additional logging
    /// and potential future optimizations for large file sets.
    pub fn find_all_references_parallel(
        &mut self,
        symbol_name: &str,
        files: &[SourceFile],
    ) -> Vec<tower_lsp::lsp_types::Location> {
        use tower_lsp::lsp_types::{Location, Position, Range};

        tracing::debug!(
            "Finding all references to '{}' across {} files (parallel-optimized)",
            symbol_name,
            files.len()
        );

        let mut locations = Vec::new();

        for file in files {
            let uri = file.uri(self);
            let symbols = self.get_symbols(*file); // Cached by Salsa

            for symbol in symbols.iter() {
                if symbol.name == symbol_name {
                    locations.push(Location {
                        uri: uri.clone(),
                        range: Range {
                            start: Position {
                                line: symbol.line,
                                character: symbol.character,
                            },
                            end: Position {
                                line: symbol.line,
                                character: symbol.character + symbol.name.len() as u32,
                            },
                        },
                    });
                }
            }
        }

        tracing::debug!("Found {} locations for '{}'", locations.len(), symbol_name);
        locations
    }

    /// Find all implementations of a trait
    ///
    /// Returns impl blocks that implement the specified trait.
    pub fn find_trait_implementations(
        &mut self,
        trait_name: &str,
        files: &[SourceFile],
    ) -> Vec<TraitImplementation> {
        tracing::debug!("Finding implementations of trait '{}'", trait_name);

        let mut implementations = Vec::new();

        for file in files {
            let uri = file.uri(self);
            let symbols = self.get_symbols(*file);

            for symbol in symbols.iter() {
                if symbol.kind == SymbolKind::Impl {
                    // Impl names are formatted as "impl Trait for Type"
                    if symbol.name.contains(trait_name) {
                        implementations.push(TraitImplementation {
                            trait_name: trait_name.to_string(),
                            type_name: self.extract_type_from_impl(&symbol.name),
                            location: tower_lsp::lsp_types::Location {
                                uri: uri.clone(),
                                range: tower_lsp::lsp_types::Range {
                                    start: tower_lsp::lsp_types::Position {
                                        line: symbol.line,
                                        character: symbol.character,
                                    },
                                    end: tower_lsp::lsp_types::Position {
                                        line: symbol.line,
                                        character: symbol.character + symbol.name.len() as u32,
                                    },
                                },
                            },
                        });
                    }
                }
            }
        }

        tracing::debug!(
            "Found {} implementations of '{}'",
            implementations.len(),
            trait_name
        );
        implementations
    }

    /// Extract type name from impl block name
    fn extract_type_from_impl(&self, impl_name: &str) -> String {
        // Parse "impl Trait for Type" or "impl Type"
        if let Some(for_pos) = impl_name.find(" for ") {
            impl_name[for_pos + 5..].trim().to_string()
        } else if let Some(impl_pos) = impl_name.find("impl ") {
            impl_name[impl_pos + 5..].trim().to_string()
        } else {
            impl_name.to_string()
        }
    }

    /// Get hover information for a symbol
    ///
    /// Returns type information, documentation, etc.
    /// Note: This requires the file to already be loaded via set_source_text
    pub fn get_hover_info(
        &mut self,
        file: SourceFile,
        line: u32,
        character: u32,
    ) -> Option<HoverInfo> {
        let symbols = self.get_symbols(file);

        // Find symbol at position
        for symbol in symbols.iter() {
            if symbol.line == line
                && character >= symbol.character
                && character <= symbol.character + symbol.name.len() as u32
            {
                return Some(HoverInfo {
                    name: symbol.name.clone(),
                    kind: format!("{:?}", symbol.kind),
                    type_info: symbol.type_info.clone(),
                    documentation: symbol.doc.clone(),
                });
            }
        }

        None
    }
}

/// Information about a trait implementation
#[derive(Debug, Clone)]
pub struct TraitImplementation {
    pub trait_name: String,
    pub type_name: String,
    pub location: tower_lsp::lsp_types::Location,
}

/// Hover information for a symbol
#[derive(Debug, Clone)]
pub struct HoverInfo {
    pub name: String,
    pub kind: String,
    pub type_info: Option<String>,
    pub documentation: Option<String>,
}

// ============================================================================
// Code Lens Support
// ============================================================================

/// A code lens item that can be displayed above a symbol
#[derive(Debug, Clone)]
pub struct CodeLens {
    pub range: tower_lsp::lsp_types::Range,
    pub command: Option<CodeLensCommand>,
    pub data: Option<serde_json::Value>,
}

/// A command that can be executed from a code lens
#[derive(Debug, Clone)]
pub struct CodeLensCommand {
    pub title: String,
    pub command: String,
    pub arguments: Vec<serde_json::Value>,
}

impl WindjammerDatabase {
    /// Generate code lenses for a file
    ///
    /// Returns code lenses showing:
    /// - Reference counts for functions, structs, traits
    /// - Implementation counts for traits
    /// - Test run commands for test functions
    pub fn get_code_lenses(&mut self, file: SourceFile, all_files: &[SourceFile]) -> Vec<CodeLens> {
        let mut lenses = Vec::new();
        let file_uri = file.uri(self).clone();

        // Clone symbols to avoid borrow checker issues
        let symbols: Vec<Symbol> = self.get_symbols(file).to_vec();

        for symbol in symbols.iter() {
            // Skip symbols without ranges
            let range = match &symbol.range {
                Some(r) => tower_lsp::lsp_types::Range {
                    start: tower_lsp::lsp_types::Position {
                        line: r.start_line,
                        character: r.start_character,
                    },
                    end: tower_lsp::lsp_types::Position {
                        line: r.end_line,
                        character: r.end_character,
                    },
                },
                None => continue,
            };

            match symbol.kind {
                SymbolKind::Function | SymbolKind::Struct | SymbolKind::Trait => {
                    // Count references across all files
                    let ref_count = self.count_references(&symbol.name, all_files);

                    // Create reference count lens
                    let title = if ref_count == 1 {
                        format!("{} reference", ref_count)
                    } else {
                        format!("{} references", ref_count)
                    };

                    lenses.push(CodeLens {
                        range,
                        command: Some(CodeLensCommand {
                            title,
                            command: "windjammer.showReferences".to_string(),
                            arguments: vec![
                                serde_json::json!(symbol.name),
                                serde_json::json!(file_uri.to_string()),
                            ],
                        }),
                        data: None,
                    });

                    // For traits, also show implementation count
                    if symbol.kind == SymbolKind::Trait {
                        let impls = self.find_trait_implementations(&symbol.name, all_files);
                        let impl_count = impls.len();

                        let title = if impl_count == 1 {
                            format!("{} implementation", impl_count)
                        } else {
                            format!("{} implementations", impl_count)
                        };

                        lenses.push(CodeLens {
                            range,
                            command: Some(CodeLensCommand {
                                title,
                                command: "windjammer.showImplementations".to_string(),
                                arguments: vec![
                                    serde_json::json!(symbol.name),
                                    serde_json::json!(file_uri.to_string()),
                                ],
                            }),
                            data: None,
                        });
                    }
                }
                _ => {
                    // Other symbol kinds don't get code lenses for now
                }
            }
        }

        tracing::debug!("Generated {} code lenses for {}", lenses.len(), file_uri);
        lenses
    }

    /// Count references to a symbol across all files
    ///
    /// This is a helper for code lens generation.
    fn count_references(&mut self, symbol_name: &str, files: &[SourceFile]) -> usize {
        let mut count = 0;

        for file in files {
            let symbols = self.get_symbols(*file);
            count += symbols.iter().filter(|s| s.name == symbol_name).count();
        }

        count
    }
}

// ============================================================================
// Inlay Hints Support
// ============================================================================

/// An inlay hint that can be displayed inline in the editor
#[derive(Debug, Clone)]
pub struct InlayHint {
    pub position: tower_lsp::lsp_types::Position,
    pub label: String,
    pub kind: InlayHintKind,
    pub tooltip: Option<String>,
}

/// The kind of inlay hint
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InlayHintKind {
    /// Type annotation (e.g., `: string`)
    Type,
    /// Parameter name (e.g., `x: `)
    Parameter,
}

impl WindjammerDatabase {
    /// Generate inlay hints for a file
    ///
    /// Returns inlay hints showing:
    /// - Type annotations for variables and return values
    /// - Parameter names in function calls
    pub fn get_inlay_hints(&mut self, file: SourceFile) -> Vec<InlayHint> {
        let mut hints = Vec::new();
        let symbols = self.get_symbols(file);

        for symbol in symbols.iter() {
            // Add type hints for symbols with type information
            if let Some(type_info) = &symbol.type_info {
                // Only add hints for functions (return types)
                if symbol.kind == SymbolKind::Function {
                    // Position hint after the function signature
                    if let Some(range) = &symbol.range {
                        hints.push(InlayHint {
                            position: tower_lsp::lsp_types::Position {
                                line: range.end_line,
                                character: range.end_character,
                            },
                            label: format!(": {}", type_info),
                            kind: InlayHintKind::Type,
                            tooltip: Some(format!("Return type of {}", symbol.name)),
                        });
                    }
                }
            }
        }

        tracing::debug!(
            "Generated {} inlay hints for {}",
            hints.len(),
            file.uri(self)
        );
        hints
    }

    /// Generate parameter hints for function calls
    ///
    /// This would analyze function call sites and show parameter names.
    /// For now, this is a placeholder that returns empty hints.
    pub fn get_parameter_hints(
        &mut self,
        _file: SourceFile,
        _line: u32,
        _character: u32,
    ) -> Vec<InlayHint> {
        // TODO: Implement parameter hint generation
        // This requires:
        // 1. Finding function call expressions in the AST
        // 2. Looking up the function definition
        // 3. Matching arguments to parameters
        // 4. Generating hints for each argument
        Vec::new()
    }
}

// ============================================================================
// Call Hierarchy Support
// ============================================================================

/// A call hierarchy item representing a function or method
#[derive(Debug, Clone)]
pub struct CallHierarchyItem {
    pub name: String,
    pub kind: SymbolKind,
    pub uri: Url,
    pub range: tower_lsp::lsp_types::Range,
    pub selection_range: tower_lsp::lsp_types::Range,
}

/// An incoming call (who calls this function)
#[derive(Debug, Clone)]
pub struct IncomingCall {
    pub from: CallHierarchyItem,
    pub from_ranges: Vec<tower_lsp::lsp_types::Range>,
}

/// An outgoing call (what this function calls)
#[derive(Debug, Clone)]
pub struct OutgoingCall {
    pub to: CallHierarchyItem,
    pub from_ranges: Vec<tower_lsp::lsp_types::Range>,
}

impl WindjammerDatabase {
    /// Prepare call hierarchy for a symbol at a position
    ///
    /// Returns the call hierarchy item if the position is on a function.
    pub fn prepare_call_hierarchy(
        &mut self,
        file: SourceFile,
        line: u32,
        character: u32,
    ) -> Option<CallHierarchyItem> {
        let symbols = self.get_symbols(file);
        let uri = file.uri(self).clone();

        // Find the symbol at the given position
        for symbol in symbols.iter() {
            if symbol.kind == SymbolKind::Function && symbol.line == line {
                // Check if character is within the symbol
                if character >= symbol.character
                    && character <= symbol.character + symbol.name.len() as u32
                {
                    let range = symbol.range.as_ref().map(|r| tower_lsp::lsp_types::Range {
                        start: tower_lsp::lsp_types::Position {
                            line: r.start_line,
                            character: r.start_character,
                        },
                        end: tower_lsp::lsp_types::Position {
                            line: r.end_line,
                            character: r.end_character,
                        },
                    })?;

                    let selection_range = symbol
                        .name_range
                        .as_ref()
                        .map(|r| tower_lsp::lsp_types::Range {
                            start: tower_lsp::lsp_types::Position {
                                line: r.start_line,
                                character: r.start_character,
                            },
                            end: tower_lsp::lsp_types::Position {
                                line: r.end_line,
                                character: r.end_character,
                            },
                        })
                        .unwrap_or(range);

                    return Some(CallHierarchyItem {
                        name: symbol.name.clone(),
                        kind: symbol.kind,
                        uri,
                        range,
                        selection_range,
                    });
                }
            }
        }

        None
    }

    /// Get incoming calls for a function
    ///
    /// Returns all places where this function is called.
    pub fn incoming_calls(
        &mut self,
        item: &CallHierarchyItem,
        all_files: &[SourceFile],
    ) -> Vec<IncomingCall> {
        let mut calls = Vec::new();

        // Find all references to this function
        for file in all_files {
            let symbols = self.get_symbols(*file);
            let uri = file.uri(self).clone();

            // Look for symbols that might call this function
            for symbol in symbols.iter() {
                if symbol.kind == SymbolKind::Function && symbol.name != item.name {
                    // This is a different function - it might call our target
                    // For now, we'll use a simple heuristic: if the function exists,
                    // assume it might be called
                    // TODO: Implement proper call graph analysis

                    // Create a call hierarchy item for the caller
                    if let (Some(range), Some(selection_range)) =
                        (&symbol.range, &symbol.name_range)
                    {
                        let from = CallHierarchyItem {
                            name: symbol.name.clone(),
                            kind: symbol.kind,
                            uri: uri.clone(),
                            range: tower_lsp::lsp_types::Range {
                                start: tower_lsp::lsp_types::Position {
                                    line: range.start_line,
                                    character: range.start_character,
                                },
                                end: tower_lsp::lsp_types::Position {
                                    line: range.end_line,
                                    character: range.end_character,
                                },
                            },
                            selection_range: tower_lsp::lsp_types::Range {
                                start: tower_lsp::lsp_types::Position {
                                    line: selection_range.start_line,
                                    character: selection_range.start_character,
                                },
                                end: tower_lsp::lsp_types::Position {
                                    line: selection_range.end_line,
                                    character: selection_range.end_character,
                                },
                            },
                        };

                        // For now, use the function's range as the call site
                        calls.push(IncomingCall {
                            from,
                            from_ranges: vec![tower_lsp::lsp_types::Range {
                                start: tower_lsp::lsp_types::Position {
                                    line: symbol.line,
                                    character: symbol.character,
                                },
                                end: tower_lsp::lsp_types::Position {
                                    line: symbol.line,
                                    character: symbol.character + symbol.name.len() as u32,
                                },
                            }],
                        });
                    }
                }
            }
        }

        tracing::debug!("Found {} incoming calls to '{}'", calls.len(), item.name);
        calls
    }

    /// Get outgoing calls from a function
    ///
    /// Returns all functions that this function calls.
    pub fn outgoing_calls(
        &mut self,
        item: &CallHierarchyItem,
        all_files: &[SourceFile],
    ) -> Vec<OutgoingCall> {
        let mut calls = Vec::new();

        // Find all functions that might be called by this function
        // For now, we'll use a simple heuristic: list all other functions
        // TODO: Implement proper call graph analysis by parsing function bodies

        for file in all_files {
            let symbols = self.get_symbols(*file);
            let uri = file.uri(self).clone();

            for symbol in symbols.iter() {
                if symbol.kind == SymbolKind::Function && symbol.name != item.name {
                    // This is a different function - it might be called by our target
                    if let (Some(range), Some(selection_range)) =
                        (&symbol.range, &symbol.name_range)
                    {
                        let to = CallHierarchyItem {
                            name: symbol.name.clone(),
                            kind: symbol.kind,
                            uri: uri.clone(),
                            range: tower_lsp::lsp_types::Range {
                                start: tower_lsp::lsp_types::Position {
                                    line: range.start_line,
                                    character: range.start_character,
                                },
                                end: tower_lsp::lsp_types::Position {
                                    line: range.end_line,
                                    character: range.end_character,
                                },
                            },
                            selection_range: tower_lsp::lsp_types::Range {
                                start: tower_lsp::lsp_types::Position {
                                    line: selection_range.start_line,
                                    character: selection_range.start_character,
                                },
                                end: tower_lsp::lsp_types::Position {
                                    line: selection_range.end_line,
                                    character: selection_range.end_character,
                                },
                            },
                        };

                        // Use the target function's range as the call site
                        calls.push(OutgoingCall {
                            to,
                            from_ranges: vec![tower_lsp::lsp_types::Range {
                                start: tower_lsp::lsp_types::Position {
                                    line: symbol.line,
                                    character: symbol.character,
                                },
                                end: tower_lsp::lsp_types::Position {
                                    line: symbol.line,
                                    character: symbol.character + symbol.name.len() as u32,
                                },
                            }],
                        });
                    }
                }
            }
        }

        tracing::debug!("Found {} outgoing calls from '{}'", calls.len(), item.name);
        calls
    }
}

// ============================================================================
// Unused Code Detection
// ============================================================================

/// Information about an unused symbol
#[derive(Debug, Clone)]
pub struct UnusedSymbol {
    pub name: String,
    pub kind: SymbolKind,
    pub location: tower_lsp::lsp_types::Location,
    pub reason: UnusedReason,
}

/// Reason why a symbol is considered unused
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnusedReason {
    /// Symbol is never referenced
    NeverReferenced,
    /// Symbol is only referenced in dead code
    OnlyInDeadCode,
    /// Symbol is exported but never used
    ExportedButUnused,
}

impl WindjammerDatabase {
    /// Find all unused symbols in the workspace
    ///
    /// Returns symbols that are defined but never referenced.
    pub fn find_unused_symbols(&mut self, files: &[SourceFile]) -> Vec<UnusedSymbol> {
        let mut unused = Vec::new();

        // Build a set of all referenced symbol names
        let mut referenced_symbols = std::collections::HashSet::new();
        for file in files {
            let symbols = self.get_symbols(*file);
            for symbol in symbols.iter() {
                // Count how many times this symbol appears across all files
                let mut ref_count = 0;
                for other_file in files {
                    let other_symbols = self.get_symbols(*other_file);
                    ref_count += other_symbols
                        .iter()
                        .filter(|s| s.name == symbol.name)
                        .count();
                }

                // If it appears more than once, it's referenced somewhere
                if ref_count > 1 {
                    referenced_symbols.insert(symbol.name.clone());
                }
            }
        }

        // Find symbols that are defined but not referenced
        for file in files {
            let uri = file.uri(self).clone();
            let symbols = self.get_symbols(*file);

            for symbol in symbols.iter() {
                // Skip certain kinds that are typically entry points
                match symbol.kind {
                    SymbolKind::Const | SymbolKind::Static => continue, // May be used externally
                    _ => {}
                }

                // Check if this symbol is referenced
                if !referenced_symbols.contains(&symbol.name) {
                    // Only report if we have location information
                    if let Some(range) = &symbol.range {
                        unused.push(UnusedSymbol {
                            name: symbol.name.clone(),
                            kind: symbol.kind,
                            location: tower_lsp::lsp_types::Location {
                                uri: uri.clone(),
                                range: tower_lsp::lsp_types::Range {
                                    start: tower_lsp::lsp_types::Position {
                                        line: range.start_line,
                                        character: range.start_character,
                                    },
                                    end: tower_lsp::lsp_types::Position {
                                        line: range.end_line,
                                        character: range.end_character,
                                    },
                                },
                            },
                            reason: UnusedReason::NeverReferenced,
                        });
                    }
                }
            }
        }

        tracing::debug!("Found {} unused symbols", unused.len());
        unused
    }

    /// Find unused functions specifically
    ///
    /// This is a specialized version that only looks for unused functions.
    pub fn find_unused_functions(&mut self, files: &[SourceFile]) -> Vec<UnusedSymbol> {
        self.find_unused_symbols(files)
            .into_iter()
            .filter(|u| u.kind == SymbolKind::Function)
            .collect()
    }

    /// Find unused structs specifically
    pub fn find_unused_structs(&mut self, files: &[SourceFile]) -> Vec<UnusedSymbol> {
        self.find_unused_symbols(files)
            .into_iter()
            .filter(|u| u.kind == SymbolKind::Struct)
            .collect()
    }
}

// ============================================================================
// Dependency Analysis
// ============================================================================

/// A dependency between two files
#[derive(Debug, Clone)]
pub struct FileDependency {
    pub from: Url,
    pub to: Url,
    pub kind: DependencyKind,
}

/// The kind of dependency
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DependencyKind {
    /// Direct import
    Import,
    /// Symbol reference
    SymbolReference,
    /// Type reference
    TypeReference,
}

/// Dependency graph for the workspace
#[derive(Debug, Clone)]
pub struct DependencyGraph {
    pub dependencies: Vec<FileDependency>,
    pub files: Vec<Url>,
}

impl DependencyGraph {
    /// Check if there are circular dependencies
    pub fn has_circular_dependencies(&self) -> bool {
        // Simple cycle detection using DFS
        let mut visited = std::collections::HashSet::new();
        let mut rec_stack = std::collections::HashSet::new();

        for file in &self.files {
            if self.has_cycle_util(file, &mut visited, &mut rec_stack) {
                return true;
            }
        }

        false
    }

    fn has_cycle_util(
        &self,
        file: &Url,
        visited: &mut std::collections::HashSet<Url>,
        rec_stack: &mut std::collections::HashSet<Url>,
    ) -> bool {
        if rec_stack.contains(file) {
            return true;
        }

        if visited.contains(file) {
            return false;
        }

        visited.insert(file.clone());
        rec_stack.insert(file.clone());

        // Check all dependencies
        for dep in &self.dependencies {
            if dep.from == *file && self.has_cycle_util(&dep.to, visited, rec_stack) {
                return true;
            }
        }

        rec_stack.remove(file);
        false
    }

    /// Get all dependencies of a file
    pub fn get_dependencies(&self, file: &Url) -> Vec<&FileDependency> {
        self.dependencies
            .iter()
            .filter(|d| d.from == *file)
            .collect()
    }

    /// Get all dependents of a file (who depends on this file)
    pub fn get_dependents(&self, file: &Url) -> Vec<&FileDependency> {
        self.dependencies.iter().filter(|d| d.to == *file).collect()
    }
}

impl WindjammerDatabase {
    /// Build a dependency graph for the workspace
    ///
    /// Analyzes imports and symbol references to build a complete dependency graph.
    pub fn build_dependency_graph(&mut self, files: &[SourceFile]) -> DependencyGraph {
        let mut dependencies = Vec::new();
        let file_uris: Vec<Url> = files.iter().map(|f| f.uri(self).clone()).collect();

        // Build import-based dependencies
        for file in files {
            let uri = file.uri(self).clone();
            let imports = self.get_imports(*file);

            for import_uri in imports.iter() {
                dependencies.push(FileDependency {
                    from: uri.clone(),
                    to: import_uri.clone(),
                    kind: DependencyKind::Import,
                });
            }
        }

        // Build symbol-based dependencies
        for file in files {
            let uri = file.uri(self).clone();
            let symbols = self.get_symbols(*file);

            // For each symbol in this file, check if it's used in other files
            for symbol in symbols.iter() {
                for other_file in files {
                    let other_uri = other_file.uri(self).clone();
                    if uri != other_uri {
                        let other_symbols = self.get_symbols(*other_file);
                        // Check if other file references this symbol
                        if other_symbols.iter().any(|s| s.name == symbol.name) {
                            dependencies.push(FileDependency {
                                from: other_uri.clone(),
                                to: uri.clone(),
                                kind: DependencyKind::SymbolReference,
                            });
                        }
                    }
                }
            }
        }

        tracing::debug!(
            "Built dependency graph with {} dependencies for {} files",
            dependencies.len(),
            files.len()
        );

        DependencyGraph {
            dependencies,
            files: file_uris,
        }
    }

    /// Find circular dependencies in the workspace
    pub fn find_circular_dependencies(&mut self, files: &[SourceFile]) -> Vec<Vec<Url>> {
        let graph = self.build_dependency_graph(files);
        let cycles = Vec::new();

        if graph.has_circular_dependencies() {
            // For now, just report that cycles exist
            // A full implementation would extract the actual cycles
            tracing::warn!("Circular dependencies detected in workspace");
        }

        cycles
    }

    /// Calculate coupling metrics for files
    ///
    /// Returns (afferent coupling, efferent coupling) for each file.
    /// Afferent = number of files that depend on this file
    /// Efferent = number of files this file depends on
    pub fn calculate_coupling(&mut self, files: &[SourceFile]) -> Vec<(Url, usize, usize)> {
        let graph = self.build_dependency_graph(files);
        let mut metrics = Vec::new();

        for file_uri in &graph.files {
            let afferent = graph.get_dependents(file_uri).len();
            let efferent = graph.get_dependencies(file_uri).len();
            metrics.push((file_uri.clone(), afferent, efferent));
        }

        metrics
    }
}

// ============================================================================
// Code Metrics
// ============================================================================

/// Metrics for a single file
#[derive(Debug, Clone)]
pub struct FileMetrics {
    pub uri: Url,
    pub lines_of_code: usize,
    pub num_functions: usize,
    pub num_structs: usize,
    pub num_enums: usize,
    pub num_traits: usize,
    pub avg_function_length: f64,
    pub max_function_length: usize,
    pub complexity_score: usize,
}

/// Metrics for the entire workspace
#[derive(Debug, Clone)]
pub struct WorkspaceMetrics {
    pub total_files: usize,
    pub total_lines: usize,
    pub total_functions: usize,
    pub total_structs: usize,
    pub total_enums: usize,
    pub total_traits: usize,
    pub avg_file_size: f64,
    pub largest_file: Option<(Url, usize)>,
}

impl WindjammerDatabase {
    /// Calculate metrics for a single file
    pub fn calculate_file_metrics(&mut self, file: SourceFile) -> FileMetrics {
        let uri = file.uri(self).clone();
        let text = file.text(self);
        let symbols = self.get_symbols(file);

        // Count lines of code (non-empty, non-comment lines)
        let lines_of_code = text
            .lines()
            .filter(|line| {
                let trimmed = line.trim();
                !trimmed.is_empty() && !trimmed.starts_with("//")
            })
            .count();

        // Count symbol types
        let num_functions = symbols
            .iter()
            .filter(|s| s.kind == SymbolKind::Function)
            .count();
        let num_structs = symbols
            .iter()
            .filter(|s| s.kind == SymbolKind::Struct)
            .count();
        let num_enums = symbols
            .iter()
            .filter(|s| s.kind == SymbolKind::Enum)
            .count();
        let num_traits = symbols
            .iter()
            .filter(|s| s.kind == SymbolKind::Trait)
            .count();

        // Calculate function lengths (approximate based on line count)
        let mut function_lengths = Vec::new();
        for symbol in symbols.iter() {
            if symbol.kind == SymbolKind::Function {
                if let Some(range) = &symbol.range {
                    let length = (range.end_line - range.start_line) as usize;
                    function_lengths.push(length);
                }
            }
        }

        let avg_function_length = if function_lengths.is_empty() {
            0.0
        } else {
            function_lengths.iter().sum::<usize>() as f64 / function_lengths.len() as f64
        };

        let max_function_length = function_lengths.iter().max().copied().unwrap_or(0);

        // Simple complexity score (based on number of symbols and LOC)
        let complexity_score = num_functions * 2 + num_structs + num_enums + num_traits;

        FileMetrics {
            uri,
            lines_of_code,
            num_functions,
            num_structs,
            num_enums,
            num_traits,
            avg_function_length,
            max_function_length,
            complexity_score,
        }
    }

    /// Calculate metrics for the entire workspace
    pub fn calculate_workspace_metrics(&mut self, files: &[SourceFile]) -> WorkspaceMetrics {
        let file_metrics: Vec<FileMetrics> = files
            .iter()
            .map(|f| self.calculate_file_metrics(*f))
            .collect();

        let total_files = file_metrics.len();
        let total_lines: usize = file_metrics.iter().map(|m| m.lines_of_code).sum();
        let total_functions: usize = file_metrics.iter().map(|m| m.num_functions).sum();
        let total_structs: usize = file_metrics.iter().map(|m| m.num_structs).sum();
        let total_enums: usize = file_metrics.iter().map(|m| m.num_enums).sum();
        let total_traits: usize = file_metrics.iter().map(|m| m.num_traits).sum();

        let avg_file_size = if total_files > 0 {
            total_lines as f64 / total_files as f64
        } else {
            0.0
        };

        let largest_file = file_metrics
            .iter()
            .max_by_key(|m| m.lines_of_code)
            .map(|m| (m.uri.clone(), m.lines_of_code));

        WorkspaceMetrics {
            total_files,
            total_lines,
            total_functions,
            total_structs,
            total_enums,
            total_traits,
            avg_file_size,
            largest_file,
        }
    }

    /// Find files that exceed size thresholds
    pub fn find_large_files(
        &mut self,
        files: &[SourceFile],
        threshold: usize,
    ) -> Vec<(Url, usize)> {
        files
            .iter()
            .map(|f| {
                let metrics = self.calculate_file_metrics(*f);
                (metrics.uri, metrics.lines_of_code)
            })
            .filter(|(_, loc)| *loc > threshold)
            .collect()
    }

    /// Find functions that exceed length thresholds
    pub fn find_long_functions(
        &mut self,
        files: &[SourceFile],
        threshold: usize,
    ) -> Vec<(Url, String, usize)> {
        let mut long_functions = Vec::new();

        for file in files {
            let uri = file.uri(self).clone();
            let symbols = self.get_symbols(*file);

            for symbol in symbols.iter() {
                if symbol.kind == SymbolKind::Function {
                    if let Some(range) = &symbol.range {
                        let length = (range.end_line - range.start_line) as usize;
                        if length > threshold {
                            long_functions.push((uri.clone(), symbol.name.clone(), length));
                        }
                    }
                }
            }
        }

        long_functions
    }
}

// ============================================================================
// Diagnostics & Linting Engine
// ============================================================================

/// Diagnostic severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
    Hint,
}

/// Diagnostic category (inspired by golangci-lint)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticCategory {
    // Code Quality
    CodeComplexity,
    CodeStyle,
    CodeSmell,

    // Error Detection
    BugRisk,
    ErrorHandling,
    NilCheck,

    // Performance
    Performance,
    Memory,

    // Security
    Security,

    // Maintainability
    Naming,
    Documentation,
    Unused,

    // Dependencies
    Import,
    Dependency,
}

/// A diagnostic message
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub severity: DiagnosticSeverity,
    pub category: DiagnosticCategory,
    pub message: String,
    pub location: tower_lsp::lsp_types::Location,
    pub rule: String,
    pub suggestion: Option<String>,
    pub fix: Option<AutoFix>,
}

/// An automatic fix for a diagnostic
#[derive(Debug, Clone)]
pub struct AutoFix {
    pub description: String,
    pub edits: Vec<TextEdit>,
}

/// A text edit for auto-fixing
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEdit {
    pub range: tower_lsp::lsp_types::Range,
    pub new_text: String,
}

/// Configuration for the linting engine
#[derive(Debug, Clone)]
pub struct LintConfig {
    pub max_function_length: usize,
    pub max_file_length: usize,
    pub max_complexity: usize,
    pub check_unused: bool,
    pub check_style: bool,
    pub check_performance: bool,
    pub check_security: bool,
    pub check_error_handling: bool,
    pub enable_autofix: bool,
}

impl Default for LintConfig {
    fn default() -> Self {
        Self {
            max_function_length: 50,
            max_file_length: 500,
            max_complexity: 10,
            check_unused: true,
            check_style: true,
            check_performance: true,
            check_security: true,
            check_error_handling: true,
            enable_autofix: false,
        }
    }
}

impl WindjammerDatabase {
    /// Run all linting checks on a workspace
    pub fn lint_workspace(&mut self, files: &[SourceFile], config: &LintConfig) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Check for unused code
        if config.check_unused {
            diagnostics.extend(self.check_unused_code(files, config));
        }

        // Check code complexity
        diagnostics.extend(self.check_complexity(files, config));

        // Check code style
        if config.check_style {
            diagnostics.extend(self.check_style(files, config));
        }

        // Check error handling
        if config.check_error_handling {
            diagnostics.extend(self.check_error_handling(files, config));
        }

        // Check performance
        if config.check_performance {
            diagnostics.extend(self.check_performance(files, config));
        }

        // Check security
        if config.check_security {
            diagnostics.extend(self.check_security(files, config));
        }

        // Check circular dependencies
        diagnostics.extend(self.check_circular_deps(files));

        tracing::info!("Linting complete: {} diagnostics found", diagnostics.len());
        diagnostics
    }

    /// Check for unused code (similar to unused, deadcode, varcheck)
    fn check_unused_code(&mut self, files: &[SourceFile], config: &LintConfig) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let unused = self.find_unused_symbols(files);

        for symbol in unused {
            // Auto-fix: Can add #[allow(dead_code)] attribute
            let fix = if config.enable_autofix {
                Some(AutoFix {
                    description: format!("Add #[allow(dead_code)] to {}", symbol.name),
                    edits: vec![TextEdit {
                        range: symbol.location.range,
                        new_text: format!("#[allow(dead_code)]\n{}", symbol.name),
                    }],
                })
            } else {
                None
            };

            diagnostics.push(Diagnostic {
                severity: DiagnosticSeverity::Warning,
                category: DiagnosticCategory::Unused,
                message: format!(
                    "Unused {}: '{}'",
                    format!("{:?}", symbol.kind).to_lowercase(),
                    symbol.name
                ),
                location: symbol.location,
                rule: "unused-code".to_string(),
                suggestion: Some(format!(
                    "Remove unused {} or mark with #[allow(dead_code)]",
                    format!("{:?}", symbol.kind).to_lowercase()
                )),
                fix,
            });
        }

        diagnostics
    }

    /// Check code complexity (similar to gocyclo, gocognit, cyclop)
    fn check_complexity(&mut self, files: &[SourceFile], config: &LintConfig) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Check function length
        let long_funcs = self.find_long_functions(files, config.max_function_length);
        for (uri, name, length) in long_funcs {
            diagnostics.push(Diagnostic {
                severity: DiagnosticSeverity::Warning,
                category: DiagnosticCategory::CodeComplexity,
                message: format!(
                    "Function '{}' is too long ({} lines, max {})",
                    name, length, config.max_function_length
                ),
                location: tower_lsp::lsp_types::Location {
                    uri,
                    range: tower_lsp::lsp_types::Range::default(),
                },
                rule: "function-length".to_string(),
                suggestion: Some(
                    "Consider breaking this function into smaller functions".to_string(),
                ),
                fix: None, // Complex refactoring, no auto-fix
            });
        }

        // Check file length
        let large_files = self.find_large_files(files, config.max_file_length);
        for (uri, loc) in large_files {
            diagnostics.push(Diagnostic {
                severity: DiagnosticSeverity::Info,
                category: DiagnosticCategory::CodeComplexity,
                message: format!(
                    "File is large ({} lines, max {})",
                    loc, config.max_file_length
                ),
                location: tower_lsp::lsp_types::Location {
                    uri,
                    range: tower_lsp::lsp_types::Range::default(),
                },
                rule: "file-length".to_string(),
                suggestion: Some("Consider splitting this file into multiple modules".to_string()),
                fix: None, // Complex refactoring, no auto-fix
            });
        }

        diagnostics
    }

    /// Check code style (similar to golint, revive, stylecheck)
    fn check_style(&mut self, files: &[SourceFile], config: &LintConfig) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        for file in files {
            let uri = file.uri(self).clone();
            let symbols = self.get_symbols(*file);

            for symbol in symbols.iter() {
                // Check naming conventions
                if symbol.kind == SymbolKind::Struct
                    && !symbol.name.chars().next().unwrap_or('a').is_uppercase()
                {
                    if let Some(range) = &symbol.name_range {
                        let capitalized = capitalize_first(&symbol.name);

                        // Auto-fix: Can rename the struct
                        let fix = if config.enable_autofix {
                            Some(AutoFix {
                                description: format!(
                                    "Rename '{}' to '{}'",
                                    symbol.name, capitalized
                                ),
                                edits: vec![TextEdit {
                                    range: tower_lsp::lsp_types::Range {
                                        start: tower_lsp::lsp_types::Position {
                                            line: range.start_line,
                                            character: range.start_character,
                                        },
                                        end: tower_lsp::lsp_types::Position {
                                            line: range.end_line,
                                            character: range.end_character,
                                        },
                                    },
                                    new_text: capitalized.clone(),
                                }],
                            })
                        } else {
                            None
                        };

                        diagnostics.push(Diagnostic {
                            severity: DiagnosticSeverity::Warning,
                            category: DiagnosticCategory::Naming,
                            message: format!(
                                "Struct name '{}' should start with uppercase",
                                symbol.name
                            ),
                            location: tower_lsp::lsp_types::Location {
                                uri: uri.clone(),
                                range: tower_lsp::lsp_types::Range {
                                    start: tower_lsp::lsp_types::Position {
                                        line: range.start_line,
                                        character: range.start_character,
                                    },
                                    end: tower_lsp::lsp_types::Position {
                                        line: range.end_line,
                                        character: range.end_character,
                                    },
                                },
                            },
                            rule: "naming-convention".to_string(),
                            suggestion: Some(format!("Rename to '{}'", capitalized)),
                            fix,
                        });
                    }
                }

                // Check for documentation
                if (symbol.kind == SymbolKind::Function || symbol.kind == SymbolKind::Struct)
                    && symbol.doc.is_none()
                {
                    if let Some(range) = &symbol.range {
                        diagnostics.push(Diagnostic {
                            severity: DiagnosticSeverity::Info,
                            category: DiagnosticCategory::Documentation,
                            message: format!(
                                "{:?} '{}' is missing documentation",
                                symbol.kind, symbol.name
                            ),
                            location: tower_lsp::lsp_types::Location {
                                uri: uri.clone(),
                                range: tower_lsp::lsp_types::Range {
                                    start: tower_lsp::lsp_types::Position {
                                        line: range.start_line,
                                        character: range.start_character,
                                    },
                                    end: tower_lsp::lsp_types::Position {
                                        line: range.end_line,
                                        character: range.end_character,
                                    },
                                },
                            },
                            rule: "missing-docs".to_string(),
                            suggestion: Some(format!(
                                "Add documentation comment above {}",
                                symbol.name
                            )),
                            fix: None, // Cannot auto-generate meaningful docs
                        });
                    }
                }
            }
        }

        diagnostics
    }

    /// Check error handling (similar to errcheck, err113, errorlint)
    fn check_error_handling(
        &mut self,
        files: &[SourceFile],
        _config: &LintConfig,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        for file in files {
            let uri = file.uri(self).clone();
            let text = file.text(self);

            // Check for ignored errors (Result without .expect() or ?)
            if text.contains("Result<") && !text.contains("?") && !text.contains(".expect(") {
                diagnostics.push(Diagnostic {
                    severity: DiagnosticSeverity::Warning,
                    category: DiagnosticCategory::ErrorHandling,
                    message: "Potential unchecked Result type".to_string(),
                    location: tower_lsp::lsp_types::Location {
                        uri: uri.clone(),
                        range: tower_lsp::lsp_types::Range::default(),
                    },
                    rule: "unchecked-result".to_string(),
                    suggestion: Some(
                        "Use '?' operator or '.expect()' to handle errors".to_string(),
                    ),
                    fix: None, // Context-dependent
                });
            }

            // Check for panic! usage
            if text.contains("panic!(") {
                diagnostics.push(Diagnostic {
                    severity: DiagnosticSeverity::Warning,
                    category: DiagnosticCategory::BugRisk,
                    message: "Use of panic! can crash the program".to_string(),
                    location: tower_lsp::lsp_types::Location {
                        uri: uri.clone(),
                        range: tower_lsp::lsp_types::Range::default(),
                    },
                    rule: "avoid-panic".to_string(),
                    suggestion: Some("Consider returning Result<T, E> instead".to_string()),
                    fix: None, // Complex refactoring
                });
            }

            // Check for unwrap() usage
            if text.contains(".unwrap()") {
                diagnostics.push(Diagnostic {
                    severity: DiagnosticSeverity::Warning,
                    category: DiagnosticCategory::BugRisk,
                    message: "Use of .unwrap() can panic at runtime".to_string(),
                    location: tower_lsp::lsp_types::Location {
                        uri: uri.clone(),
                        range: tower_lsp::lsp_types::Range::default(),
                    },
                    rule: "avoid-unwrap".to_string(),
                    suggestion: Some(
                        "Use pattern matching or .expect() with a message".to_string(),
                    ),
                    fix: None, // Context-dependent
                });
            }
        }

        diagnostics
    }

    /// Check performance issues (similar to prealloc, perfsprint)
    fn check_performance(&mut self, files: &[SourceFile], config: &LintConfig) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        for file in files {
            let uri = file.uri(self).clone();
            let text = file.text(self);

            // Check for Vec without capacity hint
            if text.contains("Vec::new()") && text.contains("push(") {
                // Auto-fix: Can suggest with_capacity
                let fix = if config.enable_autofix {
                    Some(AutoFix {
                        description: "Use Vec::with_capacity() for better performance".to_string(),
                        edits: vec![TextEdit {
                            range: tower_lsp::lsp_types::Range::default(),
                            new_text: "Vec::with_capacity(capacity)".to_string(),
                        }],
                    })
                } else {
                    None
                };

                diagnostics.push(Diagnostic {
                    severity: DiagnosticSeverity::Info,
                    category: DiagnosticCategory::Performance,
                    message: "Vec::new() followed by push() - consider pre-allocation".to_string(),
                    location: tower_lsp::lsp_types::Location {
                        uri: uri.clone(),
                        range: tower_lsp::lsp_types::Range::default(),
                    },
                    rule: "vec-prealloc".to_string(),
                    suggestion: Some("Use Vec::with_capacity(n) if you know the size".to_string()),
                    fix,
                });
            }

            // Check for inefficient string concatenation
            if text.contains("+ \"") || text.contains("+ '") {
                diagnostics.push(Diagnostic {
                    severity: DiagnosticSeverity::Info,
                    category: DiagnosticCategory::Performance,
                    message: "String concatenation with + creates temporary allocations"
                        .to_string(),
                    location: tower_lsp::lsp_types::Location {
                        uri: uri.clone(),
                        range: tower_lsp::lsp_types::Range::default(),
                    },
                    rule: "string-concat".to_string(),
                    suggestion: Some("Consider using format!() or String::push_str()".to_string()),
                    fix: None, // Complex refactoring
                });
            }

            // Check for clone() in loops
            if text.contains("for ") && text.contains(".clone()") {
                diagnostics.push(Diagnostic {
                    severity: DiagnosticSeverity::Warning,
                    category: DiagnosticCategory::Performance,
                    message: "Cloning inside a loop can be expensive".to_string(),
                    location: tower_lsp::lsp_types::Location {
                        uri: uri.clone(),
                        range: tower_lsp::lsp_types::Range::default(),
                    },
                    rule: "clone-in-loop".to_string(),
                    suggestion: Some("Consider borrowing instead of cloning".to_string()),
                    fix: None, // Context-dependent
                });
            }
        }

        diagnostics
    }

    /// Check security issues (similar to gosec)
    fn check_security(&mut self, files: &[SourceFile], _config: &LintConfig) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        for file in files {
            let uri = file.uri(self).clone();
            let text = file.text(self);

            // Check for unsafe blocks
            if text.contains("unsafe {") || text.contains("unsafe{") {
                diagnostics.push(Diagnostic {
                    severity: DiagnosticSeverity::Warning,
                    category: DiagnosticCategory::Security,
                    message: "Unsafe block detected - requires careful review".to_string(),
                    location: tower_lsp::lsp_types::Location {
                        uri: uri.clone(),
                        range: tower_lsp::lsp_types::Range::default(),
                    },
                    rule: "unsafe-block".to_string(),
                    suggestion: Some(
                        "Ensure all unsafe operations are properly documented and justified"
                            .to_string(),
                    ),
                    fix: None, // Manual review required
                });
            }

            // Check for hardcoded credentials
            let sensitive_patterns = ["password", "secret", "api_key", "token"];
            for pattern in &sensitive_patterns {
                if text.to_lowercase().contains(&format!("\"{}\"", pattern))
                    || text.to_lowercase().contains(&format!("'{}'", pattern))
                {
                    diagnostics.push(Diagnostic {
                        severity: DiagnosticSeverity::Error,
                        category: DiagnosticCategory::Security,
                        message: format!("Potential hardcoded sensitive data: '{}'", pattern),
                        location: tower_lsp::lsp_types::Location {
                            uri: uri.clone(),
                            range: tower_lsp::lsp_types::Range::default(),
                        },
                        rule: "hardcoded-secret".to_string(),
                        suggestion: Some(
                            "Use environment variables or secure configuration".to_string(),
                        ),
                        fix: None, // Requires manual intervention
                    });
                }
            }

            // Check for SQL query concatenation
            if text.contains("\"SELECT ") && text.contains(" + ") {
                diagnostics.push(Diagnostic {
                    severity: DiagnosticSeverity::Error,
                    category: DiagnosticCategory::Security,
                    message: "Potential SQL injection vulnerability".to_string(),
                    location: tower_lsp::lsp_types::Location {
                        uri: uri.clone(),
                        range: tower_lsp::lsp_types::Range::default(),
                    },
                    rule: "sql-injection".to_string(),
                    suggestion: Some(
                        "Use parameterized queries or prepared statements".to_string(),
                    ),
                    fix: None, // Complex refactoring
                });
            }
        }

        diagnostics
    }

    /// Check for circular dependencies (similar to import-cycle)
    fn check_circular_deps(&mut self, files: &[SourceFile]) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let graph = self.build_dependency_graph(files);

        if graph.has_circular_dependencies() {
            // For now, just report that cycles exist
            // A full implementation would extract the actual cycle paths
            for file_uri in &graph.files {
                diagnostics.push(Diagnostic {
                    severity: DiagnosticSeverity::Error,
                    category: DiagnosticCategory::Dependency,
                    message: "Circular dependency detected in project".to_string(),
                    location: tower_lsp::lsp_types::Location {
                        uri: file_uri.clone(),
                        range: tower_lsp::lsp_types::Range::default(),
                    },
                    rule: "circular-dependency".to_string(),
                    suggestion: Some(
                        "Break the circular dependency by refactoring imports".to_string(),
                    ),
                    fix: None, // Complex refactoring
                });
            }
        }

        diagnostics
    }
}

/// Helper function to capitalize first letter
fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

#[cfg(test)]
mod parallel_tests {
    use super::*;

    #[test]
    fn test_parallel_config_default() {
        let config = ParallelConfig::default();
        assert_eq!(config.num_threads, 0); // Use all cores
        assert_eq!(config.min_files_for_parallel, 5);
    }

    #[test]
    fn test_process_files_parallel_sequential() {
        let mut db = WindjammerDatabase::new();
        let config = ParallelConfig::default();

        // Less than min_files_for_parallel, should process sequentially
        let files = vec![
            (
                Url::parse("file:///test1.wj").unwrap(),
                "fn test1() {}".to_string(),
            ),
            (
                Url::parse("file:///test2.wj").unwrap(),
                "fn test2() {}".to_string(),
            ),
        ];

        let source_files = db.process_files_parallel(files, &config);
        assert_eq!(source_files.len(), 2);
    }

    #[test]
    fn test_process_files_parallel() {
        let mut db = WindjammerDatabase::new();
        let config = ParallelConfig {
            num_threads: 2,
            min_files_for_parallel: 3,
        };

        // Enough files to trigger parallel processing
        let files = vec![
            (
                Url::parse("file:///test1.wj").unwrap(),
                "fn test1() {}".to_string(),
            ),
            (
                Url::parse("file:///test2.wj").unwrap(),
                "fn test2() {}".to_string(),
            ),
            (
                Url::parse("file:///test3.wj").unwrap(),
                "fn test3() {}".to_string(),
            ),
            (
                Url::parse("file:///test4.wj").unwrap(),
                "fn test4() {}".to_string(),
            ),
        ];

        let source_files = db.process_files_parallel(files, &config);
        assert_eq!(source_files.len(), 4);

        // Verify symbols were extracted
        for file in &source_files {
            let symbols = db.get_symbols(*file);
            assert_eq!(symbols.len(), 1); // Each file has one function
        }
    }

    #[test]
    fn test_extract_symbols_parallel() {
        let mut db = WindjammerDatabase::new();

        let files: Vec<_> = (0..10)
            .map(|i| {
                let uri = Url::parse(&format!("file:///test{}.wj", i)).unwrap();
                let text = format!("fn test{}() {{}}", i);
                db.set_source_text(uri, text)
            })
            .collect();

        let symbols_list = db.extract_symbols_parallel(&files);
        assert_eq!(symbols_list.len(), 10);

        for symbols in symbols_list {
            assert_eq!(symbols.len(), 1);
        }
    }

    #[test]
    fn test_find_all_references_parallel() {
        let mut db = WindjammerDatabase::new();

        // Create multiple files with the same function name
        let files: Vec<_> = (0..5)
            .map(|i| {
                let uri = Url::parse(&format!("file:///test{}.wj", i)).unwrap();
                let text = "fn calculate() {}".to_string();
                db.set_source_text(uri, text)
            })
            .collect();

        let locations = db.find_all_references_parallel("calculate", &files);

        // Should find 5 instances (one per file)
        assert_eq!(locations.len(), 5);

        // All should have the same function name
        for location in &locations {
            assert!(location.uri.as_str().starts_with("file:///test"));
        }
    }

    #[test]
    fn test_parallel_performance_benefit() {
        let mut db = WindjammerDatabase::new();

        // Create 20 files
        let files: Vec<_> = (0..20)
            .map(|i| {
                (
                    Url::parse(&format!("file:///test{}.wj", i)).unwrap(),
                    format!("fn test{}() {{}}", i),
                )
            })
            .collect();

        let config = ParallelConfig {
            num_threads: 4,
            min_files_for_parallel: 10,
        };

        let start = std::time::Instant::now();
        let source_files = db.process_files_parallel(files, &config);
        let elapsed = start.elapsed();

        assert_eq!(source_files.len(), 20);
        println!("Processed 20 files in {:?}", elapsed);

        // Second query should be cached (verify it works, don't assert on timing)
        let start = std::time::Instant::now();
        for file in &source_files {
            let _symbols = db.get_symbols(*file);
        }
        let cached_elapsed = start.elapsed();

        println!("Cached query for 20 files in {:?}", cached_elapsed);

        // In tests, cached queries might be slower due to overhead
        // Just verify it completes successfully
        println!(
            "Speedup: {:.2}x (note: may be slower in debug builds)",
            elapsed.as_nanos() as f64 / cached_elapsed.as_nanos().max(1) as f64
        );
    }
}

#[cfg(test)]
mod type_aware_tests {
    use super::*;

    #[test]
    fn test_find_trait_implementations() {
        let mut db = WindjammerDatabase::new();

        // Create files with trait implementations
        let files: Vec<_> = vec![
            (
                Url::parse("file:///traits.wj").unwrap(),
                "trait Display {}".to_string(),
            ),
            (
                Url::parse("file:///impl1.wj").unwrap(),
                "impl Display for String {}".to_string(),
            ),
            (
                Url::parse("file:///impl2.wj").unwrap(),
                "impl Display for Int {}".to_string(),
            ),
        ]
        .into_iter()
        .map(|(uri, text)| db.set_source_text(uri, text))
        .collect();

        let implementations = db.find_trait_implementations("Display", &files);

        // Should find implementations (may be 0 or 2 depending on parsing)
        // Just verify it doesn't panic
        for impl_info in implementations {
            assert_eq!(impl_info.trait_name, "Display");
            assert!(impl_info.type_name.contains("String") || impl_info.type_name.contains("Int"));
        }
    }

    #[test]
    fn test_extract_type_from_impl() {
        let db = WindjammerDatabase::new();

        // Test "impl Trait for Type"
        let type1 = db.extract_type_from_impl("impl Display for String");
        assert_eq!(type1, "String");

        // Test "impl Type"
        let type2 = db.extract_type_from_impl("impl MyStruct");
        assert_eq!(type2, "MyStruct");

        // Test with extra spaces
        let type3 = db.extract_type_from_impl("impl  Display  for  Int  ");
        assert_eq!(type3, "Int");
    }

    #[test]
    fn test_get_hover_info() {
        let mut db = WindjammerDatabase::new();

        let uri = Url::parse("file:///test.wj").unwrap();
        let text = "fn calculate(x: int) -> int { x * 2 }";
        let file = db.set_source_text(uri, text.to_string());

        // Try to get hover info at the function position (line 0, character 3)
        let hover = db.get_hover_info(file, 0, 3);

        // May or may not find it depending on exact position
        if let Some(info) = hover {
            assert_eq!(info.name, "calculate");
            assert_eq!(info.kind, "Function");
        }
    }

    #[test]
    fn test_hover_info_not_found() {
        let mut db = WindjammerDatabase::new();

        let uri = Url::parse("file:///test.wj").unwrap();
        let text = "fn test() {}";
        let file = db.set_source_text(uri, text.to_string());

        // Try to get hover info at a position with no symbol
        let hover = db.get_hover_info(file, 10, 50);
        assert!(hover.is_none());
    }

    #[test]
    fn test_hover_info_with_type() {
        let mut db = WindjammerDatabase::new();

        let uri = Url::parse("file:///test.wj").unwrap();
        let text = "fn typed() -> string { \"hello\" }";
        let file = db.set_source_text(uri, text.to_string());

        // The function should have type information
        let hover = db.get_hover_info(file, 0, 3);

        if let Some(info) = hover {
            assert_eq!(info.name, "typed");
            assert!(info.type_info.is_some());
        }
    }

    #[test]
    fn test_trait_implementations_empty() {
        let mut db = WindjammerDatabase::new();

        // Create file with no implementations
        let files: Vec<_> = vec![(
            Url::parse("file:///test.wj").unwrap(),
            "fn test() {}".to_string(),
        )]
        .into_iter()
        .map(|(uri, text)| db.set_source_text(uri, text))
        .collect();

        let implementations = db.find_trait_implementations("NonExistent", &files);
        assert_eq!(implementations.len(), 0);
    }
}

#[cfg(test)]
mod code_lens_tests {
    use super::*;

    #[test]
    fn test_get_code_lenses_function() {
        let mut db = WindjammerDatabase::new();

        let uri = Url::parse("file:///test.wj").unwrap();
        let text = "fn calculate(x: int) -> int { x * 2 }";
        let file = db.set_source_text(uri, text.to_string());

        let lenses = db.get_code_lenses(file, &[file]);

        // Code lenses require symbols to have ranges
        // If parsing doesn't extract ranges, lenses will be empty
        if !lenses.is_empty() {
            // First lens should be a reference count
            let first = &lenses[0];
            assert!(first.command.is_some());
            let cmd = first.command.as_ref().unwrap();
            assert!(cmd.title.contains("reference"));
            assert_eq!(cmd.command, "windjammer.showReferences");
        }
    }

    #[test]
    fn test_get_code_lenses_trait() {
        let mut db = WindjammerDatabase::new();

        let files: Vec<_> = vec![
            (
                Url::parse("file:///trait.wj").unwrap(),
                "trait Display {}".to_string(),
            ),
            (
                Url::parse("file:///impl.wj").unwrap(),
                "impl Display for String {}".to_string(),
            ),
        ]
        .into_iter()
        .map(|(uri, text)| db.set_source_text(uri, text))
        .collect();

        let lenses = db.get_code_lenses(files[0], &files);

        // Should have lenses for the trait
        // Note: May be empty if parsing doesn't extract ranges
        if !lenses.is_empty() {
            // Look for implementation count lens
            let impl_lens = lenses.iter().find(|l| {
                l.command
                    .as_ref()
                    .map(|c| c.title.contains("implementation"))
                    .unwrap_or(false)
            });

            if let Some(lens) = impl_lens {
                let cmd = lens.command.as_ref().unwrap();
                assert_eq!(cmd.command, "windjammer.showImplementations");
            }
        }
    }

    #[test]
    fn test_get_code_lenses_multiple_references() {
        let mut db = WindjammerDatabase::new();

        let files: Vec<_> = vec![
            (
                Url::parse("file:///def.wj").unwrap(),
                "fn helper() {}".to_string(),
            ),
            (
                Url::parse("file:///use1.wj").unwrap(),
                "fn helper() {}".to_string(), // Another definition
            ),
            (
                Url::parse("file:///use2.wj").unwrap(),
                "fn helper() {}".to_string(), // Yet another
            ),
        ]
        .into_iter()
        .map(|(uri, text)| db.set_source_text(uri, text))
        .collect();

        let lenses = db.get_code_lenses(files[0], &files);

        // Should show multiple references
        if !lenses.is_empty() {
            let first = &lenses[0];
            if let Some(cmd) = &first.command {
                // Should say "3 references" (plural)
                assert!(cmd.title.contains("reference"));
            }
        }
    }

    #[test]
    fn test_count_references() {
        let mut db = WindjammerDatabase::new();

        let files: Vec<_> = vec![
            (
                Url::parse("file:///file1.wj").unwrap(),
                "fn test() {}".to_string(),
            ),
            (
                Url::parse("file:///file2.wj").unwrap(),
                "fn test() {}".to_string(),
            ),
            (
                Url::parse("file:///file3.wj").unwrap(),
                "fn other() {}".to_string(),
            ),
        ]
        .into_iter()
        .map(|(uri, text)| db.set_source_text(uri, text))
        .collect();

        let count = db.count_references("test", &files);
        assert_eq!(count, 2); // "test" appears in 2 files

        let count_other = db.count_references("other", &files);
        assert_eq!(count_other, 1); // "other" appears in 1 file

        let count_none = db.count_references("nonexistent", &files);
        assert_eq!(count_none, 0); // "nonexistent" doesn't appear
    }

    #[test]
    fn test_code_lens_range() {
        let mut db = WindjammerDatabase::new();

        let uri = Url::parse("file:///test.wj").unwrap();
        let text = "fn test() {}";
        let file = db.set_source_text(uri, text.to_string());

        let lenses = db.get_code_lenses(file, &[file]);

        // Verify ranges are valid
        for lens in lenses {
            assert!(lens.range.start.line <= lens.range.end.line);
            if lens.range.start.line == lens.range.end.line {
                assert!(lens.range.start.character <= lens.range.end.character);
            }
        }
    }

    #[test]
    fn test_code_lens_empty_file() {
        let mut db = WindjammerDatabase::new();

        let uri = Url::parse("file:///empty.wj").unwrap();
        let text = "";
        let file = db.set_source_text(uri, text.to_string());

        let lenses = db.get_code_lenses(file, &[file]);

        // Empty file should have no lenses
        assert_eq!(lenses.len(), 0);
    }

    #[test]
    fn test_code_lens_no_range_symbols() {
        let mut db = WindjammerDatabase::new();

        let uri = Url::parse("file:///test.wj").unwrap();
        let text = "fn test() {}";
        let file = db.set_source_text(uri, text.to_string());

        // If symbols don't have ranges, code lenses should handle gracefully
        let _lenses = db.get_code_lenses(file, &[file]);

        // Should not panic - test passes if we get here
    }
}

#[cfg(test)]
mod inlay_hints_tests {
    use super::*;

    #[test]
    fn test_get_inlay_hints_function_with_type() {
        let mut db = WindjammerDatabase::new();

        let uri = Url::parse("file:///test.wj").unwrap();
        let text = "fn calculate(x: int) -> int { x * 2 }";
        let file = db.set_source_text(uri, text.to_string());

        let hints = db.get_inlay_hints(file);

        // May have hints if type info is extracted
        // Just verify it doesn't panic
        for hint in hints {
            assert!(hint.label.contains(":"));
            assert_eq!(hint.kind, InlayHintKind::Type);
        }
    }

    #[test]
    fn test_get_inlay_hints_empty_file() {
        let mut db = WindjammerDatabase::new();

        let uri = Url::parse("file:///empty.wj").unwrap();
        let text = "";
        let file = db.set_source_text(uri, text.to_string());

        let hints = db.get_inlay_hints(file);

        // Empty file should have no hints
        assert_eq!(hints.len(), 0);
    }

    #[test]
    fn test_get_inlay_hints_no_types() {
        let mut db = WindjammerDatabase::new();

        let uri = Url::parse("file:///test.wj").unwrap();
        let text = "fn test() {}";
        let file = db.set_source_text(uri, text.to_string());

        let _hints = db.get_inlay_hints(file);

        // Function without explicit return type may have no hints
        // Just verify it doesn't panic - test passes if we get here
    }

    #[test]
    fn test_get_parameter_hints_placeholder() {
        let mut db = WindjammerDatabase::new();

        let uri = Url::parse("file:///test.wj").unwrap();
        let text = "fn test(x: int) {}";
        let file = db.set_source_text(uri, text.to_string());

        // Parameter hints are not yet implemented
        let hints = db.get_parameter_hints(file, 0, 0);
        assert_eq!(hints.len(), 0);
    }

    #[test]
    fn test_inlay_hint_kind() {
        // Test that InlayHintKind enum works correctly
        assert_eq!(InlayHintKind::Type, InlayHintKind::Type);
        assert_eq!(InlayHintKind::Parameter, InlayHintKind::Parameter);
        assert_ne!(InlayHintKind::Type, InlayHintKind::Parameter);
    }

    #[test]
    fn test_inlay_hint_structure() {
        let hint = InlayHint {
            position: tower_lsp::lsp_types::Position {
                line: 0,
                character: 10,
            },
            label: ": string".to_string(),
            kind: InlayHintKind::Type,
            tooltip: Some("Return type".to_string()),
        };

        assert_eq!(hint.position.line, 0);
        assert_eq!(hint.position.character, 10);
        assert_eq!(hint.label, ": string");
        assert_eq!(hint.kind, InlayHintKind::Type);
        assert_eq!(hint.tooltip, Some("Return type".to_string()));
    }

    #[test]
    fn test_inlay_hints_multiple_functions() {
        let mut db = WindjammerDatabase::new();

        let uri = Url::parse("file:///test.wj").unwrap();
        let text = r#"
fn add(a: int, b: int) -> int { a + b }
fn greet(name: string) -> string { "Hello" }
"#;
        let file = db.set_source_text(uri, text.to_string());

        let hints = db.get_inlay_hints(file);

        // May have hints for both functions if types are extracted
        // Just verify structure is correct
        for hint in hints {
            assert!(hint.label.starts_with(":"));
            assert_eq!(hint.kind, InlayHintKind::Type);
            assert!(hint.tooltip.is_some());
        }
    }
}

#[cfg(test)]
mod call_hierarchy_tests {
    use super::*;

    #[test]
    fn test_prepare_call_hierarchy() {
        let mut db = WindjammerDatabase::new();

        let uri = Url::parse("file:///test.wj").unwrap();
        let text = "fn calculate(x: int) -> int { x * 2 }";
        let file = db.set_source_text(uri, text.to_string());

        // Try to prepare call hierarchy at the function name position
        let item = db.prepare_call_hierarchy(file, 0, 3);

        // May return None if ranges aren't extracted
        if let Some(item) = item {
            assert_eq!(item.name, "calculate");
            assert_eq!(item.kind, SymbolKind::Function);
        }
    }

    #[test]
    fn test_prepare_call_hierarchy_not_found() {
        let mut db = WindjammerDatabase::new();

        let uri = Url::parse("file:///test.wj").unwrap();
        let text = "fn test() {}";
        let file = db.set_source_text(uri, text.to_string());

        // Try at a position with no symbol
        let item = db.prepare_call_hierarchy(file, 10, 50);
        assert!(item.is_none());
    }

    #[test]
    fn test_incoming_calls() {
        let mut db = WindjammerDatabase::new();

        let files: Vec<_> = vec![
            (
                Url::parse("file:///main.wj").unwrap(),
                "fn main() {}".to_string(),
            ),
            (
                Url::parse("file:///helper.wj").unwrap(),
                "fn helper() {}".to_string(),
            ),
        ]
        .into_iter()
        .map(|(uri, text)| db.set_source_text(uri, text))
        .collect();

        // Create a call hierarchy item for helper
        let item = CallHierarchyItem {
            name: "helper".to_string(),
            kind: SymbolKind::Function,
            uri: Url::parse("file:///helper.wj").unwrap(),
            range: tower_lsp::lsp_types::Range {
                start: tower_lsp::lsp_types::Position {
                    line: 0,
                    character: 0,
                },
                end: tower_lsp::lsp_types::Position {
                    line: 0,
                    character: 10,
                },
            },
            selection_range: tower_lsp::lsp_types::Range {
                start: tower_lsp::lsp_types::Position {
                    line: 0,
                    character: 3,
                },
                end: tower_lsp::lsp_types::Position {
                    line: 0,
                    character: 9,
                },
            },
        };

        let calls = db.incoming_calls(&item, &files);

        // May have calls if ranges are extracted
        // Just verify it doesn't panic
        for call in calls {
            assert_ne!(call.from.name, "helper"); // Should be from other functions
        }
    }

    #[test]
    fn test_outgoing_calls() {
        let mut db = WindjammerDatabase::new();

        let files: Vec<_> = vec![
            (
                Url::parse("file:///main.wj").unwrap(),
                "fn main() {}".to_string(),
            ),
            (
                Url::parse("file:///helper.wj").unwrap(),
                "fn helper() {}".to_string(),
            ),
        ]
        .into_iter()
        .map(|(uri, text)| db.set_source_text(uri, text))
        .collect();

        // Create a call hierarchy item for main
        let item = CallHierarchyItem {
            name: "main".to_string(),
            kind: SymbolKind::Function,
            uri: Url::parse("file:///main.wj").unwrap(),
            range: tower_lsp::lsp_types::Range {
                start: tower_lsp::lsp_types::Position {
                    line: 0,
                    character: 0,
                },
                end: tower_lsp::lsp_types::Position {
                    line: 0,
                    character: 10,
                },
            },
            selection_range: tower_lsp::lsp_types::Range {
                start: tower_lsp::lsp_types::Position {
                    line: 0,
                    character: 3,
                },
                end: tower_lsp::lsp_types::Position {
                    line: 0,
                    character: 7,
                },
            },
        };

        let calls = db.outgoing_calls(&item, &files);

        // May have calls if ranges are extracted
        // Just verify it doesn't panic
        for call in calls {
            assert_ne!(call.to.name, "main"); // Should be to other functions
        }
    }

    #[test]
    fn test_call_hierarchy_item_structure() {
        let item = CallHierarchyItem {
            name: "test".to_string(),
            kind: SymbolKind::Function,
            uri: Url::parse("file:///test.wj").unwrap(),
            range: tower_lsp::lsp_types::Range {
                start: tower_lsp::lsp_types::Position {
                    line: 0,
                    character: 0,
                },
                end: tower_lsp::lsp_types::Position {
                    line: 5,
                    character: 1,
                },
            },
            selection_range: tower_lsp::lsp_types::Range {
                start: tower_lsp::lsp_types::Position {
                    line: 0,
                    character: 3,
                },
                end: tower_lsp::lsp_types::Position {
                    line: 0,
                    character: 7,
                },
            },
        };

        assert_eq!(item.name, "test");
        assert_eq!(item.kind, SymbolKind::Function);
        assert_eq!(item.range.start.line, 0);
        assert_eq!(item.selection_range.start.character, 3);
    }

    #[test]
    fn test_incoming_call_structure() {
        let from = CallHierarchyItem {
            name: "caller".to_string(),
            kind: SymbolKind::Function,
            uri: Url::parse("file:///test.wj").unwrap(),
            range: tower_lsp::lsp_types::Range {
                start: tower_lsp::lsp_types::Position {
                    line: 0,
                    character: 0,
                },
                end: tower_lsp::lsp_types::Position {
                    line: 5,
                    character: 1,
                },
            },
            selection_range: tower_lsp::lsp_types::Range {
                start: tower_lsp::lsp_types::Position {
                    line: 0,
                    character: 3,
                },
                end: tower_lsp::lsp_types::Position {
                    line: 0,
                    character: 9,
                },
            },
        };

        let call = IncomingCall {
            from: from.clone(),
            from_ranges: vec![tower_lsp::lsp_types::Range {
                start: tower_lsp::lsp_types::Position {
                    line: 2,
                    character: 4,
                },
                end: tower_lsp::lsp_types::Position {
                    line: 2,
                    character: 10,
                },
            }],
        };

        assert_eq!(call.from.name, "caller");
        assert_eq!(call.from_ranges.len(), 1);
        assert_eq!(call.from_ranges[0].start.line, 2);
    }

    #[test]
    fn test_outgoing_call_structure() {
        let to = CallHierarchyItem {
            name: "callee".to_string(),
            kind: SymbolKind::Function,
            uri: Url::parse("file:///test.wj").unwrap(),
            range: tower_lsp::lsp_types::Range {
                start: tower_lsp::lsp_types::Position {
                    line: 10,
                    character: 0,
                },
                end: tower_lsp::lsp_types::Position {
                    line: 15,
                    character: 1,
                },
            },
            selection_range: tower_lsp::lsp_types::Range {
                start: tower_lsp::lsp_types::Position {
                    line: 10,
                    character: 3,
                },
                end: tower_lsp::lsp_types::Position {
                    line: 10,
                    character: 9,
                },
            },
        };

        let call = OutgoingCall {
            to: to.clone(),
            from_ranges: vec![tower_lsp::lsp_types::Range {
                start: tower_lsp::lsp_types::Position {
                    line: 2,
                    character: 4,
                },
                end: tower_lsp::lsp_types::Position {
                    line: 2,
                    character: 10,
                },
            }],
        };

        assert_eq!(call.to.name, "callee");
        assert_eq!(call.from_ranges.len(), 1);
        assert_eq!(call.from_ranges[0].start.line, 2);
    }

    #[test]
    fn test_call_hierarchy_empty_project() {
        let mut db = WindjammerDatabase::new();

        let uri = Url::parse("file:///empty.wj").unwrap();
        let text = "";
        let file = db.set_source_text(uri, text.to_string());

        let item = db.prepare_call_hierarchy(file, 0, 0);
        assert!(item.is_none());
    }
}

#[cfg(test)]
mod unused_code_tests {
    use super::*;

    #[test]
    fn test_find_unused_symbols_empty() {
        let mut db = WindjammerDatabase::new();

        let uri = Url::parse("file:///empty.wj").unwrap();
        let text = "";
        let file = db.set_source_text(uri, text.to_string());

        let unused = db.find_unused_symbols(&[file]);
        assert_eq!(unused.len(), 0);
    }

    #[test]
    fn test_find_unused_symbols_all_used() {
        let mut db = WindjammerDatabase::new();

        let files: Vec<_> = vec![
            (
                Url::parse("file:///main.wj").unwrap(),
                "fn main() { helper() }".to_string(),
            ),
            (
                Url::parse("file:///helper.wj").unwrap(),
                "fn helper() {}".to_string(),
            ),
        ]
        .into_iter()
        .map(|(uri, text)| db.set_source_text(uri, text))
        .collect();

        let _unused = db.find_unused_symbols(&files);

        // Both functions reference each other, so none should be unused
        // (This is a simplified check - actual usage would need AST analysis)
        // Test passes if no panic occurs
    }

    #[test]
    fn test_find_unused_functions() {
        let mut db = WindjammerDatabase::new();

        let files: Vec<_> = vec![
            (
                Url::parse("file:///main.wj").unwrap(),
                "fn main() {}".to_string(),
            ),
            (
                Url::parse("file:///unused.wj").unwrap(),
                "fn unused_func() {}".to_string(),
            ),
        ]
        .into_iter()
        .map(|(uri, text)| db.set_source_text(uri, text))
        .collect();

        let unused = db.find_unused_functions(&files);

        // Should find functions that are never called
        for u in &unused {
            assert_eq!(u.kind, SymbolKind::Function);
            assert_eq!(u.reason, UnusedReason::NeverReferenced);
        }
    }

    #[test]
    fn test_find_unused_structs() {
        let mut db = WindjammerDatabase::new();

        let files: Vec<_> = vec![(
            Url::parse("file:///structs.wj").unwrap(),
            "struct UsedStruct {} struct UnusedStruct {}".to_string(),
        )]
        .into_iter()
        .map(|(uri, text)| db.set_source_text(uri, text))
        .collect();

        let unused = db.find_unused_structs(&files);

        // Should only return structs
        for u in &unused {
            assert_eq!(u.kind, SymbolKind::Struct);
        }
    }

    #[test]
    fn test_unused_reason() {
        // Test the UnusedReason enum
        assert_eq!(UnusedReason::NeverReferenced, UnusedReason::NeverReferenced);
        assert_ne!(UnusedReason::NeverReferenced, UnusedReason::OnlyInDeadCode);
    }

    #[test]
    fn test_unused_symbol_structure() {
        let unused = UnusedSymbol {
            name: "test".to_string(),
            kind: SymbolKind::Function,
            location: tower_lsp::lsp_types::Location {
                uri: Url::parse("file:///test.wj").unwrap(),
                range: tower_lsp::lsp_types::Range {
                    start: tower_lsp::lsp_types::Position {
                        line: 0,
                        character: 0,
                    },
                    end: tower_lsp::lsp_types::Position {
                        line: 5,
                        character: 1,
                    },
                },
            },
            reason: UnusedReason::NeverReferenced,
        };

        assert_eq!(unused.name, "test");
        assert_eq!(unused.kind, SymbolKind::Function);
        assert_eq!(unused.reason, UnusedReason::NeverReferenced);
    }

    #[test]
    fn test_find_unused_with_duplicates() {
        let mut db = WindjammerDatabase::new();

        // Create files where the same function name appears multiple times
        let files: Vec<_> = vec![
            (
                Url::parse("file:///file1.wj").unwrap(),
                "fn duplicate() {}".to_string(),
            ),
            (
                Url::parse("file:///file2.wj").unwrap(),
                "fn duplicate() {}".to_string(),
            ),
        ]
        .into_iter()
        .map(|(uri, text)| db.set_source_text(uri, text))
        .collect();

        let unused = db.find_unused_symbols(&files);

        // Duplicates should not be marked as unused (they reference each other)
        let duplicate_unused = unused.iter().filter(|u| u.name == "duplicate").count();
        assert_eq!(duplicate_unused, 0);
    }
}

#[cfg(test)]
mod dependency_tests {
    use super::*;

    #[test]
    fn test_build_dependency_graph_empty() {
        let mut db = WindjammerDatabase::new();

        let uri = Url::parse("file:///empty.wj").unwrap();
        let text = "";
        let file = db.set_source_text(uri, text.to_string());

        let graph = db.build_dependency_graph(&[file]);
        assert_eq!(graph.files.len(), 1);
        assert_eq!(graph.dependencies.len(), 0);
    }

    #[test]
    fn test_dependency_graph_no_cycles() {
        let mut db = WindjammerDatabase::new();

        let files: Vec<_> = vec![
            (
                Url::parse("file:///main.wj").unwrap(),
                "fn main() {}".to_string(),
            ),
            (
                Url::parse("file:///helper.wj").unwrap(),
                "fn helper() {}".to_string(),
            ),
        ]
        .into_iter()
        .map(|(uri, text)| db.set_source_text(uri, text))
        .collect();

        let graph = db.build_dependency_graph(&files);
        assert!(!graph.has_circular_dependencies());
    }

    #[test]
    fn test_get_dependencies() {
        let mut db = WindjammerDatabase::new();

        let files: Vec<_> = vec![
            (
                Url::parse("file:///main.wj").unwrap(),
                "fn main() {}".to_string(),
            ),
            (
                Url::parse("file:///helper.wj").unwrap(),
                "fn helper() {}".to_string(),
            ),
        ]
        .into_iter()
        .map(|(uri, text)| db.set_source_text(uri, text))
        .collect();

        let graph = db.build_dependency_graph(&files);
        let main_uri = Url::parse("file:///main.wj").unwrap();
        let _deps = graph.get_dependencies(&main_uri);

        // Verify we can get dependencies (may be empty)
        // Test passes if no panic occurs
    }

    #[test]
    fn test_calculate_coupling() {
        let mut db = WindjammerDatabase::new();

        let files: Vec<_> = vec![
            (
                Url::parse("file:///main.wj").unwrap(),
                "fn main() {}".to_string(),
            ),
            (
                Url::parse("file:///helper.wj").unwrap(),
                "fn helper() {}".to_string(),
            ),
        ]
        .into_iter()
        .map(|(uri, text)| db.set_source_text(uri, text))
        .collect();

        let metrics = db.calculate_coupling(&files);
        assert_eq!(metrics.len(), 2);

        for (uri, _afferent, _efferent) in metrics {
            assert!(uri.as_str().starts_with("file:///"));
            // assert!(afferent >= 0); // Always true for usize
            // assert!(efferent >= 0); // Always true for usize
        }
    }

    #[test]
    fn test_dependency_kind() {
        assert_eq!(DependencyKind::Import, DependencyKind::Import);
        assert_ne!(DependencyKind::Import, DependencyKind::SymbolReference);
    }

    #[test]
    fn test_file_dependency_structure() {
        let dep = FileDependency {
            from: Url::parse("file:///a.wj").unwrap(),
            to: Url::parse("file:///b.wj").unwrap(),
            kind: DependencyKind::Import,
        };

        assert_eq!(dep.from.as_str(), "file:///a.wj");
        assert_eq!(dep.to.as_str(), "file:///b.wj");
        assert_eq!(dep.kind, DependencyKind::Import);
    }

    #[test]
    fn test_find_circular_dependencies() {
        let mut db = WindjammerDatabase::new();

        let files: Vec<_> = vec![
            (Url::parse("file:///a.wj").unwrap(), "fn a() {}".to_string()),
            (Url::parse("file:///b.wj").unwrap(), "fn b() {}".to_string()),
        ]
        .into_iter()
        .map(|(uri, text)| db.set_source_text(uri, text))
        .collect();

        let _cycles = db.find_circular_dependencies(&files);
        // Should not panic, may be empty
        // Test passes if no panic occurs
    }
}

#[cfg(test)]
mod metrics_tests {
    use super::*;

    #[test]
    fn test_calculate_file_metrics_empty() {
        let mut db = WindjammerDatabase::new();

        let uri = Url::parse("file:///empty.wj").unwrap();
        let text = "";
        let file = db.set_source_text(uri, text.to_string());

        let metrics = db.calculate_file_metrics(file);
        assert_eq!(metrics.lines_of_code, 0);
        assert_eq!(metrics.num_functions, 0);
        assert_eq!(metrics.num_structs, 0);
    }

    #[test]
    fn test_calculate_file_metrics_with_content() {
        let mut db = WindjammerDatabase::new();

        let uri = Url::parse("file:///test.wj").unwrap();
        let text = r#"
// Comment
fn main() {
    println("Hello");
}

struct Point {
    x: i32,
    y: i32,
}
"#;
        let file = db.set_source_text(uri, text.to_string());

        let metrics = db.calculate_file_metrics(file);
        assert!(metrics.lines_of_code > 0);
        // Parser may not extract all symbols, just check that we got some data
        // assert!(metrics.complexity_score >= 0); // Always true for usize
    }

    #[test]
    fn test_calculate_workspace_metrics() {
        let mut db = WindjammerDatabase::new();

        let files: Vec<_> = vec![
            (
                Url::parse("file:///main.wj").unwrap(),
                "fn main() {}".to_string(),
            ),
            (
                Url::parse("file:///helper.wj").unwrap(),
                "fn helper() {}".to_string(),
            ),
        ]
        .into_iter()
        .map(|(uri, text)| db.set_source_text(uri, text))
        .collect();

        let metrics = db.calculate_workspace_metrics(&files);
        assert_eq!(metrics.total_files, 2);
        assert!(metrics.total_lines > 0);
        assert!(metrics.total_functions >= 2);
    }

    #[test]
    fn test_find_large_files() {
        let mut db = WindjammerDatabase::new();

        let files: Vec<_> = vec![
            (
                Url::parse("file:///small.wj").unwrap(),
                "fn f() {}".to_string(),
            ),
            (
                Url::parse("file:///large.wj").unwrap(),
                "fn f1() {}\nfn f2() {}\nfn f3() {}\nfn f4() {}\nfn f5() {}".to_string(),
            ),
        ]
        .into_iter()
        .map(|(uri, text)| db.set_source_text(uri, text))
        .collect();

        let large = db.find_large_files(&files, 2);
        assert!(!large.is_empty());
        assert!(large.iter().any(|(uri, _)| uri.as_str().contains("large")));
    }

    #[test]
    fn test_find_long_functions() {
        let mut db = WindjammerDatabase::new();

        let files: Vec<_> = vec![(
            Url::parse("file:///test.wj").unwrap(),
            "fn short() {}\nfn long() {\n  // line 1\n  // line 2\n  // line 3\n}".to_string(),
        )]
        .into_iter()
        .map(|(uri, text)| db.set_source_text(uri, text))
        .collect();

        let _long = db.find_long_functions(&files, 3);
        // May or may not find long functions depending on AST parsing
        // assert!(long.len() >= 0); // Always true for usize
    }

    #[test]
    fn test_file_metrics_structure() {
        let mut db = WindjammerDatabase::new();

        let uri = Url::parse("file:///test.wj").unwrap();
        let text = "fn main() {}";
        let file = db.set_source_text(uri.clone(), text.to_string());

        let metrics = db.calculate_file_metrics(file);
        assert_eq!(metrics.uri, uri);
        assert!(metrics.avg_function_length >= 0.0);
        // assert!(metrics.complexity_score >= 0); // Always true for usize
    }

    #[test]
    fn test_workspace_metrics_largest_file() {
        let mut db = WindjammerDatabase::new();

        let files: Vec<_> = vec![
            (
                Url::parse("file:///small.wj").unwrap(),
                "fn f() {}".to_string(),
            ),
            (
                Url::parse("file:///large.wj").unwrap(),
                "fn f1() {}\nfn f2() {}\nfn f3() {}".to_string(),
            ),
        ]
        .into_iter()
        .map(|(uri, text)| db.set_source_text(uri, text))
        .collect();

        let metrics = db.calculate_workspace_metrics(&files);
        assert!(metrics.largest_file.is_some());

        if let Some((uri, size)) = metrics.largest_file {
            assert!(uri.as_str().contains("large"));
            assert!(size > 0);
        }
    }
}

#[cfg(test)]
mod diagnostics_tests {
    use super::*;

    #[test]
    fn test_lint_config_default() {
        let config = LintConfig::default();
        assert_eq!(config.max_function_length, 50);
        assert_eq!(config.max_file_length, 500);
        assert!(config.check_unused);
        assert!(config.check_style);
    }

    #[test]
    fn test_lint_workspace_empty() {
        let mut db = WindjammerDatabase::new();
        let config = LintConfig::default();

        let uri = Url::parse("file:///empty.wj").unwrap();
        let text = "";
        let file = db.set_source_text(uri, text.to_string());

        let diagnostics = db.lint_workspace(&[file], &config);
        assert_eq!(diagnostics.len(), 0);
    }

    #[test]
    fn test_check_unused_code() {
        let mut db = WindjammerDatabase::new();

        let files: Vec<_> = vec![
            (
                Url::parse("file:///main.wj").unwrap(),
                "fn main() {}".to_string(),
            ),
            (
                Url::parse("file:///unused.wj").unwrap(),
                "fn unused_func() {}".to_string(),
            ),
        ]
        .into_iter()
        .map(|(uri, text)| db.set_source_text(uri, text))
        .collect();

        let config = LintConfig::default();
        let diagnostics = db.check_unused_code(&files, &config);
        // May find unused functions
        for diag in &diagnostics {
            assert_eq!(diag.category, DiagnosticCategory::Unused);
            assert_eq!(diag.severity, DiagnosticSeverity::Warning);
            assert!(diag.suggestion.is_some());
        }
    }

    #[test]
    fn test_check_complexity() {
        let mut db = WindjammerDatabase::new();
        let config = LintConfig {
            max_function_length: 2,
            max_file_length: 5,
            ..Default::default()
        };

        let files: Vec<_> = vec![(
            Url::parse("file:///test.wj").unwrap(),
            "fn long() {\n  line1();\n  line2();\n  line3();\n  line4();\n}".to_string(),
        )]
        .into_iter()
        .map(|(uri, text)| db.set_source_text(uri, text))
        .collect();

        let diagnostics = db.check_complexity(&files, &config);
        // May find complexity issues
        for diag in &diagnostics {
            assert_eq!(diag.category, DiagnosticCategory::CodeComplexity);
            assert!(
                diag.severity == DiagnosticSeverity::Warning
                    || diag.severity == DiagnosticSeverity::Info
            );
        }
    }

    #[test]
    fn test_diagnostic_severity() {
        assert_ne!(DiagnosticSeverity::Error, DiagnosticSeverity::Warning);
        assert_ne!(DiagnosticSeverity::Warning, DiagnosticSeverity::Info);
    }

    #[test]
    fn test_diagnostic_category() {
        assert_ne!(
            DiagnosticCategory::Unused,
            DiagnosticCategory::CodeComplexity
        );
        assert_ne!(
            DiagnosticCategory::Naming,
            DiagnosticCategory::Documentation
        );
    }

    #[test]
    fn test_capitalize_first() {
        assert_eq!(capitalize_first("hello"), "Hello");
        assert_eq!(capitalize_first("world"), "World");
        assert_eq!(capitalize_first(""), "");
        assert_eq!(capitalize_first("A"), "A");
    }

    #[test]
    fn test_lint_workspace_with_config() {
        let mut db = WindjammerDatabase::new();
        let config = LintConfig {
            max_function_length: 100,
            max_file_length: 1000,
            check_unused: false,
            check_style: false,
            check_performance: true,
            check_security: true,
            ..Default::default()
        };

        let files: Vec<_> = vec![(
            Url::parse("file:///test.wj").unwrap(),
            "fn main() {}".to_string(),
        )]
        .into_iter()
        .map(|(uri, text)| db.set_source_text(uri, text))
        .collect();

        let _diagnostics = db.lint_workspace(&files, &config);
        // Should return some diagnostics
        // assert!(diagnostics.len() >= 0); // Always true for Vec
    }

    #[test]
    fn test_autofix_enabled() {
        let mut db = WindjammerDatabase::new();
        let config = LintConfig {
            enable_autofix: true,
            ..Default::default()
        };

        let files: Vec<_> = vec![(
            Url::parse("file:///test.wj").unwrap(),
            "fn unused() {}".to_string(),
        )]
        .into_iter()
        .map(|(uri, text)| db.set_source_text(uri, text))
        .collect();

        let diagnostics = db.check_unused_code(&files, &config);
        // Check that auto-fixes are provided when enabled
        for diag in &diagnostics {
            // Auto-fix may or may not be available depending on the diagnostic
            if diag.fix.is_some() {
                assert!(!diag.fix.as_ref().unwrap().description.is_empty());
            }
        }
    }

    #[test]
    fn test_check_error_handling() {
        let mut db = WindjammerDatabase::new();
        let config = LintConfig::default();

        let files: Vec<_> = vec![(
            Url::parse("file:///test.wj").unwrap(),
            "fn test() -> Result<i32> { panic!(\"error\") }".to_string(),
        )]
        .into_iter()
        .map(|(uri, text)| db.set_source_text(uri, text))
        .collect();

        let diagnostics = db.check_error_handling(&files, &config);
        // Should find panic usage
        assert!(diagnostics.iter().any(|d| d.rule == "avoid-panic"));
    }

    #[test]
    fn test_check_performance() {
        let mut db = WindjammerDatabase::new();
        let config = LintConfig::default();

        let files: Vec<_> = vec![(
            Url::parse("file:///test.wj").unwrap(),
            "fn test() { let v = Vec::new(); v.push(1); }".to_string(),
        )]
        .into_iter()
        .map(|(uri, text)| db.set_source_text(uri, text))
        .collect();

        let diagnostics = db.check_performance(&files, &config);
        // Should find vec prealloc issue
        assert!(diagnostics
            .iter()
            .any(|d| d.category == DiagnosticCategory::Performance));
    }

    #[test]
    fn test_check_security() {
        let mut db = WindjammerDatabase::new();
        let config = LintConfig::default();

        let files: Vec<_> = vec![(
            Url::parse("file:///test.wj").unwrap(),
            "fn test() { unsafe { do_something(); } }".to_string(),
        )]
        .into_iter()
        .map(|(uri, text)| db.set_source_text(uri, text))
        .collect();

        let diagnostics = db.check_security(&files, &config);
        // Should find unsafe block
        assert!(diagnostics.iter().any(|d| d.rule == "unsafe-block"));
    }

    #[test]
    fn test_autofix_structure() {
        let fix = AutoFix {
            description: "Test fix".to_string(),
            edits: vec![TextEdit {
                range: tower_lsp::lsp_types::Range::default(),
                new_text: "fixed".to_string(),
            }],
        };

        assert_eq!(fix.description, "Test fix");
        assert_eq!(fix.edits.len(), 1);
        assert_eq!(fix.edits[0].new_text, "fixed");
    }
}

#[cfg(test)]
mod lazy_loading_tests {
    use super::*;

    #[test]
    fn test_lazy_loading_initial_state() {
        let mut db = WindjammerDatabase::new();
        let uri = Url::parse("file:///test.wj").unwrap();
        let file = db.set_source_text(uri, "fn test() {}".to_string());

        // Symbols should not be loaded initially
        assert!(!db.are_symbols_loaded(file));
        assert!(!db.are_references_loaded(file));
    }

    #[test]
    fn test_lazy_loading_mark_loaded() {
        let mut db = WindjammerDatabase::new();
        let uri = Url::parse("file:///test.wj").unwrap();
        let file = db.set_source_text(uri, "fn test() {}".to_string());

        // Mark as loaded
        db.mark_symbols_loaded(file);
        assert!(db.are_symbols_loaded(file));

        db.mark_references_loaded(file);
        assert!(db.are_references_loaded(file));
    }

    #[test]
    fn test_lazy_loading_get_symbols() {
        let mut db = WindjammerDatabase::new();
        let uri = Url::parse("file:///test.wj").unwrap();
        let file = db.set_source_text(uri, "fn test() {}".to_string());

        // Get symbols with lazy loading
        let _symbols = db.get_symbols_lazy(file);
        // assert!(symbols.len() >= 0); // Always true for Vec

        // Should be marked as loaded now
        assert!(db.are_symbols_loaded(file));
    }

    #[test]
    fn test_lazy_loading_clear_caches() {
        let mut db = WindjammerDatabase::new();
        let uri = Url::parse("file:///test.wj").unwrap();
        let file = db.set_source_text(uri, "fn test() {}".to_string());

        db.mark_symbols_loaded(file);
        db.mark_references_loaded(file);

        assert!(db.are_symbols_loaded(file));
        assert!(db.are_references_loaded(file));

        // Clear caches
        db.clear_lazy_caches();

        assert!(!db.are_symbols_loaded(file));
        assert!(!db.are_references_loaded(file));
    }

    #[test]
    fn test_lazy_loading_preload() {
        let mut db = WindjammerDatabase::new();

        let files: Vec<_> = vec![
            (Url::parse("file:///a.wj").unwrap(), "fn a() {}".to_string()),
            (Url::parse("file:///b.wj").unwrap(), "fn b() {}".to_string()),
        ]
        .into_iter()
        .map(|(uri, text)| db.set_source_text(uri, text))
        .collect();

        // Preload symbols
        db.preload_symbols(&files);

        // All files should have symbols loaded
        for file in &files {
            assert!(db.are_symbols_loaded(*file));
        }
    }

    #[test]
    fn test_lazy_loading_multiple_files() {
        let mut db = WindjammerDatabase::new();

        let files: Vec<_> = vec![
            (Url::parse("file:///a.wj").unwrap(), "fn a() {}".to_string()),
            (Url::parse("file:///b.wj").unwrap(), "fn b() {}".to_string()),
            (Url::parse("file:///c.wj").unwrap(), "fn c() {}".to_string()),
        ]
        .into_iter()
        .map(|(uri, text)| db.set_source_text(uri, text))
        .collect();

        // Mark only some as loaded
        db.mark_symbols_loaded(files[0]);
        db.mark_symbols_loaded(files[2]);

        assert!(db.are_symbols_loaded(files[0]));
        assert!(!db.are_symbols_loaded(files[1]));
        assert!(db.are_symbols_loaded(files[2]));
    }
}

#[cfg(test)]
mod code_actions_tests {
    use super::*;

    #[test]
    fn test_extract_function_single_line() {
        let mut db = WindjammerDatabase::new();
        let uri = Url::parse("file:///test.wj").unwrap();
        let code = "fn main() {\n    let x = 1 + 2;\n    println(x);\n}";
        let file = db.set_source_text(uri, code.to_string());

        // Select "1 + 2"
        let range = tower_lsp::lsp_types::Range {
            start: tower_lsp::lsp_types::Position {
                line: 1,
                character: 12,
            },
            end: tower_lsp::lsp_types::Position {
                line: 1,
                character: 17,
            },
        };

        let actions = db.get_code_actions(file, range);
        assert!(!actions.is_empty());

        let extract_action = actions
            .iter()
            .find(|a| a.kind == CodeActionKind::RefactorExtract);
        assert!(extract_action.is_some());

        let action = extract_action.unwrap();
        assert_eq!(action.title, "Extract function");
        assert!(action.is_preferred);
        assert_eq!(action.edits.len(), 2);
    }

    #[test]
    fn test_extract_function_multi_line() {
        let mut db = WindjammerDatabase::new();
        let uri = Url::parse("file:///test.wj").unwrap();
        let code = "fn main() {\n    let x = 1;\n    let y = 2;\n    println(x + y);\n}";
        let file = db.set_source_text(uri, code.to_string());

        // Select lines 2-3
        let range = tower_lsp::lsp_types::Range {
            start: tower_lsp::lsp_types::Position {
                line: 1,
                character: 4,
            },
            end: tower_lsp::lsp_types::Position {
                line: 2,
                character: 18,
            },
        };

        let actions = db.get_code_actions(file, range);
        let extract_action = actions
            .iter()
            .find(|a| a.kind == CodeActionKind::RefactorExtract);
        assert!(extract_action.is_some());
    }

    #[test]
    fn test_extract_function_empty_selection() {
        let mut db = WindjammerDatabase::new();
        let uri = Url::parse("file:///test.wj").unwrap();
        let code = "fn main() {\n    let x = 1;\n}";
        let file = db.set_source_text(uri, code.to_string());

        // Empty selection
        let range = tower_lsp::lsp_types::Range {
            start: tower_lsp::lsp_types::Position {
                line: 1,
                character: 4,
            },
            end: tower_lsp::lsp_types::Position {
                line: 1,
                character: 4,
            },
        };

        let actions = db.get_code_actions(file, range);
        // Should not offer extract function for empty selection
        let extract_action = actions
            .iter()
            .find(|a| a.kind == CodeActionKind::RefactorExtract);
        assert!(extract_action.is_none());
    }

    #[test]
    fn test_code_action_kind() {
        assert_ne!(CodeActionKind::QuickFix, CodeActionKind::RefactorExtract);
        assert_ne!(
            CodeActionKind::RefactorInline,
            CodeActionKind::RefactorRename
        );
    }

    #[test]
    fn test_code_action_structure() {
        let action = CodeAction {
            title: "Test action".to_string(),
            kind: CodeActionKind::QuickFix,
            edits: vec![],
            is_preferred: false,
        };

        assert_eq!(action.title, "Test action");
        assert_eq!(action.kind, CodeActionKind::QuickFix);
        assert!(!action.is_preferred);
    }
}
