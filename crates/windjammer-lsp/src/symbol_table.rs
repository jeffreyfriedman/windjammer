use std::collections::HashMap;
use tower_lsp::lsp_types::{Location, Position, Range, Url};
use windjammer::parser::{Item, Program};

/// Symbol definition in the code
#[derive(Debug, Clone)]
pub struct SymbolDefinition {
    pub name: String,
    pub kind: SymbolKind,
    pub location: Location,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SymbolKind {
    Function,
    Struct,
    Enum,
    Trait,
    Impl,
    Const,
    Static,
}

/// Symbol reference (usage) in the code
#[derive(Debug, Clone)]
pub struct SymbolReference {
    pub name: String,
    pub location: Location,
}

/// Symbol table for tracking definitions and references
#[derive(Clone)]
pub struct SymbolTable {
    symbols: HashMap<String, SymbolDefinition>,
    references: HashMap<String, Vec<SymbolReference>>,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self {
            symbols: HashMap::new(),
            references: HashMap::new(),
        }
    }

    /// Build symbol table from a parsed program
    pub fn build_from_program(&mut self, program: &Program, uri: &Url) {
        self.symbols.clear();
        self.references.clear();

        for (idx, item) in program.items.iter().enumerate() {
            match item {
                Item::Function(func) => {
                    // TODO: Get actual line number from AST
                    // For now, use item index as a heuristic
                    let location = Location {
                        uri: uri.clone(),
                        range: Range {
                            start: Position {
                                line: idx as u32,
                                character: 0,
                            },
                            end: Position {
                                line: idx as u32,
                                character: 100,
                            },
                        },
                    };

                    self.symbols.insert(
                        func.name.clone(),
                        SymbolDefinition {
                            name: func.name.clone(),
                            kind: SymbolKind::Function,
                            location,
                        },
                    );
                }
                Item::Struct(struct_decl) => {
                    let location = Location {
                        uri: uri.clone(),
                        range: Range {
                            start: Position {
                                line: idx as u32,
                                character: 0,
                            },
                            end: Position {
                                line: idx as u32,
                                character: 100,
                            },
                        },
                    };

                    self.symbols.insert(
                        struct_decl.name.clone(),
                        SymbolDefinition {
                            name: struct_decl.name.clone(),
                            kind: SymbolKind::Struct,
                            location,
                        },
                    );
                }
                Item::Enum(enum_decl) => {
                    let location = Location {
                        uri: uri.clone(),
                        range: Range {
                            start: Position {
                                line: idx as u32,
                                character: 0,
                            },
                            end: Position {
                                line: idx as u32,
                                character: 100,
                            },
                        },
                    };

                    self.symbols.insert(
                        enum_decl.name.clone(),
                        SymbolDefinition {
                            name: enum_decl.name.clone(),
                            kind: SymbolKind::Enum,
                            location,
                        },
                    );

                    // Also add enum variants
                    for variant in &enum_decl.variants {
                        let variant_location = Location {
                            uri: uri.clone(),
                            range: Range {
                                start: Position {
                                    line: idx as u32,
                                    character: 0,
                                },
                                end: Position {
                                    line: idx as u32,
                                    character: 100,
                                },
                            },
                        };

                        self.symbols.insert(
                            variant.name.clone(),
                            SymbolDefinition {
                                name: variant.name.clone(),
                                kind: SymbolKind::Enum,
                                location: variant_location,
                            },
                        );
                    }
                }
                Item::Trait(trait_decl) => {
                    let location = Location {
                        uri: uri.clone(),
                        range: Range {
                            start: Position {
                                line: idx as u32,
                                character: 0,
                            },
                            end: Position {
                                line: idx as u32,
                                character: 100,
                            },
                        },
                    };

                    self.symbols.insert(
                        trait_decl.name.clone(),
                        SymbolDefinition {
                            name: trait_decl.name.clone(),
                            kind: SymbolKind::Trait,
                            location,
                        },
                    );
                }
                Item::Impl(impl_block) => {
                    // For impl blocks, we could track methods
                    // TODO: Add method tracking
                    let _ = impl_block;
                }
                Item::Const { name, .. } => {
                    let location = Location {
                        uri: uri.clone(),
                        range: Range {
                            start: Position {
                                line: idx as u32,
                                character: 0,
                            },
                            end: Position {
                                line: idx as u32,
                                character: 100,
                            },
                        },
                    };

                    self.symbols.insert(
                        name.clone(),
                        SymbolDefinition {
                            name: name.clone(),
                            kind: SymbolKind::Const,
                            location,
                        },
                    );
                }
                Item::Static { name, .. } => {
                    let location = Location {
                        uri: uri.clone(),
                        range: Range {
                            start: Position {
                                line: idx as u32,
                                character: 0,
                            },
                            end: Position {
                                line: idx as u32,
                                character: 100,
                            },
                        },
                    };

                    self.symbols.insert(
                        name.clone(),
                        SymbolDefinition {
                            name: name.clone(),
                            kind: SymbolKind::Static,
                            location,
                        },
                    );
                }
                _ => {}
            }
        }
    }

