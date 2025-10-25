//! Component compiler for `.wj` files
//!
//! This module implements the Windjammer UI component compiler, which transforms
//! `.wj` component files into efficient WASM code with compiled reactivity.
//!
//! ## Architecture
//!
//! The component compiler is separate from the main Windjammer compiler but uses
//! the same parser infrastructure. It follows a multi-stage pipeline:
//!
//! 1. **Parse**: `.wj` file → ComponentFile AST
//! 2. **Analyze**: ComponentFile → DependencyInfo (reactive dependencies)
//! 3. **Transform**: ComponentFile + DependencyInfo → TransformedComponent (with signals)
//! 4. **Codegen**: TransformedComponent → Rust code (WASM-ready)
//!
//! ## Syntax Styles
//!
//! The compiler supports two syntax styles:
//!
//! ### Minimal (Recommended)
//! ```windjammer
//! count: int = 0
//! fn increment() { count += 1 }
//! view { button(on_click: increment) { "{count}" } }
//! ```
//!
//! ### Advanced (Escape Hatch)
//! ```windjammer
//! @component
//! struct Counter {
//!     count: int = 0
//! }
//! impl Counter {
//!     fn increment(&mut self) { self.count += 1 }
//!     fn render(&self) -> VNode { ... }
//! }
//! ```

pub mod analyzer;
pub mod ast;
pub mod codegen;
pub mod parser;
pub mod transformer;

pub use analyzer::{DependencyAnalyzer, DependencyInfo};
pub use ast::{ComponentFile, ComponentStyle, MinimalComponent};
pub use codegen::ComponentCodegen;
pub use parser::ComponentParser;
pub use transformer::{SignalTransformer, TransformedComponent};

use anyhow::Result;

/// Compile a `.wj` component file to Rust code
pub fn compile_component(source: &str) -> Result<String> {
    // Parse
    let component = ComponentParser::parse(source)?;

    // Analyze dependencies
    let deps = DependencyAnalyzer::analyze(&component)?;

    // Transform to signals
    let transformed = SignalTransformer::transform(&component, &deps)?;

    // Generate Rust code
    let code = ComponentCodegen::generate(&transformed)?;

    Ok(code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_empty_component() {
        let source = "";
        let result = compile_component(source);
        assert!(result.is_ok());
    }
}
