//! Optional defer-drop wrapping for generated function bodies.

use crate::codegen::rust::generator::CodeGenerator;

impl<'ast> CodeGenerator<'ast> {
    /// PHASE 6 OPTIMIZATION: Wrap function body with defer drop logic
    /// This defers heavy deallocations to a background thread, making functions return 10,000x faster.
    /// Reference: https://abrams.cc/rust-dropping-things-in-another-thread
    ///
    /// Transform:
    ///   let result = compute();
    ///   result
    /// Into:
    ///   let result = compute();
    ///   std::thread::spawn(move || drop(variable));
    ///   result
    pub(crate) fn wrap_with_defer_drop(
        &self,
        body: String,
        optimizations: &[crate::analyzer::DeferDropOptimization],
    ) -> String {
        if optimizations.is_empty() {
            return body;
        }

        let lines: Vec<&str> = body.lines().collect();
        if lines.is_empty() {
            return body;
        }

        let mut new_body = String::new();

        // Find the last non-empty, non-comment line (likely the return expression or last statement)
        let mut last_line_idx = lines.len() - 1;
        while last_line_idx > 0 {
            let trimmed = lines[last_line_idx].trim();
            if !trimmed.is_empty() && !trimmed.starts_with("//") {
                break;
            }
            last_line_idx -= 1;
        }

        // Copy all lines except the last one
        for (i, line) in lines.iter().enumerate() {
            if i < last_line_idx {
                new_body.push_str(line);
                new_body.push('\n');
            }
        }

        // Insert defer drop statements before the final return/expression
        for opt in optimizations {
            // Generate the defer drop code
            new_body.push_str(&self.indent());
            new_body.push_str(&format!(
                "// DEFER DROP: Deallocate {} ({:?}) in background thread for faster return\n",
                opt.variable, opt.estimated_size
            ));
            new_body.push_str(&self.indent());
            new_body.push_str(&format!(
                "std::thread::spawn(move || drop({}));\n",
                opt.variable
            ));
        }

        // Add the final line (return expression or last statement)
        new_body.push_str(lines[last_line_idx]);

        // Add any trailing lines (closing braces, etc.)
        for line in &lines[last_line_idx + 1..] {
            new_body.push('\n');
            new_body.push_str(line);
        }

        new_body
    }
}
