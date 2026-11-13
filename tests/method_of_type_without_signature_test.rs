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

//! A method call on a receiver whose type is declared (`base: RailItem`) but has no Windjammer signature (a type
//! from a Rust crate) names `RailItem::active`, never the `active` of another type. Here `ui.wj` declares two
//! `active(self, id: string)` builders; the call `base.active(on)` with a `bool` must not borrow their `string`
//! formal (`on.to_string()`).
//!
//! Oracle: rustc compiles the generated crate with the real Rust `RailItem` (whose `active` takes `bool`) appended,
//! and the program prints what the source says. The project is built through the multi-file path, where the other
//! module's methods sit in the library-wide registry.

use std::fs;
use std::process::Command;

const UI: &str = r#"pub struct Tabs { pub id: string }
impl Tabs {
    pub fn new() -> Tabs { Tabs { id: "" } }
    pub fn active(self, id: string) -> Tabs { Tabs { id: id } }
}
pub struct Panel { pub id: string }
impl Panel {
    pub fn new() -> Panel { Panel { id: "" } }
    pub fn active(self, id: string) -> Panel { Panel { id: id } }
}
"#;

const MAIN: &str = r#"mod ui
use ui::Tabs
use ui::Panel

pub fn show(base: RailItem, on: bool) -> RailItem {
    base.active(on)
}

pub fn show_bound(base: RailItem) -> RailItem {
    let on = 2 > 1
    let item: RailItem = base.active(on)
    item
}

fn main() {
    let t = Tabs::new().active("home")
    let p = Panel::new().active("side")
    println("${t.id} ${p.id}")
}
"#;

/// The Rust type the Windjammer program uses without declaring it.
const RAIL_ITEM_RS: &str = r#"
pub struct RailItem { pub on: bool }
impl RailItem {
    pub fn active(self, on: bool) -> RailItem { RailItem { on } }
}
#[allow(dead_code)]
fn exercise() -> bool { show_bound(show(RailItem { on: false }, true)).on }
"#;

#[test]
fn method_on_type_without_signature_keeps_its_bool_argument() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("ui.wj"), UI).unwrap();
    fs::write(src.join("main.wj"), MAIN).unwrap();
    let out = tmp.path().join("out");
    let build = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg(&src)
        .args(["--target", "rust", "--no-cargo", "--no-lint", "--quiet", "-o"])
        .arg(&out)
        .output()
        .unwrap();
    assert!(build.status.success(), "wj build failed:\n{}", String::from_utf8_lossy(&build.stderr));

    let main_rs = fs::read_to_string(out.join("main.rs")).unwrap();
    assert!(
        main_rs.contains("base.active(on)") && !main_rs.contains("on.to_string()"),
        "`base.active(on)` must pass the bool as is\n{main_rs}"
    );

    fs::write(out.join("main.rs"), format!("{main_rs}{RAIL_ITEM_RS}")).unwrap();
    let exe = tmp.path().join("prog");
    let rustc = Command::new("rustc")
        .args(["--edition", "2021", "-A", "warnings", "--crate-name", "project", "-o"])
        .arg(&exe)
        .arg(out.join("main.rs"))
        .output()
        .unwrap();
    assert!(rustc.status.success(), "rustc rejected the crate:\n{}\n{main_rs}", String::from_utf8_lossy(&rustc.stderr));
    let run = Command::new(&exe).output().unwrap();
    assert_eq!(String::from_utf8_lossy(&run.stdout), "home side\n");
}

// Two crates: `uikit` (built, with metadata) has the `active(self, id: string)` builders; `rail_kit` (a path dependency
// without metadata) has `RailItem::active(self, on: bool)`. The app calls `RailItem::new("1000").active(flag)`: the
// receiver is built by `RailItem::new`, a type the file imports from `rail_kit`, so a `uikit` method is not it.

const UIKIT_TABS: &str = r#"pub struct Tabs { pub id: string }
impl Tabs {
    pub fn new() -> Tabs { Tabs { id: "" } }
    pub fn active(self, id: string) -> Tabs { Tabs { id: id } }
}
pub struct TabPanel { pub id: string }
impl TabPanel {
    pub fn new() -> TabPanel { TabPanel { id: "" } }
    pub fn active(self, id: string) -> TabPanel { TabPanel { id: id } }
}
"#;

