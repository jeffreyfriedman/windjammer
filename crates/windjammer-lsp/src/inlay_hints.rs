use tower_lsp::lsp_types::{InlayHint, InlayHintKind, InlayHintLabel, Position, Range};
use windjammer::analyzer::{AnalyzedFunction, OwnershipMode};

/// Inlay hints provider for displaying ownership inference
pub struct InlayHintsProvider {
    analyzed_functions: Vec<AnalyzedFunction<'static>>,
}

impl InlayHintsProvider {
    pub fn new() -> Self {
        Self {
            analyzed_functions: Vec::new(),
        }
    }

    /// Update with new analyzed functions
    pub fn update_analyzed_functions(&mut self, functions: Vec<AnalyzedFunction>) {
        self.analyzed_functions = functions;
    }

    /// Get inlay hints for a given range
    pub fn get_inlay_hints(&self, _range: Range) -> Vec<InlayHint> {
        let mut hints = Vec::new();

        // For each function, show ownership hints for parameters
        for func in &self.analyzed_functions {
            // Find the line number where the function is defined
            // TODO: Track line numbers during parsing for accurate positioning
            // For now, we'll use a placeholder approach

            // For each parameter with inferred ownership, create an inlay hint
            for (param_name, ownership) in &func.inferred_ownership {
                let ownership_text = match ownership {
                    OwnershipMode::Borrowed => "/* & */",
                    OwnershipMode::MutBorrowed => "/* &mut */",
                    OwnershipMode::Owned => "/* owned */",
                };

                // Find the parameter in the function declaration
                if let Some(_param) = func.decl.parameters.iter().find(|p| &p.name == param_name) {
                    // Create a hint at the parameter's position
                    // TODO: Get actual position from AST
                    // For now, create a placeholder hint
                    let hint = InlayHint {
                        position: Position {
                            line: 0,      // Placeholder
                            character: 0, // Placeholder
                        },
                        label: InlayHintLabel::String(format!(
                            "{}: {}",
                            param_name, ownership_text
                        )),
                        kind: Some(InlayHintKind::PARAMETER),
                        text_edits: None,
                        tooltip: Some(tower_lsp::lsp_types::InlayHintTooltip::String(format!(
                            "Inferred ownership: {}",
                            match ownership {
                                OwnershipMode::Borrowed => "borrowed (&)",
                                OwnershipMode::MutBorrowed => "mutable borrow (&mut)",
                                OwnershipMode::Owned => "owned (moved)",
                            }
                        ))),
                        padding_left: Some(false),
                        padding_right: Some(true),
                        data: None,
                    };
                    hints.push(hint);
                }
            }
        }

        hints
    }

    /// Get ownership summary for hover display (planned for hover improvements)
    #[allow(dead_code)]
    pub fn get_ownership_summary(&self, function_name: &str) -> Option<String> {
        for func in &self.analyzed_functions {
            if func.decl.name == function_name {
                let mut summary = String::from("**Inferred Ownership:**\n");
                for (param_name, ownership) in &func.inferred_ownership {
                    let ownership_text = match ownership {
                        OwnershipMode::Borrowed => "borrowed (&)",
                        OwnershipMode::MutBorrowed => "mutable borrow (&mut)",
                        OwnershipMode::Owned => "owned (moved)",
                    };
                    summary.push_str(&format!("- `{}`: {}\n", param_name, ownership_text));
                }
                return Some(summary);
            }
        }
        None
    }
}

impl Default for InlayHintsProvider {
    fn default() -> Self {
        Self::new()
    }
}
