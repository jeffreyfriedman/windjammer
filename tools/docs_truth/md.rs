//! Minimal Markdown scanning: fences, headings, inline code, links. Line-based, no dependencies.

pub use windjammer::doctest::fence::{fences, Fence};

/// Lines of `text` outside fences (1-based line numbers kept).
pub fn prose_lines(text: &str) -> Vec<(usize, &str)> {
    let mut out = Vec::new();
    let mut fence: Option<(char, usize)> = None;
    for (i, raw) in text.lines().enumerate() {
        let t = raw.trim_start();
        let ch = t.chars().next().unwrap_or(' ');
        let run = if ch == '`' || ch == '~' { t.chars().take_while(|&c| c == ch).count() } else { 0 };
        match fence {
            None if run >= 3 && !(ch == '`' && t[run..].contains('`')) => fence = Some((ch, run)),
            Some((c, n)) if ch == c && run >= n && t[run..].trim().is_empty() => fence = None,
            Some(_) => {}
            None => out.push((i + 1, raw)),
        }
    }
    out
}

/// Inline code spans (single-backtick) of one prose line.
pub fn code_spans(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let b: Vec<char> = line.chars().collect();
    let mut i = 0;
    while i < b.len() {
        if b[i] == '`' {
            let n = b[i..].iter().take_while(|&&c| c == '`').count();
            let start = i + n;
            let mut j = start;
            let mut found = None;
            while j < b.len() {
                if b[j] == '`' {
                    let m = b[j..].iter().take_while(|&&c| c == '`').count();
                    if m == n {
                        found = Some(j);
                        break;
                    }
                    j += m;
                } else {
                    j += 1;
                }
            }
            match found {
                Some(e) => {
                    out.push(b[start..e].iter().collect::<String>().trim().to_string());
                    i = e + n;
                }
                None => i += n,
            }
        } else {
            i += 1;
        }
    }
    out
}

/// Replace inline code spans with spaces (so prose scans ignore them).
pub fn strip_code_spans(line: &str) -> String {
    let mut out = String::new();
    let mut in_code = false;
    for c in line.chars() {
        if c == '`' {
            in_code = !in_code;
            out.push(' ');
        } else if in_code {
            out.push(' ');
        } else {
            out.push(c);
        }
    }
    out
}

/// `[text](target)` link targets of a prose line.
pub fn link_targets(line: &str) -> Vec<String> {
    let line = strip_code_spans(line);
    let mut out = Vec::new();
    let mut rest = line.as_str();
    while let Some(p) = rest.find("](") {
        let after = &rest[p + 2..];
        if let Some(e) = after.find(')') {
            let t = after[..e].trim();
            let t = t.split_whitespace().next().unwrap_or("");
            if !t.is_empty() {
                out.push(t.trim_matches(|c| c == '<' || c == '>').to_string());
            }
            rest = &after[e..];
        } else {
            break;
        }
    }
    out
}

/// GitHub-style heading slug.
pub fn slug(heading: &str) -> String {
    let mut s = String::new();
    for c in heading.trim().to_lowercase().chars() {
        if c.is_alphanumeric() || c == '-' || c == '_' {
            s.push(c);
        } else if c == ' ' {
            s.push('-');
        }
    }
    s
}

/// Anchors (slugs) defined by the headings of a document, with `-1` suffixes for duplicates.
pub fn anchors(text: &str) -> Vec<String> {
    let mut seen: std::collections::BTreeMap<String, usize> = Default::default();
    let mut out = Vec::new();
    for (_, l) in prose_lines(text) {
        let t = l.trim_start();
        if t.starts_with('#') {
            let h = t.trim_start_matches('#').trim().replace('`', "");
            let base = slug(&h);
            let n = seen.entry(base.clone()).or_insert(0);
            out.push(if *n == 0 { base.clone() } else { format!("{base}-{n}") });
            *n += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fences_and_spans() {
        let t = "a\n```wj,ignore\nlet x = 1\n```\nb `c` d\n";
        let f = fences(t);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].info, "wj,ignore");
        assert_eq!(f[0].line, 2);
        assert_eq!(f[0].body, "let x = 1\n");
        assert_eq!(prose_lines(t).len(), 2);
        assert_eq!(code_spans("x `a` y ``b`c``"), vec!["a", "b`c"]);
        assert_eq!(link_targets("see [x](a/b.md#c) and `[no](q)`"), vec!["a/b.md#c"]);
        assert_eq!(anchors("# Hello World!\n## A\n## A\n"), vec!["hello-world", "a", "a-1"]);
    }
}
