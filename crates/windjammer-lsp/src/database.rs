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
#[derive(Clone, Default)]
pub struct WindjammerDatabase {
    storage: salsa::Storage<Self>,
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
    pub program: parser::Program,
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

/// A symbol definition
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub line: u32,
    pub character: u32,
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
        if let parser::Item::Use { path, alias: _ } = item {
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
            parser::Item::Function(func) => {
                symbols.push(Symbol {
                    name: func.name.clone(),
                    kind: SymbolKind::Function,
                    line,
                    character: 0,
                });
            }
            parser::Item::Struct(struct_decl) => {
                symbols.push(Symbol {
                    name: struct_decl.name.clone(),
                    kind: SymbolKind::Struct,
                    line,
                    character: 0,
                });
            }
            parser::Item::Enum(enum_decl) => {
                symbols.push(Symbol {
                    name: enum_decl.name.clone(),
                    kind: SymbolKind::Enum,
                    line,
                    character: 0,
                });
            }
            parser::Item::Trait(trait_decl) => {
                symbols.push(Symbol {
                    name: trait_decl.name.clone(),
                    kind: SymbolKind::Trait,
                    line,
                    character: 0,
                });
            }
            parser::Item::Impl(impl_block) => {
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
                });
            }
            parser::Item::Const { name, .. } => {
                symbols.push(Symbol {
                    name: name.clone(),
                    kind: SymbolKind::Const,
                    line,
                    character: 0,
                });
            }
            parser::Item::Static { name, .. } => {
                symbols.push(Symbol {
                    name: name.clone(),
                    kind: SymbolKind::Static,
                    line,
                    character: 0,
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

    let mut references = Vec::new();

    // Walk the AST to find all identifier references
    // For now, we'll extract function calls as a starting point
    for (idx, item) in program.items.iter().enumerate() {
        let line = idx as u32;

        match item {
            parser::Item::Function(func) => {
                // Scan function body for references
                // TODO: Implement proper AST walking
                // For now, we'll just note that references exist
                tracing::debug!("TODO: Scan function '{}' body for references", func.name);
            }
            _ => {}
        }
    }

    tracing::debug!("Found {} references in {}", references.len(), uri);
    ReferenceInfo::new(db, references)
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
        SourceFile::new(self, uri, text)
    }

    /// Get the parsed program for a file
    pub fn get_program(&self, file: SourceFile) -> &parser::Program {
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
