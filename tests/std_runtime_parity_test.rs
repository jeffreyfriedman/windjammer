//! Every function a `std/*.wj` module declares must exist in the compiled runtime (`crates/windjammer-runtime`),
//! or be listed in `tests/std_runtime_gaps.txt`. The ledger may only shrink: a program that calls a listed function
//! runs under `wj run --interpret` and fails to compile under `wj build`.
//!
//! Independent oracle: the declarations come from the `.wj` sources (column-0 `fn` lines) and the runtime side from
//! the Rust sources (`pub fn` lines); neither the interpreter's registry nor the sink table is consulted.
//! `WJ_PRINT_GAPS=1` prints the current gap list (to regenerate the ledger after a deliberate change).

#[path = "common/scratch.rs"]
mod scratch;
use scratch::Scratch;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn fn_name(line: &str, prefixes: &[&str]) -> Option<String> {
    let rest = prefixes.iter().find_map(|p| line.strip_prefix(p))?;
    let name: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
    (!name.is_empty()).then_some(name)
}

/// Top-level functions declared in a `.wj` module (methods are indented and never counted).
fn declared_fns(src: &str) -> BTreeSet<String> {
    src.lines().filter_map(|l| fn_name(l, &["pub async fn ", "pub fn ", "async fn ", "fn "])).collect()
}

/// Top-level public functions of a runtime module.
fn runtime_fns(src: &str) -> BTreeSet<String> {
    let body = src.split("#[cfg(test)]").next().unwrap_or(src);
    body.lines().filter_map(|l| fn_name(l, &["pub async fn ", "pub fn "])).collect()
}

/// The runtime source of a std module: the crate-root file, plus the native platform file (`platform/native/net.rs` is
/// what `std::net` compiles to when there is no crate-root `net.rs`).
fn runtime_source(module: &str) -> Option<String> {
    let base = root().join("crates/windjammer-runtime/src");
    let top = [format!("{module}.rs"), format!("{module}_mod.rs"), format!("{module}/mod.rs")]
        .iter()
        .find_map(|f| std::fs::read_to_string(base.join(f)).ok());
    let native = std::fs::read_to_string(base.join(format!("platform/native/{module}.rs"))).ok();
    match (top, native) {
        (None, None) => None,
        (a, b) => Some(format!("{}\n{}", a.unwrap_or_default(), b.unwrap_or_default())),
    }
}

/// Methods of each `impl Type {` block at column 0, as `Type::method` (a column-0 `}` ends the block).
fn impl_methods(src: &str) -> BTreeSet<String> {
    let body = src.split("#[cfg(test)]").next().unwrap_or(src);
    let mut out = BTreeSet::new();
    let mut ty: Option<String> = None;
    for l in body.lines() {
        if let Some(rest) = l.strip_prefix("impl ") {
            let name: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
            ty = (!name.is_empty() && !rest.contains(" for ")).then_some(name);
        } else if l.starts_with('}') {
            ty = None;
        } else if let Some(t) = &ty {
            if l.starts_with("    pub ") {
                if let Some(m) = fn_name(l.trim_start(), &["pub async fn ", "pub fn "]) {
                    out.insert(format!("{t}::{m}"));
                }
            }
        }
    }
    out
}

fn current_gaps() -> BTreeSet<String> {
    let std_dir = root().join("std");
    let mut modules: Vec<(String, PathBuf)> = Vec::new();
    for e in std::fs::read_dir(&std_dir).unwrap().flatten() {
        let p = e.path();
        let name = p.file_stem().unwrap().to_string_lossy().to_string();
        if p.is_file() && p.extension().is_some_and(|x| x == "wj") {
            modules.push((name, p));
        } else if p.is_dir() && p.join("mod.wj").is_file() {
            modules.push((name, p.join("mod.wj")));
        }
    }
    let mut gaps = BTreeSet::new();
    for (module, path) in modules {
        let wj_src = std::fs::read_to_string(&path).unwrap();
        let declared = declared_fns(&wj_src);
        let rt_src = runtime_source(&module).unwrap_or_default();
        let have = runtime_fns(&rt_src);
        for name in declared.difference(&have) {
            gaps.insert(format!("{module}::{name}"));
        }
        // the methods of every type the runtime implements: each declared method needs its compiled twin
        let rt_methods = impl_methods(&rt_src);
        let rt_types: BTreeSet<&str> = rt_methods.iter().filter_map(|m| m.split("::").next()).collect();
        for m in impl_methods(&wj_src) {
            if rt_types.contains(m.split("::").next().unwrap_or("")) && !rt_methods.contains(&m) {
                gaps.insert(format!("{module}::{m}"));
            }
        }
    }
    gaps
}

fn ledger() -> BTreeSet<String> {
    let text = std::fs::read_to_string(root().join("tests/std_runtime_gaps.txt")).unwrap();
    text.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')).map(String::from).collect()
}

