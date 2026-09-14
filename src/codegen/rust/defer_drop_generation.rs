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

        let insert_before = Self::function_level_tail_line_index(&lines);

        let mut new_body = String::new();

        for (i, line) in lines.iter().enumerate() {
            if i < insert_before {
                new_body.push_str(line);
                new_body.push('\n');
            }
        }

        for opt in optimizations {
            if lines[insert_before..]
                .iter()
                .any(|line| line.contains(&opt.variable))
            {
                continue;
            }
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

        for line in &lines[insert_before..] {
            new_body.push_str(line);
            if *line != lines[lines.len() - 1] {
                new_body.push('\n');
            }
        }

        new_body
    }

    /// Line index of the function body's closing `}` (insert defer-drop immediately before it).
    fn function_level_tail_line_index(lines: &[&str]) -> usize {
        let mut depth = 0i32;
        let mut insert = lines.len().saturating_sub(1);
        for (i, line) in lines.iter().enumerate() {
            let before = depth;
            for ch in line.chars() {
                match ch {
                    '{' => depth += 1,
                    '}' => depth -= 1,
                    _ => {}
                }
            }
            if before == 1 && depth == 0 {
                insert = i;
                break;
            }
        }
        insert
    }
}
