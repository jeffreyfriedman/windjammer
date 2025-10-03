use tower_lsp::lsp_types::*;
use crate::database::{RootDatabase, FileId, AnalysisDatabase};

pub fn get_completions(
    db: &RootDatabase,
    file_id: FileId,
    position: Position,
) -> Vec<CompletionItem> {
    let mut completions = Vec::new();
    
    // Get all symbols in the file
    let symbols = db.symbols(file_id);
    
    for symbol in symbols.iter() {
        let kind = match symbol.kind {
            crate::database::SymbolKind::Function => CompletionItemKind::FUNCTION,
            crate::database::SymbolKind::Struct => CompletionItemKind::STRUCT,
            crate::database::SymbolKind::Enum => CompletionItemKind::ENUM,
            crate::database::SymbolKind::Variable => CompletionItemKind::VARIABLE,
            crate::database::SymbolKind::Parameter => CompletionItemKind::VARIABLE,
            crate::database::SymbolKind::Field => CompletionItemKind::FIELD,
        };
        
        completions.push(CompletionItem {
            label: symbol.name.clone(),
            kind: Some(kind),
            detail: symbol.doc.clone(),
            documentation: symbol.doc.as_ref().map(|d| {
                Documentation::String(d.clone())
            }),
            ..Default::default()
        });
    }
    
    // Add keyword completions
    let keywords = vec![
        "fn", "let", "mut", "struct", "enum", "impl", "match",
        "if", "else", "for", "while", "loop", "return", "break",
        "continue", "use", "go", "async", "await", "defer",
    ];
    
    for keyword in keywords {
        completions.push(CompletionItem {
            label: keyword.to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            ..Default::default()
        });
    }
    
    // Add type completions
    let types = vec!["int", "int32", "uint", "float", "bool", "string"];
    
    for typ in types {
        completions.push(CompletionItem {
            label: typ.to_string(),
            kind: Some(CompletionItemKind::TYPE_PARAMETER),
            ..Default::default()
        });
    }
    
    // Add common decorator completions
    let decorators = vec![
        ("@route", "HTTP route decorator"),
        ("@get", "HTTP GET method"),
        ("@post", "HTTP POST method"),
        ("@put", "HTTP PUT method"),
        ("@delete", "HTTP DELETE method"),
        ("@timing", "Measure execution time"),
        ("@cache", "Cache function results"),
        ("@wasm_bindgen", "Export to WebAssembly"),
    ];
    
    for (name, doc) in decorators {
        completions.push(CompletionItem {
            label: name.to_string(),
            kind: Some(CompletionItemKind::FUNCTION),
            documentation: Some(Documentation::String(doc.to_string())),
            insert_text: Some(name[1..].to_string()), // Remove @ for insertion
            ..Default::default()
        });
    }
    
    completions
}

