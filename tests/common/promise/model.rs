//! Promise registry model: what `wj` promises (derived from the clap tree, the generated error
//! reference and `docs/promises.toml`) and the acceptance cases in `tests/promises/*.toml`.

use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

#[derive(Deserialize, Debug, Default)]
pub struct Registry {
    /// Ratchet: number of promises allowed to be `pending` (no acceptance tests yet). May only shrink.
    pub max_pending: usize,
    #[serde(default, rename = "promise")]
    pub promises: Vec<Promise>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Promise {
    /// `wj build --no-cargo`, `code WJ0003`, `feature json-diagnostics`.
    pub id: String,
    /// What the user is told (CLI help text, doc sentence).
    #[serde(default)]
    pub statement: String,
    /// For features: the file that makes the promise and a verbatim quote that must still be there.
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub quote: Option<String>,
    /// No acceptance tests yet: the reason. Counted against `max_pending`.
    #[serde(default)]
    pub pending: Option<String>,
}

#[derive(Deserialize, Debug, Default)]
pub struct CaseFile {
    #[serde(default, rename = "case")]
    pub cases: Vec<Case>,
}

#[derive(Deserialize, Debug, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct Case {
    pub name: String,
    /// `positive` (the promise holds) or `negative` (bad input is rejected / diagnosed).
    pub polarity: String,
    /// `feature ...` ids this case exercises. CLI and code ids are derived, never declared.
    #[serde(default)]
    pub covers: Vec<String>,
    /// Files created in the scratch dir `{w}` before running (relative path -> contents).
    #[serde(default)]
    pub files: BTreeMap<String, String>,
    /// Earlier `wj` invocations (same sandbox) that prepare the scratch dir; must exit 0.
    #[serde(default)]
    pub setup: Vec<Vec<String>>,
    pub args: Vec<String>,
    #[serde(default)]
    pub stdin: Option<String>,
    /// Working directory relative to `{w}` (default `{w}`).
    #[serde(default)]
    pub cwd: Option<String>,
    /// Expected exit code. Default: 0 for positive; any non-zero for negative.
    #[serde(default)]
    pub exit: Option<i32>,
    #[serde(default)]
    pub stdout_has: Vec<String>,
    #[serde(default)]
    pub stderr_has: Vec<String>,
    /// stdout or stderr.
    #[serde(default)]
    pub output_has: Vec<String>,
    #[serde(default)]
    pub output_lacks: Vec<String>,
    /// Paths (relative to `{w}`) that must exist afterwards / must not exist.
    #[serde(default)]
    pub creates: Vec<String>,
    #[serde(default)]
    pub absent: Vec<String>,
    /// `path = "substring"`: the file must contain the substring afterwards.
    #[serde(default)]
    pub file_has: BTreeMap<String, String>,
    /// stdout must be valid JSON.
    #[serde(default)]
    pub json: bool,
    /// Host tools on PATH: `toolchain` (node/rustc/go; runs by default) or `cargo` (a full cargo
    /// build of the runtime, minutes: runs only with WJ_PROMISES_CARGO=1).
    #[serde(default)]
    pub needs: Option<String>,
    /// Known-broken promise: the case documents the promised behavior and is expected to FAIL
    /// (the bug). It does not count as coverage; when it starts passing the gate asks to drop `xfail`.
    #[serde(default)]
    pub xfail: Option<String>,
}

impl Case {
    pub fn is_negative(&self) -> bool {
        self.polarity == "negative"
    }
    /// Every text the case expects to see, used to derive which error codes it proves.
    pub fn expected_texts(&self) -> impl Iterator<Item = &String> {
        self.stdout_has.iter().chain(&self.stderr_has).chain(&self.output_has)
    }
}

pub fn load_registry(root: &Path) -> Registry {
    let text = std::fs::read_to_string(root.join("docs/promises.toml")).expect("docs/promises.toml");
    toml::from_str(&text).unwrap_or_else(|e| panic!("docs/promises.toml: {e}"))
}

/// `(file name, case)` for every `tests/promises/*.toml`.
pub fn load_cases(root: &Path) -> Vec<(String, Case)> {
    let mut out = Vec::new();
    let mut files: Vec<_> = std::fs::read_dir(root.join("tests/promises")).unwrap().flatten().map(|e| e.path()).collect();
    files.sort();
    for f in files.into_iter().filter(|p| p.extension().is_some_and(|x| x == "toml")) {
        let text = std::fs::read_to_string(&f).unwrap();
        let cf: CaseFile = toml::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", f.display()));
        let name = f.file_name().unwrap().to_string_lossy().to_string();
        out.extend(cf.cases.into_iter().map(|c| (name.clone(), c)));
    }
    out
}

