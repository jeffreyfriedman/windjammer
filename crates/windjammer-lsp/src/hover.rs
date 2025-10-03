use tower_lsp::lsp_types::*;
use crate::database::{RootDatabase, FileId, AnalysisDatabase};

pub fn get_hover_info(
    db: &RootDatabase,
    file_id: FileId,
    position: Position,
) -> Option<Hover> {
    // Try to get type information at the position
    if let Some(type_info) = db.type_at_position(file_id, position.line, position.character) {
        return Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: format!("```windjammer\n{}\n```", type_info),
            }),
            range: None,
        });
    }
    
    // Get symbols and check if cursor is on one
    let symbols = db.symbols(file_id);
    
    for symbol in symbols.iter() {
        if symbol.line == position.line {
            let hover_text = match symbol.kind {
                crate::database::SymbolKind::Function => {
                    format!("```windjammer\nfn {}()\n```", symbol.name)
                }
                crate::database::SymbolKind::Struct => {
                    format!("```windjammer\nstruct {}\n```", symbol.name)
                }
                crate::database::SymbolKind::Enum => {
                    format!("```windjammer\nenum {}\n```", symbol.name)
                }
                crate::database::SymbolKind::Variable => {
                    format!("```windjammer\nlet {}\n```", symbol.name)
                }
                _ => continue,
            };
            
            let contents = if let Some(doc) = &symbol.doc {
                format!("{}\n\n{}", hover_text, doc)
            } else {
                hover_text
            };
            
            return Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: contents,
                }),
                range: Some(Range::new(
                    Position::new(symbol.line, symbol.col),
                    Position::new(symbol.line, symbol.col + symbol.name.len() as u32),
                )),
            });
        }
    }
    
    None
}

