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

/// Symbol table for tracking definitions
#[derive(Clone)]
pub struct SymbolTable {
    symbols: HashMap<String, SymbolDefinition>,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self {
            symbols: HashMap::new(),
        }
    }

    /// Build symbol table from a parsed program
    pub fn build_from_program(&mut self, program: &Program, uri: &Url) {
        self.symbols.clear();

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
}

impl Default for SymbolTable {
    fn default() -> Self {
        Self::new()
    }
}
