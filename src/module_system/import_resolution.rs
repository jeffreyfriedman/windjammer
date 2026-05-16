//! Parsing `pub mod` / `pub use` declarations from `mod.wj`.

/// Parse mod.wj to extract pub mod and pub use declarations
///
/// Returns: (pub_mod_names, pub_use_paths)
pub(crate) fn parse_mod_declarations(content: &str) -> (Vec<String>, Vec<String>) {
    let mut pub_mods = Vec::new();
    let mut pub_uses = Vec::new();

    // Track multi-line pub use statements
    let mut in_pub_use = false;
    let mut current_pub_use = String::new();

    for line in content.lines() {
        let trimmed = line.trim();

        // Skip empty lines and comments
        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }

        // Match: pub mod <name>
        if trimmed.starts_with("pub mod ") && !in_pub_use {
            if let Some(name) = trimmed
                .strip_prefix("pub mod ")
                .and_then(|s| s.split_whitespace().next())
            {
                // Remove trailing semicolon from module name
                let name = name.trim_end_matches(';');
                pub_mods.push(name.to_string());
            }
        }
        // Match: pub use <path>
        else if trimmed.starts_with("pub use ") {
            in_pub_use = true;
            current_pub_use.push_str(trimmed.strip_prefix("pub use ").unwrap());
            current_pub_use.push(' ');

            // Check if this line completes the pub use statement
            // Complete if: has closing brace, or doesn't have opening brace (single-line)
            let has_opening_brace = trimmed.contains('{');
            let has_closing_brace = trimmed.contains('}');

            if has_closing_brace || !has_opening_brace {
                in_pub_use = false;
                // Remove trailing semicolon and whitespace
                let pub_use_str = current_pub_use.trim().trim_end_matches(';').to_string();
                pub_uses.push(pub_use_str);
                current_pub_use.clear();
            }
        }
        // Continue multi-line pub use
        else if in_pub_use {
            current_pub_use.push_str(trimmed);
            current_pub_use.push(' ');

            // Check if this line completes the pub use statement
            if trimmed.contains('}') {
                in_pub_use = false;
                // Remove trailing semicolon and whitespace
                let pub_use_str = current_pub_use.trim().trim_end_matches(';').to_string();
                pub_uses.push(pub_use_str);
                current_pub_use.clear();
            }
        }
    }

    (pub_mods, pub_uses)
}
