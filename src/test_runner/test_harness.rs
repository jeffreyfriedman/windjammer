//! Pure helpers for the generated test-harness crate: choosing files, marking generated Rust
//! functions as `#[test]`, and rendering the harness `lib.rs`.

use super::test_discovery::DiscoveredFile;
use super::test_extraction::{TestFunction, TestScope};

/// Files whose tests match `filter` (substring of the test name or its display path).
/// `None` selects everything.
pub(crate) fn select_files<'a>(
    files: &'a [DiscoveredFile],
    filter: Option<&str>,
) -> Vec<&'a DiscoveredFile> {
    files
        .iter()
        .filter(|f| match filter {
            None => true,
            Some(pat) => f
                .tests
                .iter()
                .any(|t| t.name.contains(pat) || t.display_name().contains(pat)),
        })
        .collect()
}

/// Add `#[test]` to generated functions (unless codegen already did) and append runner
/// wrappers for associated-function tests, which Rust cannot run directly.
pub(crate) fn mark_tests(mut rust_code: String, tests: &[TestFunction]) -> String {
    let mut wrappers = String::new();
    for test in tests {
        match &test.scope {
            TestScope::Assoc(ty) => wrappers.push_str(&assoc_wrapper(ty, &test.name)),
            _ => rust_code = mark_free_function(rust_code, &test.name),
        }
    }
    rust_code.push_str(&wrappers);
    rust_code
}

fn mark_free_function(rust_code: String, name: &str) -> String {
    let already_marked = [
        format!("#[test]\npub fn {name}()"),
        format!("#[test]\nfn {name}()"),
        format!("#[test]\n#[inline]\npub fn {name}()"),
        format!("#[test]\n#[inline]\nfn {name}()"),
    ]
    .iter()
    .any(|pat| rust_code.contains(pat));
    if already_marked {
        return rust_code;
    }
    for sig in [format!("pub fn {name}()"), format!("fn {name}()")] {
        if rust_code.contains(&sig) {
            return rust_code.replacen(&sig, &format!("#[test]\n{sig}"), 1);
        }
    }
    rust_code
}

fn assoc_wrapper(ty: &str, name: &str) -> String {
    format!("\n#[test]\nfn __assoc_{ty}_{name}() {{\n    {ty}::{name}();\n}}\n")
}

/// `lib.rs` for the harness crate: declares exactly the modules that were compiled.
pub(crate) fn render_harness_lib_rs(module_names: &[String]) -> String {
    let mut lib_rs = String::from("// Auto-generated test harness\n\n");
    for name in module_names {
        lib_rs.push_str(&format!("pub mod {name};\n"));
    }
    lib_rs
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn file(module: &str, names: &[&str]) -> DiscoveredFile {
        DiscoveredFile {
            path: PathBuf::from(format!("{module}.wj")),
            module_name: module.to_string(),
            tests: names
                .iter()
                .map(|n| TestFunction {
                    name: n.to_string(),
                    file: PathBuf::from(format!("{module}.wj")),
                    scope: TestScope::TopLevel,
                })
                .collect(),
        }
    }

    #[test]
    fn filter_selects_only_matching_files_so_lib_rs_never_declares_uncompiled_modules() {
        let files = vec![file("a_test", &["test_alpha"]), file("b_test", &["test_beta"])];
        let selected = select_files(&files, Some("beta"));
        let names: Vec<String> = selected.iter().map(|f| f.module_name.clone()).collect();
        assert_eq!(names, vec!["b_test"]);
        let lib = render_harness_lib_rs(&names);
        assert!(lib.contains("pub mod b_test;") && !lib.contains("a_test"));
    }

    #[test]
    fn marks_free_functions_once_and_wraps_assoc_tests() {
        let tests = vec![
            TestFunction {
                name: "test_free".into(),
                file: PathBuf::new(),
                scope: TestScope::TopLevel,
            },
            TestFunction {
                name: "test_assoc".into(),
                file: PathBuf::new(),
                scope: TestScope::Assoc("Suite".into()),
            },
        ];
        let out = mark_tests("pub fn test_free() {}\nimpl Suite { fn test_assoc() {} }\n".into(), &tests);
        assert!(out.contains("#[test]\npub fn test_free()"));
        assert!(!out.contains("#[test]\nfn test_assoc()"));
        assert!(out.contains("Suite::test_assoc();"));
        let again = mark_tests(out.clone(), &tests[..1]);
        assert_eq!(again.matches("#[test]\npub fn test_free()").count(), 1);
    }
}
