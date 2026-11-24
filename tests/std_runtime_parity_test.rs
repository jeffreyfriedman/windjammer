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
    // a module that publishes a private child with `pub use child::*;` (`time.rs`: `pub use calendar::*;`) defines
    // those functions under its own path, so the child's source is part of the module
    let top = top.map(|t| {
        // the parent's own test module ends its public surface: `runtime_fns` cuts the joined text at the first
        // `#[cfg(test)]`, so the parent's tests must not hide the children that follow them
        let t = t.split("#[cfg(test)]").next().unwrap_or(&t).to_string();
        let children: String = t
            .lines()
            .filter_map(|l| l.trim().strip_prefix("pub use ")?.strip_suffix("::*;").map(str::to_string))
            .filter(|c| !c.contains("::"))
            .filter_map(|c| std::fs::read_to_string(base.join(module).join(format!("{c}.rs"))).ok())
            .collect::<Vec<_>>()
            .join("\n");
        format!("{t}\n{children}")
    });
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

// ---- signatures: the declared parameter and return TYPES against the runtime's ---------------------------------------
//
// The runtime is what `wj build` links; the `.wj` declaration is the contract a program is checked against and the
// interpreter follows. A function whose two sides disagree on a type compiles under `wj run --interpret` and fails
// under rustc (`sha256_bytes` declared `Vec<u8>`, returning a hex `String`). Types are compared by KIND (text, bytes,
// integer, float, bool, unit, vec, option, result, map, tuple, named type): widths and borrows are not part of the
// contract (`string` is `&str`/`String`/`impl AsRef<str>`; `Vec<u8>` is `&[u8]`).

/// Splits `s` at commas that are outside every bracket pair.
fn split_top(s: &str) -> Vec<String> {
    let (mut depth, mut cur, mut out) = (0i32, String::new(), Vec::new());
    for c in s.chars() {
        match c {
            '<' | '(' | '[' => depth += 1,
            // `->` of a function type is not a closing bracket
            '>' if cur.ends_with('-') => {}
            '>' | ')' | ']' => depth -= 1,
            ',' if depth == 0 => {
                out.push(std::mem::take(&mut cur));
                continue;
            }
            _ => {}
        }
        cur.push(c);
    }
    if !cur.trim().is_empty() {
        out.push(cur);
    }
    out.into_iter().map(|p| p.trim().to_string()).collect()
}

