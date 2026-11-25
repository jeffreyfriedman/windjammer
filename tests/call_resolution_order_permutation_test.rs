#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

//! Call resolution is by the receiver's type and never depends on file order.
//!
//! Invariant: a method call resolves only against signatures of the receiver's type (or the callee path). When the
//! receiver's type is not known yet, a bare method name has as candidates every layer's `*::name` (this file, the
//! project, the std library), so a user method never answers for a std method of the same name and a static fn never
//! answers for a method. The candidate set is a set over all layers, so the result does not depend on which file was
//! analysed or generated first.
//!
//! Gate: each project is built under several file-name orderings (the module names are permuted, so a different file
//! sorts first each time). After the module names are mapped back, every generated file must be byte-identical, and
//! each call site must take its own receiver's formals.

#[path = "common/test_utils.rs"]
mod test_utils;

use std::collections::BTreeMap;

/// Module names that sort in a known order; slot `{M<i>}` of a template takes `names[i]`.
const POOL: [&str; 4] = ["aa_mod", "mm_mod", "qq_mod", "zz_mod"];

/// Builds `files` (templates: `{M0}`..`{M3}` name the modules, in file names and in `use` paths) once per rotation
/// of the module names. Returns, per rotation, each file's generated Rust with module names mapped back to `{M<i>}`.
fn build_rotations(files: &[(&str, &str)], rotations: usize) -> Vec<BTreeMap<String, String>> {
    let slots = files.len();
    (0..rotations)
        .map(|r| {
            let names: Vec<&str> = (0..slots).map(|i| POOL[(i + r) % POOL.len()]).collect();
            let fill = |text: &str| {
                let mut out = text.to_string();
                for (i, n) in names.iter().enumerate() {
                    out = out.replace(&format!("{{M{i}}}"), n);
                }
                out
            };
            let owned: Vec<(String, String)> =
                files.iter().map(|(name, body)| (fill(name), fill(body))).collect();
            let borrowed: Vec<(&str, &str)> = owned.iter().map(|(n, b)| (n.as_str(), b.as_str())).collect();
            let (map, ok) = test_utils::compile_project_dir(&borrowed);
            assert!(ok, "rotation {r} ({names:?}) must transpile");
            files
                .iter()
                .map(|(template, _)| {
                    let rs = fill(template).replace(".wj", ".rs");
                    let mut text = map.get(&rs).cloned().unwrap_or_else(|| panic!("no {rs} in rotation {r}"));
                    for (i, n) in names.iter().enumerate() {
                        text = text.replace(n, &format!("{{M{i}}}"));
                    }
                    (template.replace(".wj", ".rs"), text)
                })
                .collect()
        })
        .collect()
}

/// Every rotation generated the same text for every file; returns the first rotation's output.
fn assert_order_independent(files: &[(&str, &str)]) -> BTreeMap<String, String> {
    let runs = build_rotations(files, 3);
    for (r, run) in runs.iter().enumerate().skip(1) {
        for (file, text) in run {
            assert_eq!(
                &runs[0][file], text,
                "{file}: rotation {r} generated different Rust than rotation 0 (file order leaked into resolution)"
            );
        }
    }
    runs.into_iter().next().unwrap()
}

fn assert_has(out: &BTreeMap<String, String>, file: &str, needle: &str) {
    let text = &out[file];
    assert!(text.contains(needle), "{file} must contain `{needle}`. Got:\n{text}");
}

/// Reduced B: an extension method `Realm::set_variable` (borrowed formals) in a file that imports the caller's file,
/// and a static `Effect::set_variable` (owned formals) next to the caller.
#[test]
fn extension_method_vs_static_homonym_is_order_independent() {
    let out = assert_order_independent(&[
        ("{M0}.wj", "pub struct Realm {\n    pub tick: i32,\n}\n"),
        (
            "{M1}.wj",
            "use crate::{M0}::Realm\nuse crate::{M2}::Effect\n\npub struct Runner {\n    pending: Option<Effect>,\n}\n\nimpl Realm {\n    pub fn set_variable(self, key: string, value: string) {\n    }\n}\n",
        ),
        (
            "{M2}.wj",
            "use crate::{M0}::Realm\n\npub enum Effect {\n    SetVariable { key: string, value: string },\n}\n\nimpl Effect {\n    pub fn set_variable(key: string, value: string) -> Effect {\n        Effect::SetVariable { key: key, value: value }\n    }\n\n    pub fn apply(self, realm: Realm) {\n        match self {\n            Effect::SetVariable { key, value } => {\n                realm.set_variable(key, value)\n            }\n        }\n    }\n}\n",
        ),
    ]);
    assert_has(&out, "{M2}.rs", "realm.set_variable(&key, &value)");
}

