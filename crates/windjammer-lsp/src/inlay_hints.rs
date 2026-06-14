use tower_lsp::lsp_types::{InlayHint, Range};

/// Inlay hints provider backed by precomputed ide_analysis hints.
pub struct InlayHintsProvider {
    hints: Vec<InlayHint>,
}

impl InlayHintsProvider {
    pub fn new() -> Self {
        Self { hints: Vec::new() }
    }

    /// Replace hints from `ide_queries::to_inlay_hints`.
    pub fn update_hints(&mut self, hints: Vec<InlayHint>) {
        self.hints = hints;
    }

    /// Get inlay hints intersecting the requested range.
    pub fn get_inlay_hints(&self, range: Range) -> Vec<InlayHint> {
        self.hints
            .iter()
            .filter(|hint| range_contains_position(&range, hint.position))
            .cloned()
            .collect()
    }
}

fn range_contains_position(range: &Range, position: tower_lsp::lsp_types::Position) -> bool {
    if position.line < range.start.line || position.line > range.end.line {
        return false;
    }
    if position.line == range.start.line && position.character < range.start.character {
        return false;
    }
    if position.line == range.end.line && position.character > range.end.character {
        return false;
    }
    true
}

impl Default for InlayHintsProvider {
    fn default() -> Self {
        Self::new()
    }
}