/// The kind of a type written in either language.
fn kind(ty: &str) -> String {
    let mut t = ty.trim();
    loop {
        let before = t;
        t = t.strip_prefix('&').unwrap_or(t).trim_start();
        if t.starts_with('\'') {
            t = t.split_once(' ').map_or("", |x| x.1);
        }
        t = t.strip_prefix("mut ").unwrap_or(t);
        if t == before {
            break;
        }
    }
    let inner = |t: &str, head: &str| -> Option<String> { t.strip_prefix(head)?.strip_suffix('>').map(str::to_string) };
    if t.is_empty() || t == "()" || t == "!" {
        return "unit".into();
    }
    // a module's result alias (`FsResult<string>`, `DialogResult<string>`) is `Result<T, _>`
    if let Some(i) = t.strip_suffix('>').and_then(|r| r.split_once('<')).filter(|(h, _)| h.ends_with("Result") && *h != "Result").map(|(_, i)| i) {
        return format!("result<{},any>", kind(i));
    }
    if t.starts_with('(') && t.ends_with(')') {
        let parts: Vec<String> = split_top(&t[1..t.len() - 1]).iter().map(|p| kind(p)).collect();
        return format!("({})", parts.join(","));
    }
    if let Some(i) = inner(t, "impl AsRef<").or_else(|| inner(t, "impl Into<")).or_else(|| inner(t, "impl Borrow<")) {
        return kind(&i);
    }
    if t.starts_with("impl ") || t.starts_with("dyn ") || t.starts_with("fn(") || t.starts_with("fn ") {
        return "any".into();
    }
    if t.len() == 1 && t.chars().all(|c| c.is_ascii_uppercase()) {
        return "any".into();
    }
    match t {
        "string" | "String" | "str" | "Path" | "PathBuf" | "char" | "Cow<str>" => return "text".into(),
        "bool" => return "bool".into(),
        "float" | "f32" | "f64" => return "float".into(),
        "int" | "uint" | "i8" | "i16" | "i32" | "i64" | "i128" | "isize" | "u8" | "u16" | "u32" | "u64" | "u128" | "usize" => return "int".into(),
        "bytes" | "[u8]" | "Vec<u8>" => return "bytes".into(),
        _ => {}
    }
    for head in ["Vec<", "VecDeque<", "HashSet<", "BTreeSet<"] {
        if let Some(i) = inner(t, head) {
            let k = kind(&i);
            return if k == "int" && t == "Vec<u8>" { "bytes".into() } else { format!("vec<{k}>") };
        }
    }
    if let Some(i) = t.strip_prefix('[').and_then(|r| r.strip_suffix(']')) {
        let i = i.split(';').next().unwrap_or(i);
        return if kind(i) == "int" && i.trim() == "u8" { "bytes".into() } else { format!("vec<{}>", kind(i)) };
    }
    if let Some(i) = inner(t, "Option<") {
        return format!("option<{}>", kind(&i));
    }
    if let Some(i) = inner(t, "Result<") {
        let parts: Vec<String> = split_top(&i).iter().map(|p| kind(p)).collect();
        return format!("result<{}>", parts.join(","));
    }
    for head in ["HashMap<", "BTreeMap<", "Map<"] {
        if let Some(i) = inner(t, head) {
            let parts: Vec<String> = split_top(&i).iter().map(|p| kind(p)).collect();
            return format!("map<{}>", parts.join(","));
        }
    }
    // a named type: the last path segment, without generic arguments
    let head = t.split('<').next().unwrap_or(t);
    head.rsplit("::").next().unwrap_or(head).trim().to_string()
}

/// Whether two kinds agree (`any` is a generic or a closure: it agrees with every kind).
fn kinds_agree(a: &str, b: &str) -> bool {
    if a == "any" || b == "any" {
        return true;
    }
    let (sa, sb) = (split_kind(a), split_kind(b));
    match (sa, sb) {
        (Some((ha, pa)), Some((hb, pb))) => ha == hb && pa.len() == pb.len() && pa.iter().zip(&pb).all(|(x, y)| kinds_agree(x, y)),
        (None, None) => a == b,
        _ => false,
    }
}

/// `head<a,b>` / `(a,b)` as head and arguments.
fn split_kind(k: &str) -> Option<(String, Vec<String>)> {
    if let Some(i) = k.strip_prefix('(').and_then(|r| r.strip_suffix(')')) {
        return Some(("tuple".into(), split_top(i)));
    }
    let (head, rest) = k.split_once('<')?;
    Some((head.to_string(), split_top(rest.strip_suffix('>')?)))
}

/// A declared signature: parameter types (not `self`) and return type, as written.
#[derive(Debug, PartialEq, Clone)]
struct Sig {
    params: Vec<String>,
    ret: String,
}

/// The signature text starting at `lines[i]` (a `fn` line): joined up to the opening `{` (or `;`).
fn signature_text(lines: &[&str], i: usize) -> String {
    let mut text = String::new();
    for l in &lines[i..] {
        text.push_str(l.trim());
        text.push(' ');
        if l.contains('{') || l.trim_end().ends_with(';') {
            break;
        }
    }
    text
}

