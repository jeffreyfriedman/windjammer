#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
    feature = "integration_tests",
))]

//! A `wj.toml` path dependency that ships no `metadata.json` (only its Windjammer sources): the consumer reads the
//! dependency's formals from those sources, never assumes owned `String`s. `Panel::width(self, w: string)` only
//! reads `w`, so the dependency emits `w: &str`; the consumer must pass `"100%"`, not `String::from("100%")`.
//! Formals stored into a field (`Tag::named(name: string)` -> `name: String`, `rename` -> `impl Into<String>`) keep
//! their owned contract at the call site.
//!
//! Oracle: rustc compiles the dependency's generated crate and the consumer's against it, and a program calling
//! the consumer prints what the sources compute. The consumer's generated code must be the same whether the
//! dependency was built beforehand (its `metadata.json`) or not.

use std::fs;
use std::path::Path;
use std::process::Command;

const PANEL: &str = r#"pub struct Panel {
    pub css: string,
}

impl Panel {
    pub fn new() -> Panel {
        Panel { css: "" }
    }
    pub fn width(self, w: string) -> Panel {
        Panel { css: self.css + "width:" + w + ";" }
    }
    pub fn label(self, text: string) -> Panel {
        Panel { css: self.css + text }
    }
}
"#;

const TAG: &str = r#"pub struct Tag {
    pub name: string,
    pub items: Vec<string>,
}

impl Tag {
    pub fn named(name: string) -> Tag {
        Tag { name: name, items: [] }
    }
    pub fn rename(self, name: string) -> Tag {
        let mut t = self
        t.name = name
        t
    }
    pub fn push(self, item: string) -> Tag {
        let mut t = self
        t.items.push(item)
        t
    }
}
"#;

const PAGE: &str = r#"use sizing::panel::Panel
use sizing::tag::Tag

pub fn wide() -> Panel {
    Panel::new().width("100%").label("hi")
}

pub fn sized(w: string) -> Panel {
    Panel::new().width(w)
}

pub fn tagged(n: string, item: string) -> Tag {
    Tag::named("a").rename(n).push(item)
}
"#;

fn write(root: &Path, rel: &str, text: &str) {
    let p = root.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, text).unwrap();
}

fn wj(args: &[&str], cwd: &Path) {
    let out = Command::new(env!("CARGO_BIN_EXE_wj")).args(args).current_dir(cwd).output().unwrap();
    assert!(out.status.success(), "wj {args:?} failed:\n{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
}

fn rustc(args: &[&str], cwd: &Path, what: &str) {
    let out = Command::new("rustc").args(["--edition", "2021", "-A", "warnings"]).args(args).current_dir(cwd).output().unwrap();
    assert!(out.status.success(), "rustc rejected {what}:\n{}", String::from_utf8_lossy(&out.stderr));
}

/// A workspace with the dependency `sizing` (sources only) and the consumer `app`; `dep_path` is what the
/// consumer's `wj.toml` points at.
fn workspace(root: &Path, dep_path: &str) {
    write(root, "sizing/wj.toml", "[package]\nname = \"sizing\"\nversion = \"0.1.0\"\n\n[lib]\nname = \"sizing\"\n");
    write(root, "sizing/src/lib.wj", "pub mod panel\npub mod tag\n");
    write(root, "sizing/src/panel.wj", PANEL);
    write(root, "sizing/src/tag.wj", TAG);
    write(
        root,
        "app/wj.toml",
        &format!("[package]\nname = \"app\"\nversion = \"0.1.0\"\n\n[lib]\nname = \"app\"\n\n[dependencies]\nsizing = {{ path = \"{dep_path}\" }}\n"),
    );
    write(root, "app/src/lib.wj", "pub mod page\n");
    write(root, "app/src/page.wj", PAGE);
}

#[test]
fn path_dependency_without_metadata_is_read_from_its_sources() {
    let tmp = tempfile::tempdir().unwrap();
    let t = tmp.path();
    workspace(t, "../sizing");
    assert!(!t.join("sizing/metadata.json").exists() && !t.join("sizing/src/metadata.json").exists());
    wj(&["build", "src", "--no-cargo", "--module-file", "-o", "../app_gen"], &t.join("app"));
    let page = fs::read_to_string(t.join("app_gen/page.rs")).unwrap();
    assert!(page.contains(".width(\"100%\")") && !page.contains("String::from(\"100%\")"), "the dependency's `w: &str` must be read, not guessed:\n{page}");
    // `use sizing::..` names the declared dependency, even with the dependency's directory (`sizing/`) beside the
    // build output (`app_gen/`).
    assert!(page.contains("use sizing::panel::Panel;") && !page.contains("crate::sizing"), "{page}");

    // Same consumer code as with the dependency's own metadata (built beforehand).
    let built = tempfile::tempdir().unwrap();
    let b = built.path();
    workspace(b, "../sizing_gen");
    wj(&["build", "src", "--library", "--no-cargo", "-o", "../sizing_gen"], &b.join("sizing"));
    wj(&["build", "src", "--no-cargo", "--module-file", "-o", "../app_gen"], &b.join("app"));
    assert_eq!(page, fs::read_to_string(b.join("app_gen/page.rs")).unwrap());

    // rustc oracle: the dependency's crate, the consumer against it, and a program using both.
    rustc(&["--crate-type", "lib", "--crate-name", "sizing", "sizing_gen/lib.rs", "-o", "libsizing.rlib"], b, "the dependency");
    rustc(&["--crate-type", "lib", "--crate-name", "app", "app_gen/lib.rs", "--extern", "sizing=libsizing.rlib", "-o", "libapp.rlib"], b, "the consumer");
    write(
        b,
        "main.rs",
        "fn main() { let t = app::tagged(\"n\".to_string(), \"i\".to_string()); println!(\"{} {} {} {}\", app::wide().css, app::sized(\"5px\").css, t.name, t.items.len()); }\n",
    );
    rustc(&["main.rs", "--extern", "app=libapp.rlib", "--extern", "sizing=libsizing.rlib", "-L", ".", "-o", "prog"], b, "the program");
    let run = Command::new(b.join("prog")).output().unwrap();
    assert_eq!(String::from_utf8_lossy(&run.stdout), "width:100%;hi width:5px; n 1\n");
}

/// A dependency whose sources cannot be analyzed stops the build with an error naming it, never guessed formals.
#[test]
fn unanalyzable_path_dependency_is_reported_by_name() {
    let tmp = tempfile::tempdir().unwrap();
    let t = tmp.path();
    write(t, "broken/wj.toml", "[package]\nname = \"broken\"\nversion = \"0.1.0\"\n");
    write(t, "broken/src/lib.wj", "pub fn width(self w: string -> {\n");
    write(t, "app/wj.toml", "[package]\nname = \"app\"\nversion = \"0.1.0\"\n\n[dependencies]\nbroken = { path = \"../broken\" }\n");
    write(t, "app/src/lib.wj", "pub fn one() -> int {\n    1\n}\n");
    let out = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args(["build", "src", "--no-cargo", "--module-file", "-o", "../app_gen"])
        .current_dir(t.join("app"))
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("path dependency `broken`"), "the dependency must be named:\n{stderr}");
    assert!(!out.status.success(), "a dependency without signatures stops the build (fail closed)");
}
