//! WASM / Cargo feature strings used in generated manifests.

use anyhow::Result;
use std::fs;

use super::dependency_management::walk_rs_files;
use std::path::Path;

/// `web-sys` feature list for browser/WASM builds (embedded in Cargo.toml).
pub(crate) const WEB_SYS_CARGO_FEATURES: &str = r#"    "Document",
    "Element",
    "HtmlElement",
    "Node",
    "Text",
    "Window",
    "Event",
    "MouseEvent",
    "KeyboardEvent","#;

/// Whether generated Rust needs `windjammer-runtime` (WASM feature), mirroring `main.rs` `create_wasm_cargo_toml`.
pub(crate) fn wasm_output_needs_runtime(output_dir: &Path) -> Result<bool> {
    for path in walk_rs_files(output_dir)? {
        let content = fs::read_to_string(&path)?;
        if content.contains("windjammer_runtime") {
            return Ok(true);
        }
        // Platform modules routed through runtime (same heuristic as CLI)
        if content.contains("fs::")
            || content.contains("process::")
            || content.contains("dialog::")
            || content.contains("env::")
            || content.contains("encoding::")
        {
            return Ok(true);
        }
    }
    Ok(false)
}
