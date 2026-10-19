#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "interpreter_tests",
))]

//! ADR-0021 D6: `HashMap`/`HashSet` keep Rust's unspecified iteration order. The interpreter iterates
//! them in key order by default; `--hash-seed N` permutes that order deterministically.
//!
//! * `hash_seed_permutes_hash_collections_only`: the permutation changes `HashMap`/`HashSet` order,
//!   never `BTreeMap`/`BTreeSet` order, and a seed reproduces.
//! * `hash_order_gate`: every verified example and spec program that mentions a hash collection runs
//!   under several seeds; a program whose output changes depends on the unspecified order and must be
//!   listed in `tests/data/hash_order_dependent.txt` (shrink-only) until it is fixed.
//! * `ordered_collections_match_a_python_model`: `OrderedMap`/`OrderedSet` against Python dict/list.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

fn run_src(src: &str, flags: &[&str]) -> (String, String, i32) {
    let dir = TempDir::new().unwrap();
    let f = dir.path().join("p.wj");
    fs::write(&f, src).unwrap();
    let o = Command::new(WJ).args(["run", "--interpret"]).args(flags).arg(&f).current_dir(dir.path()).output().unwrap();
    (String::from_utf8_lossy(&o.stdout).to_string(), String::from_utf8_lossy(&o.stderr).to_string(), o.status.code().unwrap_or(-1))
}

const KEYS: &str = r#"
use std::collections::{HashMap, HashSet, BTreeMap, BTreeSet}
fn main() {
    let mut h: HashMap<int, int> = HashMap::new()
    let mut s: HashSet<int> = HashSet::new()
    let mut b: BTreeMap<int, int> = BTreeMap::new()
    let mut t: BTreeSet<int> = BTreeSet::new()
    for i in 0..12 {
        h.insert(i, i * i)
        s.insert(i)
        b.insert(i, i * i)
        t.insert(i)
    }
    let mut hk = ""
    for (k, _) in h { hk = "${hk}${k} " }
    let mut sk = ""
    for k in s { sk = "${sk}${k} " }
    let mut bk = ""
    for (k, _) in b { bk = "${bk}${k} " }
    let mut tk = ""
    for k in t { tk = "${tk}${k} " }
    println(hk)
    println(sk)
    println(bk)
    println(tk)
}
"#;

#[test]
fn hash_seed_permutes_hash_collections_only() {
    let lines = |flags: &[&str]| -> Vec<String> { run_src(KEYS, flags).0.lines().map(str::to_string).collect() };
    let base = lines(&[]);
    assert_eq!(base[0], "0 1 2 3 4 5 6 7 8 9 10 11 ", "default order is key order");
    let s1 = lines(&["--hash-seed", "1"]);
    let s2 = lines(&["--hash-seed", "2"]);
    assert_ne!(s1[0], base[0], "seed 1 must reorder the HashMap");
    assert_ne!(s1[1], base[1], "seed 1 must reorder the HashSet");
    assert_ne!(s1[0], s2[0], "different seeds give different orders");
    assert_eq!(s1, lines(&["--hash-seed", "1"]), "a seed reproduces");
    for l in [&s1, &s2] {
        assert_eq!(l[2], base[2], "BTreeMap order is sorted under every seed");
        assert_eq!(l[3], base[3], "BTreeSet order is sorted under every seed");
        // same elements, different order
        let norm = |s: &str| s.split_whitespace().map(|x| x.parse::<i32>().unwrap()).collect::<BTreeSet<_>>();
        assert_eq!(norm(&l[0]), norm(&base[0]));
        assert_eq!(norm(&l[1]), norm(&base[1]));
    }
}

fn wj_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    let mut entries: Vec<_> = rd.flatten().map(|e| e.path()).collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            wj_files(&p, out);
        } else if p.extension().is_some_and(|e| e == "wj") {
            out.push(p);
        }
    }
}

fn run_file(p: &Path, seed: &str) -> (Vec<u8>, Option<i32>) {
    let o = Command::new(WJ)
        .args(["run", "--interpret", "--deterministic-time", "--hash-seed", seed])
        .arg(p)
        .current_dir(p.parent().unwrap())
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap();
    (o.stdout, o.status.code())
}

