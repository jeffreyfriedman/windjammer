//! Bridge between web editor and Windjammer compiler

use crate::error_display::CompilerError;
use wasm_bindgen::prelude::*;

/// Compile Windjammer code to Rust
#[wasm_bindgen]
pub fn compile_windjammer(source: &str) -> Result<String, JsValue> {
    // TODO: Integrate actual Windjammer compiler
    // For now, just return a placeholder

    if source.trim().is_empty() {
        return Err("Source code is empty".into());
    }

    // Placeholder: In the future, this will call the actual compiler
    Ok(format!("// Compiled from Windjammer\n// Source length: {} bytes\n\nfn main() {{\n    println!(\"Compiled successfully!\");\n}}", source.len()))
}

/// Check Windjammer code for errors
#[wasm_bindgen]
pub fn check_windjammer(_source: &str) -> Result<JsValue, JsValue> {
    // TODO: Integrate actual Windjammer compiler error checking
    // For now, just return empty errors

    let errors: Vec<CompilerError> = Vec::new();

    // Convert to JsValue
    serde_wasm_bindgen::to_value(&errors).map_err(|e| e.into())
}

/// Format Windjammer code
#[wasm_bindgen]
pub fn format_windjammer(source: &str) -> Result<String, JsValue> {
    // TODO: Integrate actual Windjammer formatter
    // For now, just return the source as-is
    Ok(source.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_empty() {
        let result = compile_windjammer("");
        assert!(result.is_err());
    }

    #[test]
    fn test_compile_basic() {
        let result = compile_windjammer("fn main() {}");
        assert!(result.is_ok());
    }
}