fn parse_sig(text: &str, name: &str) -> Option<Sig> {
    let after = &text[text.find(&format!("fn {name}"))? + 3 + name.len()..];
    let mut rest = after.trim_start();
    if rest.starts_with('<') {
        let mut depth = 0;
        let end = rest.char_indices().find(|(_, c)| {
            depth += match c { '<' => 1, '>' => -1, _ => 0 };
            depth == 0
        })?;
        rest = rest[end.0 + 1..].trim_start();
    }
    let rest = rest.strip_prefix('(')?;
    let mut depth = 1;
    let close = rest.char_indices().find(|(_, c)| {
        depth += match c { '(' => 1, ')' => -1, _ => 0 };
        depth == 0
    })?.0;
    let params: Vec<String> = split_top(&rest[..close])
        .into_iter()
        .filter(|p| !p.is_empty() && !matches!(p.as_str(), "self" | "&self" | "&mut self" | "mut self"))
        .map(|p| p.split_once(':').map_or(p.clone(), |x| x.1.trim().to_string()))
        .collect();
    let tail = rest[close + 1..].trim();
    let ret = tail.strip_prefix("->").map(|r| r.split(" where ").next().unwrap_or(r).split('{').next().unwrap_or(r).trim().trim_end_matches(';').trim().to_string()).unwrap_or_default();
    Some(Sig { params, ret })
}

/// Signatures of the top-level functions of a module, the declaration side (`fn`) or the runtime side (`pub fn`).
fn top_sigs(src: &str, runtime: bool) -> std::collections::BTreeMap<String, Sig> {
    let body = if runtime { src.split("#[cfg(test)]").next().unwrap_or(src) } else { src };
    let prefixes: &[&str] = if runtime { &["pub async fn ", "pub fn "] } else { &["pub async fn ", "pub fn ", "async fn ", "fn "] };
    let lines: Vec<&str> = body.lines().collect();
    let mut out = std::collections::BTreeMap::new();
    for (i, l) in lines.iter().enumerate() {
        if let Some(name) = fn_name(l, prefixes) {
            if let Some(sig) = parse_sig(&signature_text(&lines, i), &name) {
                out.entry(name).or_insert(sig);
            }
        }
    }
    out
}

/// `module::name#ret` / `module::name#pN` for every type the runtime function disagrees with its declaration on
/// (`#arity` when the parameter counts differ).
fn signature_mismatches() -> std::collections::BTreeMap<String, String> {
    let mut out = std::collections::BTreeMap::new();
    for (module, path) in std_modules() {
        let declared = top_sigs(&std::fs::read_to_string(&path).unwrap(), false);
        let runtime = top_sigs(&runtime_source(&module).unwrap_or_default(), true);
        for (name, d) in &declared {
            let Some(r) = runtime.get(name) else { continue };
            if d.params.len() != r.params.len() {
                out.insert(format!("{module}::{name}#arity"), format!("declared {d:?}, runtime {r:?}"));
                continue;
            }
            for (i, (a, b)) in d.params.iter().zip(&r.params).enumerate() {
                if !kinds_agree(&kind(a), &kind(b)) {
                    out.insert(format!("{module}::{name}#p{i}"), format!("declared `{a}`, runtime `{b}`"));
                }
            }
            if !kinds_agree(&kind(&d.ret), &kind(&r.ret)) {
                out.insert(format!("{module}::{name}#ret"), format!("declared `{}`, runtime `{}`", d.ret, r.ret));
            }
        }
    }
    out
}

