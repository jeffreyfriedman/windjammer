//! `std::…` import paths → Rust `use` output (including runtime remapping).

use crate::codegen::rust::CodeGenerator;

impl CodeGenerator<'_> {
    /// If `full_path` is a Windjammer `std::` / `std.` import, returns the generated `use` line(s).
    /// Otherwise returns [`None`] so the caller can continue with other rules.
    pub(in crate::codegen::rust) fn try_generate_std_import_use(
        &self,
        full_path: &str,
        alias: Option<&str>,
    ) -> Option<String> {
        if !(full_path.starts_with("std::") || full_path.starts_with("std.")) {
            return None;
        }

        // Normalize to use :: separator
        let normalized = full_path.replace('.', "::");
        let module_name = normalized.strip_prefix("std::").unwrap();

        // Strip glob suffix if present for checking
        let module_base = module_name.strip_suffix("::*").unwrap_or(module_name);

        // Handle Rust stdlib modules that should NOT be mapped to windjammer_runtime
        // These are native Rust modules that should be used directly
        if module_base.starts_with("collections")
            || module_base.starts_with("cmp")
            || module_base.starts_with("ops")
            || module_base == "ops"
        {
            // TDD FIX: Pass through to Rust's std library with alias support
            if let Some(alias_name) = alias {
                return Some(format!("use std::{} as {};\n", module_name, alias_name));
            }
            return Some(format!("use std::{};\n", module_name));
        }

        // Handle UI framework - skip explicit import (handled by implicit imports)
        if module_base == "ui" || module_base.starts_with("ui::") {
            return Some(String::new());
        }

        // Handle Game framework - skip explicit import (handled by implicit imports)
        if module_base == "game" || module_base.starts_with("game::") {
            return Some(String::new());
        }

        // Handle Tauri framework - skip explicit import (functions are generated inline)
        if module_base == "tauri" || module_base.starts_with("tauri::") {
            return Some(String::new());
        }

        // Rust std modules: pass through as `use std::fs`, `use std::process`, etc.
        // `std::env` maps to windjammer_runtime (cross-backend get/get_or).
        if module_base == "fs"
            || module_base.starts_with("fs::")
            || module_base == "process"
            || module_base.starts_with("process::")
        {
            return Some(format!("use std::{};\n", module_name));
        }

        if module_base == "env" || module_base.starts_with("env::") {
            return Some("use windjammer_runtime::env;\n".to_string());
        }

        // Platform APIs with no Rust std equivalent - skip (http maps to windjammer_runtime below)
        if module_base == "dialog"
            || module_base.starts_with("dialog::")
            || module_base == "encoding"
            || module_base.starts_with("encoding::")
            || module_base == "compute"
            || module_base.starts_with("compute::")
            || module_base == "net"
            || module_base.starts_with("net::")
            || module_base == "storage"
            || module_base.starts_with("storage::")
        {
            return Some(String::new());
        }

        // Map to windjammer_runtime (all stdlib modules are now implemented!)
        let rust_import = match module_base {
            // Core modules
            "http" => "windjammer_runtime::http",
            "mime" => "windjammer_runtime::mime",
            "json" => "windjammer_runtime::json",
            "io" => "windjammer_runtime::io",
            "subprocess" => "windjammer_runtime::subprocess",

            // Additional modules
            "async" | "async_runtime" => "windjammer_runtime::async_runtime",
            "cli" => "windjammer_runtime::cli",
            "crypto" => "windjammer_runtime::crypto",
            "csv" => "windjammer_runtime::csv_mod",
            "db" => "windjammer_runtime::db",
            "log" => "windjammer_runtime::log_mod",
            "math" => "windjammer_runtime::math",
            "random" => "windjammer_runtime::random",
            "regex" => "windjammer_runtime::regex_mod",
            "strings" => "windjammer_runtime::strings",
            "testing" => "windjammer_runtime::testing",
            "time" => "windjammer_runtime::time",
            "game" => "windjammer_runtime::game",

            _ => {
                // Unknown module - try windjammer_runtime
                return Some(format!("use windjammer_runtime::{};\n", module_name));
            }
        };

        if let Some(alias_name) = alias {
            return Some(format!("use {} as {};\n", rust_import, alias_name));
        }

        // For _mod suffixed modules (log_mod, regex_mod), alias back to the original name
        // AND import any public types they export
        if rust_import.ends_with("_mod") {
            let original_name = rust_import
                .strip_suffix("_mod")
                .and_then(|s| s.split("::").last())
                .unwrap_or(rust_import);

            let mut result = format!("use {} as {};\n", rust_import, original_name);

            // Import types for modules that export them
            match original_name {
                "regex" => {
                    result.push_str(&format!("use {}::Regex;\n", rust_import));
                }
                "time" => {
                    result.push_str(&format!("use {}::{{Duration, Instant}};\n", rust_import));
                }
                _ => {}
            }

            return Some(result);
        }
        // Preserve `::*` glob suffix from the Windjammer import
        let glob_suffix = module_name.strip_prefix(module_base).unwrap_or("");
        // Import the module (glob or qualified) to match the Windjammer `use` form
        Some(format!("use {}{};\n", rust_import, glob_suffix))
    }
}