/// Reduced E: a literal into an owned `string` formal (`note_item` stores it with `Vec::push`) while a user method
/// `CallStack::push(self, frame: string)` exists; the caller's file sorts first in one rotation and last in another.
#[test]
fn literal_into_owned_formal_is_order_independent() {
    let out = assert_order_independent(&[
        ("{M0}.wj", "pub struct Greeter {\n    volume: f32,\n}\n\nimpl Greeter {\n    pub fn announce(self, ctx: Host) {\n        ctx.note_item(\"ready\")\n    }\n}\n"),
        ("{M1}.wj", "pub struct Host {\n    registry: Vec<string>,\n}\n\nimpl Host {\n    pub fn note_item(self, name: string) {\n        self.registry.push(name)\n    }\n}\n"),
        ("{M2}.wj", "pub struct CallStack {\n    frames: Vec<string>,\n}\n\nimpl CallStack {\n    pub fn push(self, frame: string) {\n        self.frames.push(frame)\n    }\n}\n"),
    ]);
    assert_has(&out, "{M0}.rs", "ctx.note_item(\"ready\".to_string())");
    assert_has(&out, "{M1}.rs", "pub fn note_item(&mut self, name: String)");
}

/// Homonym matrix: `record` on three unrelated user types (owned method, borrowed method, owned static fn) and
/// `contains` / `push` on a user type and on the builtin `Vec`. Each call site takes its own receiver's formals,
/// under every file order.
#[test]
fn homonym_matrix_each_call_resolves_to_its_receiver() {
    let out = assert_order_independent(&[
        // Owned: the key is stored.
        ("{M0}.wj", "pub struct Ledger {\n    keys: Vec<string>,\n}\n\nimpl Ledger {\n    pub fn record(self, key: string) {\n        self.keys.push(key)\n    }\n\n    pub fn contains(self, id: i64) -> bool {\n        id > 0\n    }\n}\n"),
        // Borrowed: the key is only read.
        // `tag` is also a free fn with owned formals of the same shape (registered under the bare key `tag`).
        ("{M1}.wj", "pub struct Probe {\n    pub hits: i64,\n}\n\nimpl Probe {\n    pub fn record(self, key: string) -> bool {\n        key.len() > 0\n    }\n\n    pub fn push(self, frame: string) -> bool {\n        frame.len() > 1\n    }\n\n    pub fn tag(self, a: string, b: string) -> bool {\n        a.len() > b.len()\n    }\n}\n\npub fn tag(a: string, b: string) -> Vec<string> {\n    vec![a, b]\n}\n"),
        // Static fn of the same name: owned, stored in the result.
        ("{M2}.wj", "pub struct Counter {\n    pub label: string,\n}\n\nimpl Counter {\n    pub fn record(key: string) -> Counter {\n        Counter { label: key }\n    }\n}\n"),
        // Caller: every homonym once, plus the builtin `Vec::contains` / `Vec::push` on a late-inferred local.
        ("{M3}.wj", "pub fn drive(ledger: Ledger, probe: Probe, ids: Vec<i64>) -> Counter {\n    ledger.record(\"owned\")\n    let seen_probe = probe.record(\"shared\")\n    let pushed = probe.push(\"frame\") && probe.tag(\"x\", \"y\")\n    let mut seen = Vec::new()\n    let mut j = 0\n    while j < ids.len() {\n        let id = ids[j]\n        if !seen.contains(id) && ledger.contains(id) {\n            seen.push(id)\n        }\n        j = j + 1\n    }\n    let mut names = Vec::new()\n    names.push(\"n\")\n    if seen_probe && pushed && seen.len() > names.len() {\n        return Counter::record(\"static\")\n    }\n    Counter::record(\"other\")\n}\n"),
    ]);
    assert_has(&out, "{M3}.rs", "ledger.record(\"owned\".to_string())");
    assert_has(&out, "{M3}.rs", "probe.record(\"shared\")");
    assert_has(&out, "{M3}.rs", "probe.push(\"frame\")");
    assert_has(&out, "{M3}.rs", "probe.tag(\"x\", \"y\")");
    assert_has(&out, "{M3}.rs", "seen.contains(&id)");
    assert_has(&out, "{M3}.rs", "ledger.contains(id)");
    assert_has(&out, "{M3}.rs", "Counter::record(\"static\".to_string())");
}
