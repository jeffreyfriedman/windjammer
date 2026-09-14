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
            // Owned map/collection helpers that tail-match on `.get` borrow the param —
            // defer-drop must not splice inside the match (wj-notes-api int_from_map).
            if body.contains("match ")
                && (body.contains(&format!("{}.get(", opt.variable))
                    || body.contains(&format!("{}.get(&", opt.variable)))
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

    /// Insertion point for defer-drop lines inside a generated function body.
    ///
    /// `body` is the inner block (no surrounding `fn { … }`). Treat depth as starting
    /// inside the function so a tail `match { … }` is not mistaken for the function
    /// tail (wj-notes-api `int_from_map` mid-match spawn).
    fn function_level_tail_line_index(lines: &[&str]) -> usize {
        let mut depth = 1i32;
        let mut insert = lines.len();
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
        if insert == lines.len() && lines.len() >= 2 {
            let last = lines.last().map(|s| s.trim()).unwrap_or("");
            if !last.starts_with('}') && !last.ends_with('{') {
                insert = lines.len().saturating_sub(1);
            }
        }
        insert
    }
}
