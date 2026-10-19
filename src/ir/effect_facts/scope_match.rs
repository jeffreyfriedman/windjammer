//! Resource matching for grants: file paths, network hosts, environment names, program names.
//!
//! Pure functions plus filesystem canonicalization (symlinks, `..`). Used by [`super::grant::Grant`].

use std::path::{Component, Path, PathBuf};

/// Lexically normalize `p` against `base` (absolute result; `.` and `..` resolved without touching disk).
pub fn lexical_abs(p: &str, base: &Path) -> PathBuf {
    let raw = Path::new(p);
    let joined = if raw.is_absolute() { raw.to_path_buf() } else { base.join(raw) };
    let mut out = PathBuf::new();
    for c in joined.components() {
        match c {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// Resolve symlinks of the deepest existing ancestor and append the rest (the target may not exist yet).
pub fn canonical_abs(p: &str, base: &Path) -> PathBuf {
    let lex = lexical_abs(p, base);
    let mut existing = lex.clone();
    let mut rest: Vec<std::ffi::OsString> = Vec::new();
    loop {
        if let Ok(c) = std::fs::canonicalize(&existing) {
            let mut out = c;
            for r in rest.iter().rev() {
                out.push(r);
            }
            return out;
        }
        match (existing.file_name().map(|n| n.to_os_string()), existing.parent().map(|p| p.to_path_buf())) {
            (Some(n), Some(parent)) => {
                rest.push(n);
                existing = parent;
            }
            _ => return lex,
        }
    }
}

fn components(p: &Path) -> Vec<String> {
    p.components()
        .filter_map(|c| match c {
            Component::Normal(s) => Some(fold_case(&s.to_string_lossy())),
            _ => None,
        })
        .collect()
}

fn fold_case(s: &str) -> String {
    if cfg!(any(target_os = "macos", target_os = "windows")) {
        s.to_lowercase()
    } else {
        s.to_string()
    }
}

/// Wildcard match of one path component: `*` matches any run of characters.
fn component_glob(pat: &str, s: &str) -> bool {
    let (p, t): (Vec<char>, Vec<char>) = (pat.chars().collect(), s.chars().collect());
    let (mut pi, mut ti, mut star, mut mark) = (0usize, 0usize, None::<usize>, 0usize);
    while ti < t.len() {
        if pi < p.len() && (p[pi] == t[ti] && p[pi] != '*') {
            pi += 1;
            ti += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some(pi);
            mark = ti;
            pi += 1;
        } else if let Some(sp) = star {
            pi = sp + 1;
            mark += 1;
            ti = mark;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == '*' {
        pi += 1;
    }
    pi == p.len()
}

/// Does the absolute, canonical `path` fall inside the path pattern `pat` (relative to `base`)?
/// `dir/**` = `dir` and everything below it; `*` inside a component matches within that component;
/// anything else must match exactly. The pattern's non-glob prefix is canonicalized like the path.
pub fn path_matches(pat: &str, path: &Path, base: &Path) -> bool {
    let segs: Vec<&str> = pat.split('/').filter(|s| !s.is_empty() && *s != ".").collect();
    let first_glob = segs.iter().position(|s| s.contains('*')).unwrap_or(segs.len());
    let abs_prefix = if pat.starts_with('/') { "/".to_string() } else { String::new() };
    let prefix_str = format!("{}{}", abs_prefix, segs[..first_glob].join("/"));
    let prefix = canonical_abs(if prefix_str.is_empty() { "." } else { &prefix_str }, base);
    let pc = components(&prefix);
    let tail = &segs[first_glob..];
    let target = components(path);
    if target.len() < pc.len() || target[..pc.len()] != pc[..] {
        return false;
    }
    let rest = &target[pc.len()..];
    if tail.is_empty() {
        return rest.is_empty();
    }
    let mut i = 0;
    for (k, seg) in tail.iter().enumerate() {
        if *seg == "**" {
            // `**` swallows the rest only when it is last; otherwise it matches any run of components
            if k == tail.len() - 1 {
                return true;
            }
            return (i..=rest.len()).any(|j| tail_matches(&tail[k + 1..], &rest[j..]));
        }
        if i >= rest.len() || !component_glob(&fold_case(seg), &rest[i]) {
            return false;
        }
        i += 1;
    }
    i == rest.len()
}

fn tail_matches(tail: &[&str], rest: &[String]) -> bool {
    if tail.is_empty() {
        return rest.is_empty();
    }
    if tail[0] == "**" {
        return (0..=rest.len()).any(|j| tail_matches(&tail[1..], &rest[j..]));
    }
    !rest.is_empty() && component_glob(&fold_case(tail[0]), &rest[0]) && tail_matches(&tail[1..], &rest[1..])
}

/// Split a URL or `host:port` address into (host, port, path-and-query). The host is lower-cased; userinfo is
/// dropped (the host is what follows the LAST `@` of the authority, so `http://api.example.com@evil.example/` is
/// `evil.example`). This is the ONE URL parser: the capability check and the interpreter's http client both use it,
/// so what is checked is what is connected to.
pub fn split_url(url: &str) -> Option<(String, Option<u16>, String)> {
    let (scheme, rest) = match url.split_once("://") {
        Some((s, r)) => (Some(s.to_ascii_lowercase()), r),
        None => (None, url),
    };
    let end = rest.find(['/', '?', '#', '\\']).unwrap_or(rest.len());
    let authority = &rest[..end];
    let tail = &rest[end..];
    let path = if tail.starts_with('/') { tail.to_string() } else { format!("/{}", tail.trim_start_matches('\\')) };
    let hostport = authority.rsplit_once('@').map(|(_, h)| h).unwrap_or(authority);
    let (host, port) = if let Some(stripped) = hostport.strip_prefix('[') {
        let (h, after) = stripped.split_once(']')?;
        (h.to_string(), after.strip_prefix(':').and_then(|p| p.parse().ok()))
    } else {
        match hostport.rsplit_once(':') {
            Some((h, p)) if !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()) => (h.to_string(), p.parse().ok()),
            _ => (hostport.to_string(), None),
        }
    };
    if host.is_empty() {
        return None;
    }
    let port = port.or(match scheme.as_deref() {
        Some("http") => Some(80),
        Some("https") => Some(443),
        _ => None,
    });
    Some((host.trim_end_matches('.').to_ascii_lowercase(), port, path))
}

/// Host and port of a URL or `host:port` address (see [`split_url`]).
pub fn host_port(url: &str) -> Option<(String, Option<u16>)> {
    split_url(url).map(|(h, p, _)| (h, p))
}

/// Host pattern: `host`, `host:port`, `*.suffix[:port]`, `*`. No port in the pattern = any port.
pub fn host_matches(pat: &str, host: &str, port: Option<u16>) -> bool {
    if pat == "*" {
        return true;
    }
    let (ph, pp) = match pat.strip_prefix('[') {
        Some(s) => match s.split_once(']') {
            Some((h, after)) => (h.to_string(), after.strip_prefix(':').and_then(|p| p.parse::<u16>().ok())),
            None => (pat.to_string(), None),
        },
        None => match pat.rsplit_once(':') {
            Some((h, p)) if p.chars().all(|c| c.is_ascii_digit()) && !p.is_empty() => (h.to_string(), p.parse::<u16>().ok()),
            _ => (pat.to_string(), None),
        },
    };
    if let Some(pp) = pp {
        if port != Some(pp) {
            return false;
        }
    }
    let ph = ph.to_ascii_lowercase();
    match ph.strip_prefix("*.") {
        Some(suffix) => host.len() > suffix.len() + 1 && host.ends_with(suffix) && host.as_bytes()[host.len() - suffix.len() - 1] == b'.',
        None => ph == host,
    }
}

/// Name pattern: exact, `PREFIX*`, or `*`.
pub fn name_matches(pat: &str, name: &str) -> bool {
    match pat.strip_suffix('*') {
        Some(prefix) => name.starts_with(prefix),
        None => pat == name,
    }
}

/// How `process::run("...")` executes a command line: (program, args). A plain command (`ls -l /tmp`) runs
/// directly; anything with shell syntax (`;`, `|`, `&`, `$`, quotes, redirects, globs, ...) runs through `sh -c`,
/// so its program is `sh`. The capability check and the execution use this one function: a grant for `echo`
/// must not cover `echo hi; rm -rf x`.
pub fn parse_command(cmd: &str) -> (String, Vec<String>) {
    const SHELL_META: &[char] = &[';', '&', '|', '<', '>', '(', ')', '$', '`', '\\', '"', '\'', '*', '?', '[', ']', '{', '}', '~', '!', '#', '\n', '\r'];
    if !cmd.contains(char::is_whitespace) {
        return (cmd.to_string(), Vec::new());
    }
    if cmd.contains(SHELL_META) {
        return ("sh".to_string(), vec!["-c".to_string(), cmd.to_string()]);
    }
    let mut words = cmd.split_whitespace().map(str::to_string);
    let prog = words.next().unwrap_or_default();
    (prog, words.collect())
}

/// Program name for a grant: the file name of the program path (`/usr/bin/git` -> `git`).
pub fn program_name(cmd: &str) -> String {
    let first = cmd.split_whitespace().next().unwrap_or("");
    Path::new(first).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| first.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> PathBuf {
        std::env::temp_dir().join("wj_scope_match_test_base")
    }

    #[test]
    fn dotdot_leaves_the_scope() {
        let b = base();
        let p = canonical_abs("./data/../secret.txt", &b);
        assert!(!path_matches("./data/**", &p, &b));
        assert!(path_matches("./data/**", &canonical_abs("./data/sub/x.json", &b), &b));
        assert!(path_matches("./data/**", &canonical_abs("./data", &b), &b));
    }

    #[test]
    fn exact_and_single_level_patterns() {
        let b = base();
        assert!(path_matches("./config.toml", &canonical_abs("config.toml", &b), &b));
        assert!(!path_matches("./config.toml", &canonical_abs("config.toml.bak", &b), &b));
        assert!(path_matches("./data/*.json", &canonical_abs("data/a.json", &b), &b));
        assert!(!path_matches("./data/*.json", &canonical_abs("data/sub/a.json", &b), &b));
        assert!(!path_matches("./data/*.json", &canonical_abs("data/a.txt", &b), &b));
        assert!(path_matches("./a/**/c.txt", &canonical_abs("a/x/y/c.txt", &b), &b));
        assert!(path_matches("./a/**/c.txt", &canonical_abs("a/c.txt", &b), &b));
    }

    #[test]
    fn sibling_prefix_is_not_inside() {
        let b = base();
        assert!(!path_matches("./data/**", &canonical_abs("data2/x", &b), &b));
    }

    #[test]
    fn symlink_out_of_scope_is_followed() {
        let b = std::env::temp_dir().join(format!("wj_scope_symlink_{}", std::process::id()));
        std::fs::create_dir_all(b.join("data")).unwrap();
        std::fs::create_dir_all(b.join("outside")).unwrap();
        std::fs::write(b.join("outside/secret.txt"), "s").unwrap();
        #[cfg(unix)]
        {
            let _ = std::os::unix::fs::symlink(b.join("outside"), b.join("data/link"));
            let p = canonical_abs("data/link/secret.txt", &b);
            assert!(!path_matches("./data/**", &p, &b), "symlink target {p:?} must be outside ./data");
        }
        std::fs::remove_dir_all(&b).unwrap();
    }

    #[test]
    fn hosts_ports_and_userinfo() {
        assert_eq!(host_port("http://API.example.com/x"), Some(("api.example.com".into(), Some(80))));
        assert_eq!(host_port("https://api.example.com:8443/x"), Some(("api.example.com".into(), Some(8443))));
        assert_eq!(host_port("http://api.example.com@evil.example/"), Some(("evil.example".into(), Some(80))));
        assert_eq!(host_port("http://user:pw@evil.example:81/"), Some(("evil.example".into(), Some(81))));
        assert_eq!(host_port("http://[::1]:8080/"), Some(("::1".into(), Some(8080))));
        assert_eq!(host_port("127.0.0.1:8080"), Some(("127.0.0.1".into(), Some(8080))));
        assert_eq!(host_port("http://api.example.com./"), Some(("api.example.com".into(), Some(80))));
        assert_eq!(host_port("http://"), None);
    }

    #[test]
    fn split_url_keeps_checked_host_and_connect_host_equal() {
        // backslash ends the authority (browser rule): the host is what precedes it
        assert_eq!(split_url("http://api.example.com\\@evil.example/x").unwrap().0, "api.example.com");
        assert_eq!(split_url("http://evil.example#@api.example.com/").unwrap().0, "evil.example");
        assert_eq!(split_url("http://evil.example?h=api.example.com").unwrap().0, "evil.example");
        assert_eq!(split_url("http://a.example:8080/p?q=1").unwrap(), ("a.example".into(), Some(8080), "/p?q=1".into()));
        assert_eq!(split_url("http://a.example").unwrap().2, "/");
    }

    #[test]
    fn host_patterns() {
        assert!(host_matches("api.example.com", "api.example.com", Some(443)));
        assert!(!host_matches("api.example.com", "evil.api.example.com", Some(443)));
        assert!(host_matches("*.example.com", "a.example.com", None));
        assert!(!host_matches("*.example.com", "example.com", None));
        assert!(!host_matches("*.example.com", "badexample.com", None));
        assert!(host_matches("api.example.com:443", "api.example.com", Some(443)));
        assert!(!host_matches("api.example.com:443", "api.example.com", Some(80)));
        assert!(host_matches("*", "anything", None));
        assert!(host_matches("127.0.0.1", "127.0.0.1", Some(1)));
        assert!(!host_matches("127.0.0.1", "127.0.0.2", Some(1)));
    }

    #[test]
    fn shell_syntax_means_sh() {
        assert_eq!(parse_command("ls"), ("ls".to_string(), vec![]));
        assert_eq!(parse_command("ls -l /tmp"), ("ls".to_string(), vec!["-l".to_string(), "/tmp".to_string()]));
        assert_eq!(parse_command("echo hi; id").0, "sh");
        assert_eq!(parse_command("echo $(id)").0, "sh");
        assert_eq!(parse_command("echo hi | cat").0, "sh");
        assert_eq!(parse_command("echo 'a b'").0, "sh");
        assert_eq!(parse_command("echo a\nid").0, "sh");
    }

    #[test]
    fn names_and_programs() {
        assert!(name_matches("AWS_*", "AWS_KEY"));
        assert!(!name_matches("AWS_*", "HOME"));
        assert_eq!(program_name("/usr/bin/git status"), "git");
        assert_eq!(program_name("ls"), "ls");
    }
}
