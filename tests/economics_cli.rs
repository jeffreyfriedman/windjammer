//! `wj economics` counts exactly what `scripts/wj/economics.wj --static` recorded for the suite
//! (bench/economics/results/static.json), so the command and the suite never disagree on D7.

use std::path::Path;
use windjammer::cli::economics::source_counts;

#[test]
fn the_command_and_the_suite_count_the_same() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let doc: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(root.join("bench/economics/results/static.json")).unwrap()).unwrap();
    let programs = doc["programs"].as_array().unwrap();
    assert!(programs.len() >= 78);
    let paths = |id: &str| -> (String, String) {
        let (suite, name) = id.split_once('/').unwrap();
        match suite {
            "dx" => (format!("bench/agent-dx/solutions/wj/{name}/main.wj"), format!("bench/agent-dx/solutions/rust/{name}/main.rs")),
            "parity" => (format!("benchmarks/rust_parity/wj/{name}.wj"), format!("benchmarks/rust_parity/rust/{name}.rs")),
            _ => (format!("bench/interp/{name}.wj"), format!("bench/economics/twins/{name}.rs")),
        }
    };
    for p in programs {
        let id = p["id"].as_str().unwrap();
        let (wj, rs) = paths(id);
        for (path, key) in [(wj, "wj_src"), (rs, "rust_src")] {
            let c = source_counts(&std::fs::read_to_string(root.join(&path)).unwrap());
            let want = &p[key];
            let got = [c.lines, c.chars, c.tokens, c.ceremony];
            let exp = ["lines", "chars", "tokens", "ceremony"].map(|k| want[k].as_u64().unwrap() as usize);
            assert_eq!(got, exp, "{id} {key} ({path}): wj economics {got:?} vs the suite {exp:?}");
        }
    }
}