#[test]
fn every_declared_std_function_exists_in_the_compiled_runtime_or_is_listed() {
    let gaps = current_gaps();
    if std::env::var("WJ_PRINT_GAPS").is_ok() {
        for g in &gaps {
            println!("{g}");
        }
    }
    let known = ledger();
    let new: Vec<_> = gaps.difference(&known).collect();
    assert!(new.is_empty(), "std functions the compiled runtime lacks (add them to windjammer-runtime): {new:?}");
    let fixed: Vec<_> = known.difference(&gaps).collect();
    assert!(fixed.is_empty(), "now present in the runtime (or no longer declared): remove from tests/std_runtime_gaps.txt: {fixed:?}");
}

#[test]
fn the_scanners_see_what_they_are_meant_to_see() {
    // mutation check on the oracle itself: a declared function without a runtime twin is reported
    let declared = declared_fns("pub fn a() {}\nfn b(x: int) {}\n    fn method(self) {}\nasync fn c() {}\n");
    assert_eq!(declared, ["a", "b", "c"].iter().map(|s| s.to_string()).collect());
    let have = runtime_fns("pub fn a() {}\nfn private() {}\n    pub fn in_impl(&self) {}\npub async fn c() {}\n#[cfg(test)]\nmod t { pub fn tested() {} }\n");
    assert_eq!(have, ["a", "c"].iter().map(|s| s.to_string()).collect());
    assert_eq!(declared.difference(&have).cloned().collect::<Vec<_>>(), vec!["b".to_string()]);
    // methods: a declared method of a runtime type that the runtime lacks is visible; trait impls are not inherent methods
    let wj = impl_methods("impl S {\n    pub fn a(self) {}\n    pub fn gone(self) {}\n}\n");
    let rt = impl_methods("impl S {\n    pub fn a(&self) {}\n        pub fn nested() {}\n}\nimpl Display for S {\n    pub fn fmt(&self) {}\n}\n");
    assert_eq!(rt, ["S::a".to_string()].into_iter().collect());
    assert_eq!(wj.difference(&rt).cloned().collect::<Vec<_>>(), vec!["S::gone".to_string()]);
}

#[test]
fn the_socket_types_have_every_declared_method_compiled() {
    // ADR-0032: TcpStream, TcpListener and UdpSocket are complete in the compiled runtime, so none may be a gap
    let gaps = current_gaps();
    let socket_gaps: Vec<_> = gaps.iter().filter(|g| ["TcpStream", "TcpListener", "UdpSocket"].iter().any(|t| g.contains(t))).collect();
    assert!(socket_gaps.is_empty(), "{socket_gaps:?}");
    let declared = impl_methods(&std::fs::read_to_string(root().join("std/net/mod.wj")).unwrap());
    assert!(declared.contains("TcpStream::read_exact") && declared.contains("UdpSocket::recv_from"), "the scanner must see the socket methods");
}

#[test]
fn the_shipped_hex_and_base64_functions_are_in_the_runtime() {
    // the four functions of the original report: present in the compiled runtime, so never in the ledger
    for g in ["encoding::hex_encode_string", "encoding::hex_decode_string", "crypto::base64_encode_bytes", "crypto::base64_decode_bytes"] {
        assert!(!ledger().contains(g), "{g} must not be listed as a gap");
    }
}

const PROGRAM: &str = r#"use std::encoding
use std::crypto

fn main() {
    println(encoding.hex_encode_string("hi"))
    match encoding.hex_decode_string("6869") {
        Ok(s) => println(s),
        Err(e) => println("error ${e}"),
    }
    match encoding.hex_decode_string("zz") {
        Ok(s) => println(s),
        Err(_) => println("bad hex"),
    }
    let data: Vec<u8> = [1, 2, 3]
    println(crypto.base64_encode_bytes(data))
    match crypto.base64_decode_bytes("AQID") {
        Ok(b) => println("${b.len()} ${b[0]} ${b[2]}"),
        Err(e) => println("error ${e}"),
    }
}
"#;

/// Expected output computed by hand from the specifications: hex("hi") = 6869; base64([1,2,3]) = AQID.
const EXPECTED: &str = "6869\nhi\nbad hex\nAQID\n3 1 3\n";

fn run(args: &[&str]) -> (String, String) {
    let _scr = Scratch::new("std-parity");
    let d = _scr.path().to_path_buf();
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("prog.wj");
    std::fs::write(&f, PROGRAM).unwrap();
    let o = Command::new(WJ).arg("run").args(args).arg(&f).current_dir(&d).output().unwrap();
    let _ = std::fs::remove_dir_all(&d);
    (String::from_utf8_lossy(&o.stdout).to_string(), String::from_utf8_lossy(&o.stderr).to_string())
}

#[test]
fn hex_and_base64_helpers_interpreted_equal_expected() {
    let (out, err) = run(&["--interpret"]);
    assert_eq!(out, EXPECTED, "{err}");
}

#[test]
fn hex_and_base64_helpers_compiled_equal_expected() {
    let (out, err) = run(&[]);
    assert!(out.ends_with(EXPECTED), "stdout: {out}\nstderr: {err}");
}
