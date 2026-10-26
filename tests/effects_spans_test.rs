//! The locations of the effect diagnostics (WJ0040 to WJ0048) are well-formed: every span `wj check --json` reports for them lies
//! inside the source it names (`error::span_check::validate_report_spans`), and every one of them has a primary span.

use std::fs;
use windjammer::check::{check_path, CheckOptions};

struct Case {
    code: &'static str,
    manifest: &'static str,
    source: &'static str,
}

const CASES: &[Case] = &[
    Case { code: "WJ0040", manifest: "[capabilities]\n", source: "use std::process\nfn main() {\n    println(\"${process::run(\"ls\") ?? \"\"}\")\n}\n" },
    Case { code: "WJ0041", manifest: "[capabilities]\nnet.connect = [\"api.example.com\"]\n", source: "use std::http\nfn main() {\n    println(\"${http::get(\"http://evil.example/\").is_ok()}\")\n}\n" },
    Case { code: "WJ0042", manifest: "", source: "use std::fs\n@pure\nfn size() -> int {\n    fs::read_to_string(\"x\").unwrap_or(\"\").len()\n}\nfn main() {\n    println(\"${size()}\")\n}\n" },
    Case {
        code: "WJ0043",
        manifest: "[capabilities]\neffects = [\"ledger.post\"]\n[policy]\n\"ledger.post\" = { only_via = [\"billing\"] }\n",
        source: "mod ledger {\n    @effect(\"ledger.post\")\n    pub fn post(amount: int) { println(\"posted ${amount}\") }\n}\nmod billing {\n    pub fn charge(amount: int) { super::ledger::post(amount) }\n}\nmod reports {\n    pub fn adjust(amount: int) { super::ledger::post(amount) }\n}\nfn main() {\n    billing::charge(5)\n    reports::adjust(1)\n}\n",
    },
    Case { code: "WJ0044", manifest: "[capabilities]\n", source: "extern fn native_open(path: string) -> int\nfn main() {\n    println(\"${native_open(\"db\")}\")\n}\n" },
    Case {
        code: "WJ0045",
        manifest: "",
        source: "use std::random\nfn make_counter() -> fn() -> int {\n    let n = random::range(0, 9)\n    move || n\n}\ndurable fn pick() -> int {\n    let c = make_counter()\n    c()\n}\nfn main() {\n    println(\"${pick()}\")\n}\n",
    },
    Case { code: "WJ0045", manifest: "", source: "durable fn keep(f: fn() -> int) -> int {\n    1\n}\nfn main() {\n    println(\"${keep(|| 1)}\")\n}\n" },
    Case {
        code: "WJ0046",
        manifest: "",
        source: "use std::random\nfn add_noise(acc: Vec<int>) { acc.push(random::range(0, 9)) }\ndurable fn sample() -> int {\n    let mut acc = []\n    add_noise(acc)\n    acc.len()\n}\nfn main() {\n    println(\"${sample()}\")\n}\n",
    },
    Case { code: "WJ0047", manifest: "", source: "use std::sync\ndurable fn consume(rx: sync::Receiver<int>) -> int {\n    rx.recv()\n}\nfn main() {\n    let (tx, rx) = sync::channel()\n    tx.send(1)\n    println(\"${consume(rx)}\")\n}\n" },
    Case {
        code: "WJ0048",
        manifest: "",
        source: "use std::collections::HashMap\ndurable fn first_key(m: HashMap<string, int>) -> string {\n    let mut out = \"\"\n    for (k, v) in m {\n        if out == \"\" { out = k }\n    }\n    out\n}\nfn main() {\n    let mut m = HashMap::new()\n    m.insert(\"a\", 1)\n    println(first_key(m))\n}\n",
    },
    // an effect only inside an interpolation, and a tool call (code mode): same rules, same span quality
    Case { code: "WJ0040", manifest: "[capabilities]\n", source: "use std::process\nfn listing() -> string {\n    \"ls says ${process::run(\"ls\") ?? \"\"}\"\n}\nfn main() {\n    println(listing())\n}\n" },
    Case { code: "WJ0040", manifest: "[capabilities]\n", source: "use std::mcp\nfn main() {\n    println(\"${mcp::call(\"demo\", \"ping\", \"{}\") ?? \"\"}\")\n}\n" },
];

#[test]
fn effect_diagnostics_have_well_formed_spans() {
    let mut problems = Vec::new();
    for (i, c) in CASES.iter().enumerate() {
        let dir = tempfile::tempdir().unwrap();
        if !c.manifest.is_empty() {
            fs::write(dir.path().join("wj.toml"), format!("[package]\nname = \"p\"\nversion = \"0.1.0\"\n\n{}", c.manifest)).unwrap();
        }
        let file = dir.path().join("main.wj");
        fs::write(&file, c.source).unwrap();
        let checked = check_path(dir.path(), &CheckOptions { use_cache: false }).unwrap();
        let mine: Vec<_> = checked.report.diagnostics.iter().filter(|d| d.code == c.code).collect();
        if mine.is_empty() {
            problems.push(format!("case {i}: no {} reported: {:?}", c.code, checked.report.diagnostics.iter().map(|d| d.code.clone()).collect::<Vec<_>>()));
            continue;
        }
        for d in &mine {
            if d.primary_span.is_none() {
                problems.push(format!("case {i}: {} has no primary span: {}", c.code, d.message));
            }
        }
        let json = serde_json::to_value(&checked.report).unwrap();
        for v in windjammer::error::span_check::validate_report_spans(&json, &|f| checked.sources.get(f).cloned().or_else(|| fs::read_to_string(f).ok())) {
            problems.push(format!("case {i} ({}): {v}", c.code));
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}
