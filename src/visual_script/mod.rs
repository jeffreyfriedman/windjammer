//! Visual script (**`.vgraph.json`**) → Windjammer **source** lowering.
//!
//! Design: see `windjammer-game/VISUAL_SCRIPTING_DESIGN.md` in the game repo.
//! The canonical compilation path emits `.wj` text and reuses the normal lexer/parser/analyzer/codegen pipeline.

mod lower;
mod schema;

pub use lower::{lower_document_to_windjammer, LowerError};
pub use schema::{EdgeKind, VsDocument, VsEdge, VsEndpoint, VsNode, VsNodeKind};

use anyhow::{Context, Result};

/// Parse JSON and lower to a Windjammer module source string.
pub fn compile_vgraph_json_to_windjammer(json: &str) -> Result<String> {
    let doc: VsDocument =
        serde_json::from_str(json).context("parse windjammer-vgraph JSON")?;
    lower_document_to_windjammer(&doc).map_err(|e| anyhow::anyhow!("{}", e))
}