// ------------------------------------------------------------------------------------------
// Derived promises
// ------------------------------------------------------------------------------------------

fn walk(c: &clap::Command, path: &mut Vec<String>, out: &mut BTreeMap<String, String>) {
    let text = |s: Option<&clap::builder::StyledStr>| s.map(|x| x.to_string().lines().next().unwrap_or("").to_string()).unwrap_or_default();
    for s in c.get_subcommands() {
        if s.is_hide_set() || s.get_name() == "help" {
            continue;
        }
        path.push(s.get_name().to_string());
        let base = format!("wj {}", path.join(" "));
        out.insert(base.clone(), text(s.get_about()));
        for a in s.get_arguments() {
            if let Some(l) = a.get_long().filter(|l| *l != "help" && *l != "version") {
                out.insert(format!("{base} --{l}"), text(a.get_help()));
            }
        }
        walk(s, path, out);
        path.pop();
    }
}

/// `wj <sub>` and `wj <sub> --<flag>` for every visible subcommand and long flag, with its help line.
pub fn cli_promises(cli: &clap::Command) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    walk(cli, &mut Vec::new(), &mut out);
    out
}

/// `code XXXX` for every code in a table of the generated `docs/reference/errors.md`.
pub fn code_promises(root: &Path) -> BTreeSet<String> {
    let text = std::fs::read_to_string(root.join("docs/reference/errors.md")).unwrap();
    text.lines()
        .filter_map(|l| l.strip_prefix("| `"))
        .filter_map(|l| l.split('`').next())
        .filter(|c| c.len() >= 5 && c.chars().all(|ch| ch.is_ascii_alphanumeric()))
        .map(|c| format!("code {c}"))
        .collect()
}

/// CLI promise ids a concrete invocation exercises, read back from clap's own parse (flags that
/// were actually given on the command line). Falls back to token matching when clap rejects the
/// arguments (negative cases with invalid values).
pub fn covered_cli(cli: &clap::Command, args: &[String]) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let argv: Vec<String> = std::iter::once("wj".to_string()).chain(args.iter().cloned()).collect();
    if let Ok(m) = cli.clone().try_get_matches_from(&argv) {
        let (mut cmd, mut m, mut path) = (cli.clone(), m, Vec::<String>::new());
        while let Some((name, sub_m)) = m.subcommand().map(|(n, s)| (n.to_string(), s.clone())) {
            let Some(sub) = cmd.find_subcommand(&name).cloned() else { break };
            path.push(name);
            let base = format!("wj {}", path.join(" "));
            out.insert(base.clone());
            for a in sub.get_arguments() {
                let given = sub_m.value_source(a.get_id().as_str()) == Some(clap::parser::ValueSource::CommandLine);
                if let (true, Some(l)) = (given, a.get_long()) {
                    out.insert(format!("{base} --{l}"));
                }
            }
            cmd = sub;
            m = sub_m;
        }
        return out;
    }
    // clap rejected the invocation: walk subcommand names, then long/short flags of the leaf.
    let mut cmd = cli.clone();
    let mut path = Vec::new();
    for a in args {
        if let Some(sub) = cmd.find_subcommand(a).cloned() {
            path.push(a.clone());
            out.insert(format!("wj {}", path.join(" ")));
            cmd = sub;
            continue;
        }
        let base = format!("wj {}", path.join(" "));
        if let Some(l) = a.strip_prefix("--") {
            let l = l.split('=').next().unwrap();
            if cmd.get_arguments().any(|x| x.get_long() == Some(l)) {
                out.insert(format!("{base} --{l}"));
            }
        } else if let Some(s) = a.strip_prefix('-').and_then(|s| s.chars().next()) {
            if let Some(l) = cmd.get_arguments().find(|x| x.get_short() == Some(s)).and_then(|x| x.get_long()) {
                out.insert(format!("{base} --{l}"));
            }
        }
    }
    out
}

/// `code XXXX` ids a case proves: codes that appear in its expected output.
pub fn covered_codes(case: &Case, codes: &BTreeSet<String>) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for t in case.expected_texts() {
        for c in codes {
            let code = c.trim_start_matches("code ");
            if t.contains(code) {
                out.insert(c.clone());
            }
        }
    }
    out
}
