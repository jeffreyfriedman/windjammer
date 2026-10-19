//! `wj.toml [capabilities]` -> [`Grant`]. The grant model itself is shared with the runtime (`core`); only the
//! TOML reading lives here, so the runtime needs no TOML parser.

use super::model::grant::{known_effect_names, Grant};
use super::model::kind::EffectKind;
use std::path::Path;

/// Build from the `[capabilities]` table. Unknown keys are an error (a typo must not silently deny).
pub fn from_table(table: &toml::value::Table, root: &Path) -> Result<Grant, String> {
    let mut g = Grant::sandbox(root);
    let mut flat: Vec<(String, &toml::Value)> = Vec::new();
    flatten("", table, &mut flat);
    for (key, val) in flat {
        match key.as_str() {
            "deny" => {
                for n in strings(&key, val)? {
                    g.deny(EffectKind::parse_builtin(&n).ok_or_else(|| format!("[capabilities] deny: unknown effect `{n}`"))?);
                }
            }
            "effects" => {
                for n in strings(&key, val)? {
                    g.allow_all(EffectKind::User(n));
                }
            }
            _ => {
                let kind = EffectKind::parse_builtin(&key)
                    .ok_or_else(|| format!("[capabilities]: unknown key `{key}` (known: {}, effects, deny)", known_effect_names()))?;
                match val {
                    toml::Value::Boolean(true) => g.allow_all(kind),
                    toml::Value::Boolean(false) => g.deny(kind),
                    _ => {
                        let pats = strings(&key, val)?;
                        if pats.is_empty() {
                            g.declare_empty(kind.clone());
                        }
                        for p in pats {
                            g.allow(kind.clone(), &p);
                        }
                    }
                }
            }
        }
    }
    Ok(g)
}

/// Parse the `[capabilities]` table out of a whole `wj.toml` text. `Ok(None)` = the table is absent.
pub fn grant_from_manifest_text(text: &str, root: &Path) -> Result<Option<Grant>, String> {
    let doc: toml::Value = toml::from_str(text).map_err(|e| format!("wj.toml: {e}"))?;
    match doc.get("capabilities") {
        None => Ok(None),
        Some(toml::Value::Table(t)) => from_table(t, root).map(Some),
        Some(_) => Err("wj.toml: [capabilities] must be a table".to_string()),
    }
}

fn flatten<'a>(prefix: &str, t: &'a toml::value::Table, out: &mut Vec<(String, &'a toml::Value)>) {
    for (k, v) in t {
        let key = if prefix.is_empty() { k.clone() } else { format!("{prefix}.{k}") };
        if key == "dependencies" {
            continue; // per-dependency grants: see packages.rs
        }
        match v {
            toml::Value::Table(inner) => flatten(&key, inner, out),
            _ => out.push((key, v)),
        }
    }
}