    /// Find a symbol by name
    pub fn find_symbol(&self, name: &str) -> Option<&SymbolDefinition> {
        self.symbols.get(name)
    }

    /// Get all symbols
    pub fn all_symbols(&self) -> Vec<&SymbolDefinition> {
        self.symbols.values().collect()
    }

    /// Find symbol at a given position
    /// TODO: Implement proper position-based lookup when we have line tracking
    pub fn find_symbol_at_position(&self, _position: Position) -> Option<&SymbolDefinition> {
        // Placeholder: return None for now
        // This would need proper AST position tracking
        None
    }

    /// Find all references to a symbol by name
    pub fn find_references(&self, name: &str) -> Vec<&SymbolReference> {
        self.references
            .get(name)
            .map(|refs| refs.iter().collect())
            .unwrap_or_default()
    }

    /// Build references from source code
    /// This is a simple text-based search for now
    /// TODO: Use proper AST walking for accurate reference detection
    pub fn build_references_from_source(&mut self, source: &str, uri: &Url) {
        self.references.clear();

        // Get all symbol names
        let symbol_names: Vec<String> = self.symbols.keys().cloned().collect();

        // Search for each symbol in the source code
        for symbol_name in &symbol_names {
            let mut line_num = 0;
            for line in source.lines() {
                // Find all occurrences of the symbol name in this line
                let mut start = 0;
                while let Some(pos) = line[start..].find(symbol_name.as_str()) {
                    let actual_pos = start + pos;

                    // Simple word boundary check
                    let before_ok = actual_pos == 0
                        || !line
                            .chars()
                            .nth(actual_pos - 1)
                            .unwrap_or(' ')
                            .is_alphanumeric();
                    let after_ok = actual_pos + symbol_name.len() >= line.len()
                        || !line
                            .chars()
                            .nth(actual_pos + symbol_name.len())
                            .unwrap_or(' ')
                            .is_alphanumeric();

                    if before_ok && after_ok {
                        let reference = SymbolReference {
                            name: symbol_name.clone(),
                            location: Location {
                                uri: uri.clone(),
                                range: Range {
                                    start: Position {
                                        line: line_num,
                                        character: actual_pos as u32,
                                    },
                                    end: Position {
                                        line: line_num,
                                        character: (actual_pos + symbol_name.len()) as u32,
                                    },
                                },
                            },
                        };

                        self.references
                            .entry(symbol_name.clone())
                            .or_insert_with(Vec::new)
                            .push(reference);
                    }

                    start = actual_pos + 1;
                }

                line_num += 1;
            }
        }
    }
}

impl Default for SymbolTable {
    fn default() -> Self {
        Self::new()
    }
}
