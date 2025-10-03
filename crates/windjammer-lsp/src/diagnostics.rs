use tower_lsp::Client;
use tower_lsp::lsp_types::*;

use crate::database::{RootDatabase, FileId, ParseDatabase, AnalysisDatabase};

pub async fn publish_diagnostics(
    client: &Client,
    db: &RootDatabase,
    uri: &Url,
    file_id: FileId,
) {
    let mut diagnostics = Vec::new();
    
    // Collect syntax errors
    let syntax_errors = db.syntax_errors(file_id);
    for error in syntax_errors.iter() {
        diagnostics.push(Diagnostic {
            range: Range::new(
                Position::new(error.line, error.col),
                Position::new(error.line, error.col + 1),
            ),
            severity: Some(DiagnosticSeverity::ERROR),
            code: None,
            code_description: None,
            source: Some("windjammer".to_string()),
            message: error.message.clone(),
            related_information: None,
            tags: None,
            data: None,
        });
    }
    
    // Collect semantic errors
    let semantic_errors = db.semantic_errors(file_id);
    for error in semantic_errors.iter() {
        diagnostics.push(Diagnostic {
            range: Range::new(
                Position::new(error.line, error.col),
                Position::new(error.line, error.col + 1),
            ),
            severity: Some(DiagnosticSeverity::ERROR),
            code: None,
            code_description: None,
            source: Some("windjammer".to_string()),
            message: error.message.clone(),
            related_information: None,
            tags: None,
            data: None,
        });
    }
    
    // Add ownership analysis hints
    let analyzed = db.analyze_file(file_id);
    for func in analyzed.iter() {
        for (param_name, ownership_mode) in &func.inferred_ownership {
            // Add informational diagnostic showing inferred ownership
            let message = format!(
                "Parameter '{}' inferred as {:?}",
                param_name, ownership_mode
            );
            
            diagnostics.push(Diagnostic {
                range: Range::new(Position::new(0, 0), Position::new(0, 1)),
                severity: Some(DiagnosticSeverity::HINT),
                code: None,
                code_description: None,
                source: Some("windjammer-inference".to_string()),
                message,
                related_information: None,
                tags: None,
                data: None,
            });
        }
    }
    
    client
        .publish_diagnostics(uri.clone(), diagnostics, None)
        .await;
}

