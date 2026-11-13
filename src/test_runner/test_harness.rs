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
/// wrappers for associated-function tests, which Rust cannot run directly. With `cwd` (the package root),
/// every test first makes it the working directory: cargo runs the harness from its scratch crate, and a
/// test reading `README.md` must see the package's file, as `wj test --interpret` does.
pub(crate) fn mark_tests(mut rust_code: String, tests: &[TestFunction], cwd: Option<&std::path::Path>) -> String {
    let enter = cwd.map(|d| format!("\n    let _ = ::std::env::set_current_dir({:?});", d.to_string_lossy())).unwrap_or_default();
    let mut wrappers = String::new();
    for test in tests {
        match &test.scope {
            TestScope::Assoc(ty) => wrappers.push_str(&assoc_wrapper(ty, &test.name, &enter)),
            _ => {
                rust_code = mark_free_function(rust_code, &test.name);
                rust_code = enter_body(rust_code, &test.name, &enter);
            }
        }
    }
    rust_code.push_str(&wrappers);
    rust_code
}

/// Put `enter` at the start of free test function `name`'s body.
fn enter_body(rust_code: String, name: &str, enter: &str) -> String {
    if enter.is_empty() {
        return rust_code;
    }
    for sig in [format!("pub fn {name}()"), format!("fn {name}()")] {
        if let Some(pos) = rust_code.find(&sig) {
            if let Some(open) = rust_code[pos..].find('{') {
                let at = pos + open + 1;
                return format!("{}{}{}", &rust_code[..at], enter, &rust_code[at..]);
            }
        }
    }
    rust_code
}

fn mark_free_function(rust_code: String, name: &str) -> String {
    for sig in [format!("pub fn {name}()"), format!("fn {name}()")] {
        if let Some(pos) = rust_code.find(&sig) {
            if preceded_by_test_attribute(&rust_code[..pos]) {
                return rust_code;
            }
            return rust_code.replacen(&sig, &format!("#[test]\n{sig}"), 1);
        }
    }
    rust_code
}

/// True when the attribute block directly above an item (attribute lines and blank lines, in
/// any order) already contains `#[test]`. Codegen may separate it from the `fn` by a blank line.
fn preceded_by_test_attribute(before: &str) -> bool {
    for line in before.lines().rev() {
        let t = line.trim();
        if t == "#[test]" {
            return true;
        }
        if !(t.is_empty() || t.starts_with("#[")) {
            return false;
        }
    }
    false
}

fn assoc_wrapper(ty: &str, name: &str, enter: &str) -> String {
    format!("\n#[test]\nfn __assoc_{ty}_{name}() {{{enter}\n    {ty}::{name}();\n}}\n")
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
    fn every_test_enters_the_package_root_first() {
        let tests = vec![
            TestFunction { name: "test_free".into(), file: PathBuf::new(), scope: TestScope::TopLevel },
            TestFunction { name: "test_assoc".into(), file: PathBuf::new(), scope: TestScope::Assoc("Suite".into()) },
        ];
        let code = "fn helper() {\n}\n\nfn test_free() {\n    helper();\n}\n".to_string();
        let out = mark_tests(code, &tests, Some(std::path::Path::new("/pkg/root")));
        let enter = "let _ = ::std::env::set_current_dir(\"/pkg/root\");";
        assert!(out.contains(&format!("#[test]\nfn test_free() {{\n    {enter}\n    helper();")), "{out}");
        assert!(out.contains(&format!("fn __assoc_Suite_test_assoc() {{\n    {enter}\n    Suite::test_assoc();")), "{out}");
        assert!(!out.contains(&format!("fn helper() {{\n    {enter}")), "{out}");
        let plain = mark_tests("fn test_free() {\n}\n".to_string(), &tests[..1], None);
        assert!(!plain.contains("set_current_dir"), "{plain}");
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
        let out = mark_tests("pub fn test_free() {}\nimpl Suite { fn test_assoc() {} }\n".into(), &tests, None);
        assert!(out.contains("#[test]\npub fn test_free()"));
        assert!(!out.contains("#[test]\nfn test_assoc()"));
        assert!(out.contains("Suite::test_assoc();"));
        let again = mark_tests(out.clone(), &tests[..1], None);
        assert_eq!(again.matches("#[test]\npub fn test_free()").count(), 1);
    }

    #[test]
    fn codegen_marked_test_separated_by_blank_line_is_not_marked_twice() {
        let tests = vec![TestFunction {
            name: "test_smoke".into(),
            file: PathBuf::new(),
            scope: TestScope::TopLevel,
        }];
        let src = "#[test]\n\n#[inline]\npub fn test_smoke() {}\n";
        assert_eq!(mark_tests(src.into(), &tests, None), src);
    }
}