fn std_modules() -> Vec<(String, PathBuf)> {
    let mut modules = Vec::new();
    for e in std::fs::read_dir(root().join("std")).unwrap().flatten() {
        let p = e.path();
        let name = p.file_stem().unwrap().to_string_lossy().to_string();
        if p.is_file() && p.extension().is_some_and(|x| x == "wj") {
            modules.push((name, p));
        } else if p.is_dir() && p.join("mod.wj").is_file() {
            modules.push((name, p.join("mod.wj")));
        }
    }
    modules
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

fn signature_ledger() -> BTreeSet<String> {
    let text = std::fs::read_to_string(root().join("tests/std_runtime_signature_gaps.txt")).unwrap();
    text.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')).map(String::from).collect()
}

#[test]
fn every_runtime_function_has_the_declared_parameter_and_return_types_or_is_listed() {
    let detail = signature_mismatches();
    let found: BTreeSet<String> = detail.keys().cloned().collect();
    if std::env::var("WJ_PRINT_GAPS").is_ok() {
        for (g, why) in &detail {
            println!("SIG {g}: {why}");
        }
    }
    let known = signature_ledger();
    let new: Vec<_> = found.difference(&known).collect();
    assert!(new.is_empty(), "the compiled runtime disagrees with the std declaration on a type (fix the side that breaks the wj-facing contract): {new:?}");
    let fixed: Vec<_> = known.difference(&found).collect();
    assert!(fixed.is_empty(), "types now agree: remove from tests/std_runtime_signature_gaps.txt: {fixed:?}");
}

#[test]
fn the_signature_scanner_sees_a_type_that_disagrees() {
    // mutation check on the oracle: the `sha256_bytes` shape (declared bytes, runtime text) is a mismatch; borrows,
    // widths, text spellings and generics are not
    let d = top_sigs("fn sha256_bytes(data: Vec<u8>) -> Vec<u8> {\n    vec![]\n}\nfn n(a: string, b: int) -> Result<Vec<u8>, string> {\n}\n", false);
    let bad = top_sigs("pub fn sha256_bytes(data: &[u8]) -> String {\n}\n", true);
    assert_eq!(d["sha256_bytes"].params, ["Vec<u8>"]);
    assert!(kinds_agree(&kind(&d["sha256_bytes"].params[0]), &kind(&bad["sha256_bytes"].params[0])));
    assert!(!kinds_agree(&kind(&d["sha256_bytes"].ret), &kind(&bad["sha256_bytes"].ret)));
    let ok = top_sigs("pub fn n(\n    a: impl AsRef<str>,\n    b: u64,\n) -> Result<Vec<u8>, String> {\n}\n", true);
    assert_eq!(d["n"].params.len(), ok["n"].params.len());
    assert!(kinds_agree(&kind(&d["n"].ret), &kind(&ok["n"].ret)));
    for (a, b) in d["n"].params.iter().zip(&ok["n"].params) {
        assert!(kinds_agree(&kind(a), &kind(b)), "{a} vs {b}");
    }
    // a result that became a plain value is a mismatch
    assert!(!kinds_agree(&kind("string"), &kind("Result<String, String>")));
}

/// `sha256_bytes` is declared `Vec<u8>`: the digest bytes feed `hex_encode`, compiled and interpreted alike. SHA-256 of
/// the bytes 1, 2, 3 is 039058c6f2c0cb492c533b0a4d14ef77cc0f78abccced5287d84a1a2011cfb81 (FIPS 180-4).
#[test]
fn sha256_bytes_are_bytes_compiled_and_interpreted() {
    const SRC: &str = "use std::crypto\nuse std::encoding\n\nfn main() {\n    let digest: Vec<u8> = crypto.sha256_bytes([1, 2, 3])\n    println(\"${digest.len()} ${encoding.hex_encode(digest)}\")\n}\n";
    let want = "32 039058c6f2c0cb492c533b0a4d14ef77cc0f78abccced5287d84a1a2011cfb81\n";
    for args in [&["--interpret"][..], &[][..]] {
        let scr = Scratch::new("std-sha256-bytes");
        let d = scr.path().to_path_buf();
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        let f = d.join("prog.wj");
        std::fs::write(&f, SRC).unwrap();
        let o = Command::new(WJ).arg("run").args(args).arg(&f).current_dir(&d).output().unwrap();
        let out = String::from_utf8_lossy(&o.stdout).to_string();
        assert!(out.ends_with(want), "{args:?}: stdout {out}\nstderr {}", String::from_utf8_lossy(&o.stderr));
    }
}
