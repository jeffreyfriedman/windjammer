//! `repro.md`: what the oracle was, how to reproduce, and how much smaller the project got.
//! Contains no input source text; the reduced project next to it is the only source output.

use super::project::Project;

pub struct Summary<'a> {
    pub oracle: String,
    pub before: &'a Project,
    pub after: &'a Project,
    pub steps: usize,
    pub cache_hits: usize,
    pub rejected: usize,
    pub seed: u64,
    pub anonymized: bool,
    pub warnings: &'a [String],
}

pub fn repro_md(s: &Summary) -> String {
    let (bf, af) = (s.before.files.len(), s.after.files.len());
    let (bl, al) = (s.before.line_count(), s.after.line_count());
    let pct = if bl == 0 {
        0.0
    } else {
        100.0 * (1.0 - al as f64 / bl as f64)
    };
    let mut out = String::new();
    out.push_str("# Reduced reproduction\n\n");
    out.push_str(&format!("Oracle: `{}`\n\n", s.oracle));
    out.push_str("## Reproduce\n\n");
    out.push_str(&format!(
        "```bash\n{}\n```\n\n",
        reproduce_command(&s.oracle)
    ));
    out.push_str("## Size\n\n| | before | after |\n|---|---|---|\n");
    out.push_str(&format!(
        "| files | {bf} | {af} |\n| lines (non-blank) | {bl} | {al} |\n| bytes | {} | {} |\n\n",
        s.before.byte_count(),
        s.after.byte_count()
    ));
    out.push_str(&format!("Reduction: {pct:.1}% of lines removed.\n\n"));
    out.push_str(&format!(
        "Search: {} oracle runs, {} cache hits, {} candidates rejected by `wj check`, seed {}.\n\n",
        s.steps, s.cache_hits, s.rejected, s.seed
    ));
    if s.anonymized {
        out.push_str("Identifiers, strings, comments and file paths are anonymized (`T1`, `f1`, `field1`, `m1`, `v1`, `s1`); the oracle was re-checked on this exact output.\n");
    }
    for w in s.warnings {
        out.push_str(&format!("\nWarning: {w}\n"));
    }
    out
}

fn reproduce_command(oracle: &str) -> String {
    match oracle.split_once(' ') {
        Some(("cmd", c)) => format!("cd <this directory> && sh -c {c}"),
        _ if oracle.starts_with("rustc-error") => {
            "wj build . --no-cargo -o out && (cd out && cargo check)".to_string()
        }
        _ if oracle == "panic" => {
            "wj build . --no-cargo -o out   # the compiler panics".to_string()
        }
        _ if oracle == "miscompile" => {
            "wj run . --interpret > a.txt; wj run . --compiled > b.txt; diff a.txt b.txt"
                .to_string()
        }
        _ => "see the oracle above".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_has_sizes_oracle_and_no_source() {
        let mut b = Project::default();
        b.files
            .insert("a.wj".into(), "fn a() {\n}\nfn b() {\n}\n".into());
        b.files.insert("b.wj".into(), "fn c() {\n}\n".into());
        let mut a = Project::default();
        a.files.insert("a.wj".into(), "fn a() {\n}\n".into());
        let md = repro_md(&Summary {
            oracle: "cmd 'true'".into(),
            before: &b,
            after: &a,
            steps: 3,
            cache_hits: 1,
            rejected: 2,
            seed: 0,
            anonymized: true,
            warnings: &[],
        });
        assert!(md.contains("| files | 2 | 1 |"));
        assert!(md.contains("cmd 'true'"));
        assert!(!md.contains("fn a"));
    }
}
