use super::*;

fn scope(names: &[&str]) -> UseScope {
    UseScope { depth: 5, names: names.iter().map(|s| s.to_string()).collect() }
}

fn meta(cwd: &Path, rel: &str, text: &str) {
    let p = cwd.join(".wj-cache").join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, text).unwrap();
}

#[test]
fn key_changes_when_a_sidecar_the_use_reaches_changes() {
    let t = tempfile::tempdir().unwrap();
    meta(t.path(), "math/vec3.wj.meta", "{}");
    meta(t.path(), "util.wj.meta", "{}");
    let s = scope(&["math", "vec3", "Vec3", "util"]);
    let k0 = fingerprint_in(t.path(), &s).unwrap();
    meta(t.path(), "math/vec3.wj.meta", "{\"a\":1}");
    let k1 = fingerprint_in(t.path(), &s).unwrap();
    assert_ne!(k0, k1, "an edited nested sidecar left the key stale");
    meta(t.path(), "util.wj.meta", "{\"b\":2}");
    assert_ne!(fingerprint_in(t.path(), &s).unwrap(), k1, "an edited top-level sidecar left the key stale");
}

#[test]
fn key_ignores_sidecars_no_use_reaches_and_mtime_only_touches() {
    let t = tempfile::tempdir().unwrap();
    meta(t.path(), "math/vec3.wj.meta", "{}");
    let s = scope(&["math", "vec3", "Vec3"]);
    let k0 = fingerprint_in(t.path(), &s).unwrap();
    for i in 0..50 {
        meta(t.path(), &format!("junk{i}.wj.meta"), "x");
        meta(t.path(), &format!("other{}/m{i}.wj.meta", i % 5), "x");
    }
    assert_eq!(fingerprint_in(t.path(), &s).unwrap(), k0, "unrelated sidecars changed the key");
    // Rule: the key is the sidecar CONTENT, so touching a file without changing its text does not change the key.
    let f = t.path().join(".wj-cache/math/vec3.wj.meta");
    std::fs::write(&f, "{}").unwrap();
    assert_eq!(fingerprint_in(t.path(), &s).unwrap(), k0, "an mtime-only touch changed the key");
}
