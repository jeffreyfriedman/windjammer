//! `wj check` method-name surface (WJ0015). The table itself is `crate::stdlib_surface::known_methods`, shared with the
//! interpreter's parity gate (`tests/interpreter_method_parity_test.rs`); this file adds the checker-only names and the
//! gate that every method `std/**/*.wj` declares is in the table.

pub use crate::stdlib_surface::known_methods::KNOWN_METHODS;

pub fn is_known(name: &str) -> bool {
    crate::stdlib_surface::known_methods::is_surface_method(name) || name == crate::parser::optional_chain::OPT_CHAIN
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A valid call into the standard library must never be reported as an unknown method (the
    /// interpreter refuses programs `wj check` rejects, so a false WJ0015 would block a valid run).
    #[test]
    fn std_methods_cover_the_embedded_std() {
        fn walk(dir: &std::path::Path, missing: &mut Vec<String>) {
            for e in std::fs::read_dir(dir).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() {
                    walk(&p, missing);
                } else if p.extension().is_some_and(|x| x == "wj") {
                    for line in std::fs::read_to_string(&p).unwrap().lines() {
                        let t = line.trim_start();
                        if line.len() - t.len() < 2 {
                            continue;
                        }
                        let t = t.strip_prefix("pub ").unwrap_or(t);
                        let t = t.strip_prefix("async ").unwrap_or(t);
                        if let Some(rest) = t.strip_prefix("fn ") {
                            let name: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
                            if !name.is_empty() && !name.starts_with(char::is_uppercase) && !is_known(&name) {
                                missing.push(format!("{}: {name}", p.display()));
                            }
                        }
                    }
                }
            }
        }
        let mut missing = Vec::new();
        walk(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("std"), &mut missing);
        assert!(missing.is_empty(), "std methods missing from KNOWN_METHODS:\n{}", missing.join("\n"));
    }
}
