//! Minimal line-based edits between two texts (used by editor integrations).

/// Replace source lines `start_line..end_line` (0-based, end exclusive) with `new_text`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineEdit {
    pub start_line: usize,
    pub end_line: usize,
    pub new_text: String,
}

const LCS_CELL_LIMIT: usize = 4_000_000;

/// Minimal set of hunks turning `old` into `new`.
pub fn diff_lines(old: &str, new: &str) -> Vec<LineEdit> {
    let a: Vec<&str> = old.split_inclusive('\n').collect();
    let b: Vec<&str> = new.split_inclusive('\n').collect();
    let pre = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    let suf = a[pre..]
        .iter()
        .rev()
        .zip(b[pre..].iter().rev())
        .take_while(|(x, y)| x == y)
        .count();
    let (ma, mb) = (&a[pre..a.len() - suf], &b[pre..b.len() - suf]);
    if ma.is_empty() && mb.is_empty() {
        return Vec::new();
    }
    if ma.len() * mb.len() > LCS_CELL_LIMIT {
        return vec![hunk(pre, pre + ma.len(), mb)];
    }
    lcs_hunks(ma, mb, pre)
}

fn hunk(start: usize, end: usize, new_lines: &[&str]) -> LineEdit {
    LineEdit {
        start_line: start,
        end_line: end,
        new_text: new_lines.concat(),
    }
}

fn lcs_hunks(a: &[&str], b: &[&str], base: usize) -> Vec<LineEdit> {
    let (n, m) = (a.len(), b.len());
    let mut t = vec![vec![0u32; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            t[i][j] = if a[i] == b[j] {
                t[i + 1][j + 1] + 1
            } else {
                t[i + 1][j].max(t[i][j + 1])
            };
        }
    }
    let (mut i, mut j) = (0, 0);
    let mut out = Vec::new();
    while i < n || j < m {
        if i < n && j < m && a[i] == b[j] {
            i += 1;
            j += 1;
            continue;
        }
        let (si, sj) = (i, j);
        while (i < n || j < m) && !(i < n && j < m && a[i] == b[j]) {
            if j >= m || (i < n && t[i + 1][j] >= t[i][j + 1]) {
                i += 1;
            } else {
                j += 1;
            }
        }
        out.push(hunk(base + si, base + i, &b[sj..j]));
    }
    out
}

/// Keep only hunks touching the 0-based inclusive line range.
pub fn restrict_to_range(edits: Vec<LineEdit>, first: usize, last: usize) -> Vec<LineEdit> {
    edits
        .into_iter()
        .filter(|e| {
            let end = e.end_line.max(e.start_line + 1);
            e.start_line <= last && end > first
        })
        .collect()
}