fn strings(key: &str, v: &toml::Value) -> Result<Vec<String>, String> {
    match v {
        toml::Value::String(s) => Ok(vec![s.clone()]),
        toml::Value::Array(a) => a
            .iter()
            .map(|x| x.as_str().map(str::to_string).ok_or_else(|| format!("[capabilities] {key}: expected strings")))
            .collect(),
        _ => Err(format!("[capabilities] {key}: expected a list of strings")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn root() -> PathBuf {
        std::env::current_dir().unwrap()
    }

    fn g(manifest: &str) -> Grant {
        grant_from_manifest_text(&format!("[capabilities]\n{manifest}"), &root()).unwrap().unwrap()
    }

    #[test]
    fn empty_table_grants_only_determinism_effects() {
        let g = g("");
        assert!(g.check(&EffectKind::Time, None).is_ok());
        assert!(g.check(&EffectKind::Random, None).is_ok());
        assert!(g.check(&EffectKind::Console, None).is_ok());
        let d = g.check(&EffectKind::FsRead, Some("/etc/hosts")).unwrap_err();
        assert_eq!(d.subject(), "fs.read(/etc/hosts)");
        assert!(!d.outside_scope);
        assert!(d.fix.contains("fs.read"), "{}", d.fix);
    }

    #[test]
    fn dotted_keys_nest_and_quoted_keys_work() {
        let a = g("net.connect = [\"api.example.com\"]");
        let b = g("\"net.connect\" = [\"api.example.com\"]");
        for g in [&a, &b] {
            assert!(g.check(&EffectKind::NetConnect, Some("http://api.example.com/x")).is_ok());
            let d = g.check(&EffectKind::NetConnect, Some("http://other.example/")).unwrap_err();
            assert_eq!(d.subject(), "net.connect(other.example)");
            assert!(d.outside_scope);
        }
    }

    #[test]
    fn deny_wins_over_default_and_allow() {
        let g = g("deny = [\"time\", \"fs.read\"]\nfs.read = [\"./**\"]");
        assert!(g.check(&EffectKind::Time, None).is_err());
        assert!(g.check(&EffectKind::FsRead, Some("./x")).is_err());
    }

    #[test]
    fn unknown_keys_are_errors() {
        let r = grant_from_manifest_text("[capabilities]\nnet.conect = [\"x\"]\n", &root());
        assert!(r.unwrap_err().contains("unknown key `net.conect`"));
    }

    #[test]
    fn userinfo_trick_does_not_pass_the_host_check() {
        let g = g("net.connect = [\"api.example.com\"]");
        assert!(g.check(&EffectKind::NetConnect, Some("http://api.example.com@evil.example/")).is_err());
    }

    #[test]
    fn fs_scope_and_display_are_relative_to_the_root() {
        let g = g("fs.read = [\"./data/**\"]");
        assert!(g.check(&EffectKind::FsRead, Some("./data/a.json")).is_ok());
        let d = g.check(&EffectKind::FsRead, Some("./data/../secret.txt")).unwrap_err();
        assert_eq!(d.subject(), "fs.read(./secret.txt)");
        assert!(d.outside_scope);
    }

    #[test]
    fn user_effects_and_flags() {
        let mut g = g("effects = [\"ledger.post\"]");
        assert!(g.check(&EffectKind::User("ledger.post".into()), None).is_ok());
        assert!(g.check(&EffectKind::User("other".into()), None).is_err());
        g.allow_flag("net.connect=*.github.com").unwrap();
        assert!(g.check(&EffectKind::NetConnect, Some("https://api.github.com/")).is_ok());
        assert!(g.allow_flag("net.conect=x").is_err());
    }

    #[test]
    fn narrowing_never_widens() {
        let g = g("net.connect = [\"api.example.com\", \"*.github.com\"]\nfs.read = [\"./data/**\"]");
        let n = g.derive_scope(&EffectKind::NetConnect, Some(&["api.example.com".to_string()])).unwrap();
        assert!(n.check(&EffectKind::NetConnect, Some("http://api.example.com/")).is_ok());
        assert!(n.check(&EffectKind::NetConnect, Some("http://x.github.com/")).is_err(), "narrowed away");
        assert!(g.derive_scope(&EffectKind::NetConnect, Some(&["evil.example".to_string()])).is_err(), "not covered by the parent");
        assert!(g.derive_scope(&EffectKind::NetConnect, Some(&["*".to_string()])).is_err(), "cannot widen to everything");
        assert!(g.derive_scope(&EffectKind::NetConnect, Some(&["*.github.com".to_string()])).is_ok(), "an existing pattern is fine");
        assert!(g.derive_scope(&EffectKind::FsRead, Some(&["./data/sub/**".to_string()])).is_ok(), "a sub-tree");
        assert!(g.derive_scope(&EffectKind::FsRead, Some(&["./**".to_string()])).is_err(), "the parent of the grant");
        assert!(g.derive_scope(&EffectKind::Process, None).is_err(), "an effect that is not granted cannot be minted");
    }

    #[test]
    fn process_grants_match_names_not_local_paths() {
        let g = g("process = [\"echo\", \"/usr/bin/git\"]");
        assert!(g.check(&EffectKind::Process, Some("echo")).is_ok());
        assert!(g.check(&EffectKind::Process, Some("./echo")).is_err());
        assert!(g.check(&EffectKind::Process, Some("/tmp/echo")).is_err());
        assert!(g.check(&EffectKind::Process, Some("/usr/bin/git")).is_ok());
        assert!(g.check(&EffectKind::Process, Some("git")).is_err());
        assert!(g.check(&EffectKind::Process, Some("sh")).is_err());
    }

    #[test]
    fn manifest_and_sources_are_not_writable_through_globs() {
        let g = Grant::local_profile(&root());
        assert!(g.check(&EffectKind::FsWrite, Some("out.txt")).is_ok());
        assert!(g.check(&EffectKind::FsWrite, Some("wj.toml")).is_err());
        assert!(g.check(&EffectKind::FsWrite, Some("src/main.wj")).is_err());
        assert!(g.check(&EffectKind::FsRead, Some("wj.toml")).is_ok(), "reading is fine");
        let mut named = Grant::sandbox(&root());
        named.allow(EffectKind::FsWrite, "./src/gen.wj");
        assert!(named.check(&EffectKind::FsWrite, Some("src/gen.wj")).is_ok(), "named exactly");
    }

    #[test]
    fn local_profile_is_the_project_directory() {
        let g = Grant::local_profile(&root());
        assert!(g.check(&EffectKind::FsWrite, Some("out/x.txt")).is_ok());
        assert!(g.check(&EffectKind::FsRead, Some("/etc/hosts")).is_err());
        assert!(g.check(&EffectKind::NetConnect, Some("http://example.com/")).is_err());
        assert!(g.check(&EffectKind::EnvRead, Some("HOME")).is_err());
        assert!(g.check(&EffectKind::Process, Some("ls")).is_err());
    }
}
