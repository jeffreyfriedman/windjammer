//! `wj check` keys its run cache on the sidecars the program's `use`s can reach, never on the whole `.wj-cache`:
//! a program with `use std::collections::HashMap` costs the same whether the working directory's `.wj-cache` holds
//! 500 unrelated files or none (counted: `meta-scan: ambient` reads and `meta-scan: dir` listings, not wall time).

use std::process::Command;

const PROGRAM: &str = "use std::collections::HashMap\n\nfn main() {\n    let m: HashMap<string, int> = HashMap::new()\n    println(\"${m.len()}\")\n}\n";

fn counts(junk: usize) -> (usize, usize, String) {
    let t = tempfile::tempdir().unwrap();
    std::fs::write(t.path().join("main.wj"), PROGRAM).unwrap();
    let c = t.path().join(".wj-cache");
    std::fs::create_dir_all(&c).unwrap();
    for i in 0..junk {
        let d = c.join(format!("area{}", i % 10));
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join(format!("m{i}.wj.meta")), "{}").unwrap();
        std::fs::write(c.join(format!("top{i}.wj.meta")), "{}").unwrap();
    }
    let o = Command::new(env!("CARGO_BIN_EXE_wj")).args(["check", "main.wj"]).current_dir(t.path()).env("WJ_META_SCAN_STATS", "1").output().unwrap();
    let err = String::from_utf8_lossy(&o.stderr).to_string();
    let n = |k: &str| err.lines().filter(|l| *l == format!("meta-scan: {k}")).count();
    (n("ambient"), n("dir"), err.lines().filter(|l| !l.starts_with("meta-scan:")).collect::<Vec<_>>().join("\n"))
}

#[test]
fn five_hundred_unrelated_cache_files_cost_nothing() {
    let (a0, d0, out0) = counts(0);
    let (a1, d1, out1) = counts(500);
    assert_eq!((a0, d0), (a1, d1), "ambient reads/dirs listed: empty .wj-cache {:?}, 500 files {:?}", (a0, d0), (a1, d1));
    assert_eq!(out0, out1, "output differs with a populated .wj-cache");
}
