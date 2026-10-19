//! Generic ddmin over `n` removable items. `build(removed)` makes the candidate project that has
//! exactly the items in `removed` taken out; the surviving item indices are returned and the
//! winning candidate is adopted in the session.

use super::project::Project;
use super::session::Session;

pub fn ddmin(
    s: &mut Session,
    n_items: usize,
    build: &dyn Fn(&Project, &[usize]) -> Project,
) -> Vec<usize> {
    let mut keep: Vec<usize> = (0..n_items).collect();
    let removed_of = |keep: &[usize], extra: &[usize]| -> Vec<usize> {
        (0..n_items)
            .filter(|i| !keep.contains(i) || extra.contains(i))
            .collect()
    };
    if keep.len() > 1 {
        let c = build(&s.project, &removed_of(&keep, &keep));
        if s.eng.first_success(std::slice::from_ref(&c)).is_some() {
            s.project = c;
            keep.clear();
        }
    }
    let mut n = 2usize;
    while !keep.is_empty() && !s.eng.exhausted() {
        let parts = n.min(keep.len());
        let chunks = split(&keep, parts);
        let cands: Vec<Project> = chunks
            .iter()
            .map(|c| build(&s.project, &removed_of(&keep, c)))
            .collect();
        match s.eng.first_success(&cands) {
            Some(i) => {
                s.project = cands.into_iter().nth(i).expect("index from first_success");
                keep.retain(|k| !chunks[i].contains(k));
                n = n.saturating_sub(1).max(2);
            }
            None if parts >= keep.len() => break,
            None => n = (n * 2).min(keep.len()),
        }
    }
    keep
}

pub fn split(items: &[usize], parts: usize) -> Vec<Vec<usize>> {
    let (q, r) = (items.len() / parts, items.len() % parts);
    let mut out = Vec::with_capacity(parts);
    let mut at = 0;
    for i in 0..parts {
        let len = q + usize::from(i < r);
        out.push(items[at..at + len].to_vec());
        at += len;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::split;

    #[test]
    fn split_is_balanced_and_complete() {
        let v: Vec<usize> = (0..7).collect();
        let parts = split(&v, 3);
        assert_eq!(parts.iter().map(Vec::len).collect::<Vec<_>>(), [3, 2, 2]);
        assert_eq!(parts.concat(), v);
    }
}