const RAIL_ITEM: &str = r#"pub struct RailItem {
    code: string,
    on: bool,
}

impl RailItem {
    pub fn new(code: string) -> RailItem {
        RailItem { code: code, on: false }
    }
    pub fn active(self, on: bool) -> RailItem {
        RailItem { code: self.code, on: on }
    }
    pub fn is_on(self) -> bool {
        self.on
    }
}
"#;

const PAGE: &str = r#"use rail_kit::item::RailItem

pub fn mark(flag: bool) -> RailItem {
    RailItem::new("1000").active(flag)
}
"#;

fn wj_ok(args: &[&std::ffi::OsStr], cwd: &std::path::Path) {
    let out = Command::new(env!("CARGO_BIN_EXE_wj")).args(args).current_dir(cwd).output().unwrap();
    assert!(out.status.success(), "wj {args:?} failed:\n{}", String::from_utf8_lossy(&out.stderr));
}

fn rustc_ok(args: &[&std::ffi::OsStr], what: &str) {
    let out = Command::new("rustc").args(["--edition", "2021", "-A", "warnings", "--crate-type", "lib"]).args(args).output().unwrap();
    assert!(out.status.success(), "rustc rejected {what}:\n{}", String::from_utf8_lossy(&out.stderr));
}

#[test]
fn method_on_type_of_another_crate_ignores_a_dependency_homonym() {
    let tmp = tempfile::tempdir().unwrap();
    let t = tmp.path();
    let write = |rel: &str, text: &str| {
        let p = t.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, text).unwrap();
    };
    write("uikit/wj.toml", "[package]\nname = \"uikit\"\nversion = \"0.1.0\"\n\n[lib]\nname = \"uikit\"\n");
    write("uikit/mod.wj", "pub mod tabs\n");
    write("uikit/tabs.wj", UIKIT_TABS);
    write("comp/wj.toml", "[package]\nname = \"rail_kit\"\nversion = \"0.1.0\"\n\n[lib]\nname = \"rail_kit\"\n");
    write("comp/mod.wj", "pub mod item\n");
    write("comp/item.wj", RAIL_ITEM);
    write(
        "app/wj.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\n\n[lib]\nname = \"app\"\n\n[dependencies]\nuikit = { path = \"../uikit_gen\" }\nrail_kit = { path = \"../comp\" }\n",
    );
    write("app/mod.wj", "pub mod page\n");
    write("app/page.wj", PAGE);

    let os = |s: &str| std::ffi::OsString::from(s);
    let lib = |src: &str, out: &str| {
        wj_ok(&[&os("build"), t.join(src).as_os_str(), &os("--output"), t.join(out).as_os_str(), &os("--library"), &os("--no-cargo"), &os("--module-file")], t);
    };
    lib("uikit", "uikit_gen");
    lib("comp", "comp_gen");
    wj_ok(&[&os("build"), &os("."), &os("-o"), t.join("app_gen").as_os_str(), &os("--no-cargo"), &os("--module-file")], &t.join("app"));

    let page = fs::read_to_string(t.join("app_gen/page.rs")).unwrap();
    assert!(page.contains(".active(flag)") && !page.contains("flag.to_string()"), "`.active(flag)` must pass the bool as is\n{page}");

    let rlib = t.join("librail_kit.rlib");
    rustc_ok(&[&os("--crate-name"), &os("rail_kit"), t.join("comp_gen/lib.rs").as_os_str(), &os("-o"), rlib.as_os_str()], "rail_kit");
    let extern_arg = std::ffi::OsString::from(format!("rail_kit={}", rlib.display()));
    rustc_ok(
        &[&os("--crate-name"), &os("app"), t.join("app_gen/lib.rs").as_os_str(), &os("--extern"), &extern_arg, &os("-o"), t.join("libapp.rlib").as_os_str()],
        "the app crate",
    );
}