#[test]
fn hash_order_gate() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut programs = Vec::new();
    for d in ["examples/verified", "tests/spec", "tests/conformance_programs"] {
        wj_files(&root.join(d), &mut programs);
    }
    let mut varying = BTreeSet::new();
    let mut checked = 0;
    for p in &programs {
        let src = fs::read_to_string(p).unwrap_or_default();
        let hashy = src.contains("HashMap") || src.contains("HashSet") || src.contains("MapLiteral");
        if !hashy || !src.contains("fn main") || src.contains("stdin") {
            continue;
        }
        checked += 1;
        let base = run_file(p, "0");
        if ["1", "2", "3"].iter().any(|s| run_file(p, s) != base) {
            varying.insert(p.strip_prefix(root).unwrap().to_string_lossy().to_string());
        }
    }
    assert!(checked > 5, "too few programs with hash collections checked: {checked}");
    let list = root.join("tests/data/hash_order_dependent.txt");
    if std::env::var("WJ_UPDATE_HASH_GATE").is_ok() {
        let head = fs::read_to_string(&list).unwrap().lines().filter(|l| l.starts_with('#')).collect::<Vec<_>>().join("\n");
        fs::write(&list, format!("{head}\n{}\n", varying.iter().cloned().collect::<Vec<_>>().join("\n"))).unwrap();
        return;
    }
    let allowed: BTreeSet<String> =
        fs::read_to_string(&list).unwrap().lines().filter(|l| !l.starts_with('#') && !l.trim().is_empty()).map(str::to_string).collect();
    let new: Vec<_> = varying.difference(&allowed).collect();
    let fixed: Vec<_> = allowed.difference(&varying).collect();
    assert!(new.is_empty(), "programs whose output depends on HashMap/HashSet order (use BTreeMap/OrderedMap or sort): {new:?}");
    assert!(fixed.is_empty(), "no longer order-dependent; delete from tests/data/hash_order_dependent.txt: {fixed:?}");
}

const ORDERED_WJ: &str = r#"
fn main() {
    let mut m: OrderedMap<string, int> = OrderedMap::new()
    m.insert("c", 3)
    m.insert("a", 1)
    m.insert("b", 2)
    println("${m.insert("a", 10).unwrap_or(-1)} ${m.insert("z", 26).unwrap_or(-1)}")
    println("${m.len()} ${m.contains_key("a")} ${m.get("zz").is_none()}")
    for (k, v) in m { println("${k}=${v}") }
    println("${m.remove("c").unwrap_or(-1)}")
    for k in m.keys() { println(k) }
    println("${m.swap_remove("a").unwrap_or(-1)}")
    for (k, v) in m { println("${k}=${v}") }
    let mut s: OrderedSet<int> = OrderedSet::new()
    for x in [5, 3, 5, 9, 3, 1] { println("${s.insert(x)}") }
    println("${s.remove(3)} ${s.remove(42)} ${s.len()}")
    for x in s { println("${x}") }
    println(format("{:?}", s))
    println(format("{:?}", m))
}
"#;

const ORDERED_PY: &str = r#"
m = {}
def ins(k, v):
    old = m.get(k, -1)
    m[k] = v
    return old
ins("c", 3); ins("a", 1); ins("b", 2)
print(ins("a", 10), ins("z", 26))
print(len(m), str("a" in m).lower(), str("zz" not in m).lower())
for k, v in m.items(): print(f"{k}={v}")
print(m.pop("c"))
for k in m: print(k)
# swap_remove: last entry takes the removed slot
keys = list(m.keys()); i = keys.index("a"); v = m["a"]
keys[i] = keys[-1]; keys.pop()
m = {k: m[k] for k in keys}
print(v)
for k, v2 in m.items(): print(f"{k}={v2}")
s = []
for x in [5, 3, 5, 9, 3, 1]:
    new = x not in s
    if new: s.append(x)
    print("true" if new else "false")
r3 = 3 in s
if r3: s.remove(3)
print("true" if r3 else "false", "false", len(s))
for x in s: print(x)
print("{" + ", ".join(map(str, s)) + "}")
print("{" + ", ".join(f'"{k}": {v}' for k, v in m.items()) + "}")
"#;

#[test]
fn ordered_collections_match_a_python_model() {
    let Ok(o) = Command::new("python3").arg("-I").arg("-c").arg(ORDERED_PY).output() else { return };
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let (out, err, code) = run_src(ORDERED_WJ, &[]);
    assert_eq!(code, 0, "{err}");
    assert_eq!(out, String::from_utf8_lossy(&o.stdout), "{err}");
}
