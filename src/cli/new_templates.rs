//! Project templates embedded in the binary at compile time.
//!
//! `wj new` must work from an installed binary in any directory, so templates are
//! never read from disk at runtime. The test `embedded_table_covers_templates_dir`
//! fails if a file is added under `templates/` without being listed here.

/// `(template, path relative to the project root, contents)`.
/// Contents may contain `{{PROJECT_NAME}}`.
pub const EMBEDDED: &[(&str, &str, &str)] = &[
    (
        "cli",
        "src/main.wj",
        include_str!("../../templates/cli/main.wj"),
    ),
    (
        "cli",
        "wj.toml",
        include_str!("../../templates/cli/wj.toml"),
    ),
    (
        "cli",
        ".gitignore",
        include_str!("../../templates/cli/gitignore"),
    ),
    (
        "cli",
        "README.md",
        include_str!("../../templates/cli/README.md"),
    ),
    (
        "web",
        "src/main.wj",
        include_str!("../../templates/web/main.wj"),
    ),
    (
        "web",
        "wj.toml",
        include_str!("../../templates/web/wj.toml"),
    ),
    (
        "web",
        ".gitignore",
        include_str!("../../templates/web/gitignore"),
    ),
    (
        "web",
        "README.md",
        include_str!("../../templates/web/README.md"),
    ),
    (
        "lib",
        "src/lib.wj",
        include_str!("../../templates/lib/lib.wj"),
    ),
    (
        "lib",
        "wj.toml",
        include_str!("../../templates/lib/wj.toml"),
    ),
    (
        "lib",
        ".gitignore",
        include_str!("../../templates/lib/gitignore"),
    ),
    (
        "lib",
        "README.md",
        include_str!("../../templates/lib/README.md"),
    ),
    (
        "wasm",
        "src/main.wj",
        include_str!("../../templates/wasm/main.wj"),
    ),
    (
        "wasm",
        "wj.toml",
        include_str!("../../templates/wasm/wj.toml"),
    ),
    (
        "wasm",
        ".gitignore",
        include_str!("../../templates/wasm/gitignore"),
    ),
    (
        "wasm",
        "README.md",
        include_str!("../../templates/wasm/README.md"),
    ),
    (
        "wasm",
        "www/index.html",
        include_str!("../../templates/wasm/www/index.html"),
    ),
];

pub const TEMPLATE_NAMES: &[&str] = &["cli", "web", "lib", "wasm"];

/// Map an on-disk template file (relative to `templates/<name>/`) to its project path.
#[cfg(test)]
fn project_path_for(rel: &str) -> String {
    match rel {
        "main.wj" | "lib.wj" => format!("src/{rel}"),
        "gitignore" => ".gitignore".to_string(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn walk(dir: &Path, base: &Path, out: &mut Vec<String>) {
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, base, out);
            } else {
                out.push(
                    p.strip_prefix(base)
                        .unwrap()
                        .to_string_lossy()
                        .replace('\\', "/"),
                );
            }
        }
    }

    #[test]
    fn embedded_table_covers_templates_dir() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates");
        for name in TEMPLATE_NAMES {
            let mut files = Vec::new();
            walk(&root.join(name), &root.join(name), &mut files);
            for f in files {
                let want = project_path_for(&f);
                let hit = EMBEDDED.iter().find(|(t, p, _)| t == name && *p == want);
                let (_, _, body) = hit.unwrap_or_else(|| {
                    panic!("templates/{name}/{f} is not embedded in new_templates.rs")
                });
                let disk = std::fs::read_to_string(root.join(name).join(&f)).unwrap();
                assert_eq!(
                    *body, disk,
                    "embedded copy of templates/{name}/{f} is stale"
                );
            }
        }
        for (t, p, _) in EMBEDDED {
            assert!(TEMPLATE_NAMES.contains(t), "unknown template {t} ({p})");
        }
    }
}
