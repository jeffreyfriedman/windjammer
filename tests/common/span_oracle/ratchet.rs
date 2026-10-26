//! Ratchet files: lists of known violations that may only shrink.
//! A file is `tests/span_oracle_ratchet/<name>.txt`, one entry per line, `#` comments allowed.
//! The gate fails when a violation is NOT listed (new bug) and when a listed entry no longer
//! violates (stale entry: delete the line, the ratchet tightens).

use std::collections::BTreeSet;

pub fn load(name: &str) -> BTreeSet<String> {
    let p = super::corpus::repo_root().join("tests/span_oracle_ratchet").join(format!("{name}.txt"));
    std::fs::read_to_string(&p)
        .unwrap_or_default()
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(str::to_string)
        .collect()
}

fn path(name: &str) -> std::path::PathBuf {
    super::corpus::repo_root().join("tests/span_oracle_ratchet").join(format!("{name}.txt"))
}

/// `SPAN_ORACLE_BLESS=1` rewrites a ratchet file to the current violations, but only to TIGHTEN
/// it (never to add entries or raise counts). A file that is still empty may be initialised.
fn bless_allowed() -> bool {
    std::env::var("SPAN_ORACLE_BLESS").is_ok_and(|v| v == "1" || v == "force")
}

/// `SPAN_ORACLE_BLESS=force` re-baselines after a merge brought in code or corpora from other
/// branches (the diff of the ratchet file is then reviewed like any other change).
fn force() -> bool {
    std::env::var("SPAN_ORACLE_BLESS").is_ok_and(|v| v == "force")
}

/// Compare `actual` violations against the ratchet file; panic with an actionable message.
pub fn enforce(name: &str, actual: &BTreeSet<String>) {
    let allowed = load(name);
    if bless_allowed() && (force() || allowed.is_empty() || actual.is_subset(&allowed)) {
        let header = "# Known violations. May only shrink: fix the code, then delete the line.\n";
        let body: String = actual.iter().map(|l| format!("{l}\n")).collect();
        std::fs::write(path(name), format!("{header}{body}")).unwrap();
        return;
    }
    let new: Vec<_> = actual.difference(&allowed).collect();
    let stale: Vec<_> = allowed.difference(actual).collect();
    let mut msg = String::new();
    if !new.is_empty() {
        msg += &format!("\nNEW violations not in tests/span_oracle_ratchet/{name}.txt (fix them; do not add):\n");
        for n in &new {
            msg += &format!("  {n}\n");
        }
    }
    if !stale.is_empty() {
        msg += &format!("\nSTALE entries in tests/span_oracle_ratchet/{name}.txt (fixed; delete these lines):\n");
        for n in &stale {
            msg += &format!("  {n}\n");
        }
    }
    assert!(msg.is_empty(), "{msg}");
}

/// Count ratchet: lines are `<class> <max count>`. Actual counts must equal the recorded ones:
/// more is a regression, fewer means the ratchet must be tightened (edit the file).
pub fn enforce_counts(name: &str, actual: &std::collections::BTreeMap<String, usize>) {
    let allowed: std::collections::BTreeMap<String, usize> = load(name)
        .into_iter()
        .filter_map(|l| {
            let (k, n) = l.rsplit_once(' ')?;
            Some((k.trim().to_string(), n.parse().ok()?))
        })
        .collect();
    if bless_allowed() && (force() || allowed.is_empty() || actual.iter().all(|(k, n)| allowed.get(k).is_some_and(|a| n <= a))) {
        let header = "# class count. Counts may only shrink: fix the code, then lower or delete the line.\n";
        let body: String = actual.iter().map(|(k, n)| format!("{k} {n}\n")).collect();
        std::fs::write(path(name), format!("{header}{body}")).unwrap();
        return;
    }
    let mut msg = String::new();
    for (k, n) in actual {
        match allowed.get(k) {
            None => msg += &format!("  NEW class {k}: {n} (not in tests/span_oracle_ratchet/{name}.txt; fix it)\n"),
            Some(a) if n > a => msg += &format!("  REGRESSION {k}: {n} > allowed {a}\n"),
            Some(a) if n < a => msg += &format!("  TIGHTEN {k}: now {n}, file says {a}; edit the ratchet down\n"),
            _ => {}
        }
    }
    for (k, a) in &allowed {
        if !actual.contains_key(k) {
            msg += &format!("  STALE {k} (allowed {a}, now 0); delete the line\n");
        }
    }
    assert!(msg.is_empty(), "\nratchet {name}:\n{msg}\nactual counts:\n{}", actual.iter().map(|(k, v)| format!("{k} {v}\n")).collect::<String>());
}
