//! Replacement passes: each *site* (a token range) has alternative smaller replacements; sites are
//! tried back to front so accepted edits never shift the sites still to be tried.
//!
//! Sites: call arguments and `let`/assignment right-hand sides (-> literals), generic type
//! applications (-> first type argument), attributes (-> removed), simple function calls (-> body).

use super::session::Session;
use super::toks::{groups, render, replace_range, scan, string_lit, units, word, Group, GroupKind};
use crate::formatter::{Tok, TokKind};
use std::ops::Range;

pub struct Site {
    pub range: Range<usize>,
    pub alts: Vec<Vec<Tok>>,
}

pub fn run(s: &mut Session) -> bool {
    let mut changed = false;
    for file in s.file_order() {
        changed |= apply(s, &file, sites_literals);
        changed |= apply(s, &file, sites_types);
        changed |= apply(s, &file, sites_attrs);
        changed |= apply_inline(s, &file);
    }
    changed
}

fn apply(s: &mut Session, file: &str, find: fn(&[Tok]) -> Vec<Site>) -> bool {
    let Some(text) = s.project.files.get(file) else {
        return false;
    };
    let Some(mut toks) = scan(text) else {
        return false;
    };
    let mut sites = find(&toks);
    sites.sort_by_key(|x| std::cmp::Reverse(x.range.start));
    let mut flat: Vec<(Range<usize>, Vec<Tok>)> = Vec::new();
    for site in sites {
        for alt in site.alts {
            if alt.len() < site.range.len() {
                flat.push((site.range.clone(), alt));
            }
        }
    }
    let mut changed = false;
    let mut at = 0;
    while at < flat.len() && !s.eng.exhausted() {
        let end = (at + s.eng.jobs).min(flat.len());
        let cands: Vec<_> = flat[at..end]
            .iter()
            .map(|(r, alt)| {
                s.project
                    .with_file(file, render(&replace_range(&toks, r.clone(), alt.clone())))
            })
            .collect();
        match s.adopt_first(cands) {
            Some(k) => {
                let (r, alt) = flat[at + k].clone();
                toks = replace_range(&toks, r.clone(), alt);
                changed = true;
                at += k + 1;
                // later entries that overlap the edited range are stale
                while at < flat.len() && flat[at].0.end > r.start {
                    at += 1;
                }
            }
            None => at = end,
        }
    }
    changed
}

/// Inlining grows the call site, so it is applied outside the shrink-only filter of [`apply`]; it is
/// restricted to a function with a single call, so the follow-up deletion of the function shrinks the file.
fn apply_inline(s: &mut Session, file: &str) -> bool {
    let Some(toks) = s.project.files.get(file).and_then(|t| scan(t)) else {
        return false;
    };
    for site in sites_inline(&toks).into_iter().rev() {
        if s.eng.exhausted() {
            break;
        }
        let c = s.project.with_file(
            file,
            render(&replace_range(
                &toks,
                site.range.clone(),
                site.alts[0].clone(),
            )),
        );
        if s.adopt_first(vec![c]).is_some() {
            return true;
        }
    }
    false
}

/// `fn name() { expr }` called exactly once as `name()` -> the call becomes `(expr)`.
pub fn sites_inline(toks: &[Tok]) -> Vec<Site> {
    let gs = groups(toks);
    let mut out = Vec::new();
    for i in 0..toks.len().saturating_sub(4) {
        if !(toks[i].text == "fn"
            && toks[i + 1].kind == TokKind::Word
            && toks[i + 2].is("(")
            && toks[i + 3].is(")"))
        {
            continue;
        }
        let name = &toks[i + 1].text;
        let Some(body) = gs
            .iter()
            .find(|g| {
                g.kind == GroupKind::Block
                    && g.open > i + 3
                    && toks[i + 4..g.open].iter().all(|t| !t.is("{"))
            })
            .cloned()
        else {
            continue;
        };
        let inner = body.inner();
        let units_in = units(toks, &body);
        let simple = units_in.len() == 1
            && toks[inner.clone()].iter().all(|t| {
                !matches!(
                    t.text.as_str(),
                    "return" | "let" | "if" | "match" | "for" | "while" | "loop" | "{" | "}"
                )
            });
        if !simple || toks[inner.clone()].iter().any(|t| &t.text == name) {
            continue;
        }
        let calls: Vec<usize> = (0..toks.len().saturating_sub(2))
            .filter(|&j| {
                &toks[j].text == name
                    && toks[j + 1].is("(")
                    && toks[j + 2].is(")")
                    && j > 0
                    && !matches!(toks[j - 1].text.as_str(), "fn" | "." | "::")
            })
            .collect();
        let others = toks
            .iter()
            .enumerate()
            .filter(|(j, t)| &t.text == name && *j != i + 1 && !calls.contains(j))
            .count();
        if let [call] = calls[..] {
            if others == 0 {
                let mut alt = vec![word("(")];
                alt.extend(toks[inner].iter().cloned());
                alt.push(word(")"));
                out.push(Site {
                    range: call..call + 3,
                    alts: vec![alt],
                });
            }
        }
    }
    out
}

fn literals() -> Vec<Vec<Tok>> {
    vec![vec![word("0")], vec![word("false")], vec![string_lit()]]
}

fn is_single_literal(t: &[Tok]) -> bool {
    t.len() == 1
        && (t[0].kind == TokKind::Str
            || t[0].text.chars().all(|c| c.is_ascii_digit())
            || matches!(t[0].text.as_str(), "true" | "false"))
}

/// Call arguments and right-hand sides of `=`.
pub fn sites_literals(toks: &[Tok]) -> Vec<Site> {
    let mut out = Vec::new();
    for g in groups(toks) {
        match g.kind {
            GroupKind::Paren | GroupKind::Bracket => {
                for u in units(toks, &g) {
                    let end = if toks[u.end - 1].is(",") {
                        u.end - 1
                    } else {
                        u.end
                    };
                    // `name: Type` parameter lists are not expressions; skip units with a top-level `:`
                    if end > u.start
                        && !is_single_literal(&toks[u.start..end])
                        && !looks_like_param(&toks[u.start..end])
                    {
                        out.push(Site {
                            range: u.start..end,
                            alts: literals(),
                        });
                    }
                }
            }
            GroupKind::Block | GroupKind::Root => out.extend(rhs_sites(toks, &g)),
            GroupKind::Angle => {}
        }
    }
    out
}

fn looks_like_param(t: &[Tok]) -> bool {
    t.len() >= 3 && t[0].kind == TokKind::Word && t[1].is(":")
}

fn rhs_sites(toks: &[Tok], g: &Group) -> Vec<Site> {
    let mut out = Vec::new();
    for u in units(toks, g) {
        let mut depth = 0i32;
        for i in u.clone() {
            let t = &toks[i];
            if t.is_opener() {
                depth += 1;
            } else if t.is_closer() {
                depth -= 1;
            } else if depth == 0
                && t.is("=")
                && i + 1 < u.end
                && !is_single_literal(&toks[i + 1..u.end])
            {
                out.push(Site {
                    range: i + 1..u.end,
                    alts: literals(),
                });
                break;
            }
        }
    }
    out
}

/// `Name<A, B>` -> `A` for capitalised applications in type-ish positions.
pub fn sites_types(toks: &[Tok]) -> Vec<Site> {
    let mut out = Vec::new();
    let mut i = 0;
    while i + 2 < toks.len() {
        let t = &toks[i];
        let after_type_colon = i > 0 && (toks[i - 1].is(":") || toks[i - 1].is("->"));
        if after_type_colon
            && t.kind == TokKind::Word
            && t.text.starts_with(|c: char| c.is_uppercase())
            && toks[i + 1].is("<")
        {
            if let Some((first_end, close)) = angle_args(toks, i + 1) {
                out.push(Site {
                    range: i..close + 1,
                    alts: vec![toks[i + 2..first_end].to_vec()],
                });
            }
        }
        i += 1;
    }
    out
}

/// For `<` at `open`: (end of the first type argument, index of the matching `>`).
fn angle_args(toks: &[Tok], open: usize) -> Option<(usize, usize)> {
    let mut depth = 0i32;
    let mut first_end = None;
    for (i, t) in toks.iter().enumerate().skip(open) {
        match t.text.as_str() {
            "<" => depth += 1,
            ">" => {
                depth -= 1;
                if depth == 0 {
                    return Some((first_end.unwrap_or(i), i));
                }
            }
            ">>" if depth >= 2 => {
                depth -= 2;
                if depth == 0 {
                    return None; // closes two applications at once; skip
                }
            }
            "," if depth == 1 && first_end.is_none() => first_end = Some(i),
            ";" | "{" | "}" | "(" if t.kind == TokKind::Punct => return None,
            _ => {}
        }
    }
    None
}

/// `@name` and `@name(...)` removal.
pub fn sites_attrs(toks: &[Tok]) -> Vec<Site> {
    let mut out = Vec::new();
    for (i, t) in toks.iter().enumerate() {
        if t.kind != TokKind::Decorator {
            continue;
        }
        let mut end = i + 1;
        if toks.get(end).is_some_and(|n| n.is("(") && !n.space) {
            if let Some(g) = groups(toks)
                .into_iter()
                .find(|g| g.kind == GroupKind::Paren && g.open == end)
            {
                end = g.close + 1;
            }
        }
        out.push(Site {
            range: i..end,
            alts: vec![vec![]],
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(s: &str) -> Vec<Tok> {
        scan(s).unwrap()
    }

    #[test]
    fn finds_argument_and_rhs_sites_but_not_parameters() {
        let toks = t("fn f(a: int) {\n    let x = g(a + 1, h(2))\n}\n");
        let sites = sites_literals(&toks);
        let texts: Vec<String> = sites
            .iter()
            .map(|s| {
                toks[s.range.clone()]
                    .iter()
                    .map(|t| t.text.as_str())
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect();
        assert!(texts.contains(&"g ( a + 1 , h ( 2 ) )".to_string()));
        assert!(texts.contains(&"a + 1".to_string()));
        assert!(texts.contains(&"h ( 2 )".to_string()));
        assert!(!texts.iter().any(|s| s == "a : int"));
    }

    #[test]
    fn type_site_unwraps_first_argument() {
        let toks = t("fn f(x: Vec<Option<int>>) -> Map<str, int> {}\n");
        let sites = sites_types(&toks);
        assert_eq!(
            sites.len(),
            1,
            "the nested `>>` form is skipped, the plain one found"
        );
        assert_eq!(sites[0].alts[0][0].text, "str");
    }

    #[test]
    fn inline_site_for_a_single_call_of_a_trivial_function() {
        let toks = t("fn k() -> int {\n    40 + 2\n}\nfn main() {\n    let a = k()\n}\n");
        let sites = sites_inline(&toks);
        assert_eq!(sites.len(), 1);
        let text: Vec<_> = sites[0].alts[0].iter().map(|t| t.text.as_str()).collect();
        assert_eq!(text, ["(", "40", "+", "2", ")"]);
        let twice =
            t("fn k() -> int {\n    1\n}\nfn main() {\n    let a = k()\n    let b = k()\n}\n");
        assert!(sites_inline(&twice).is_empty());
    }

    #[test]
    fn attribute_sites_cover_their_arguments() {
        let toks = t("@derive(Debug, Clone)\nstruct S {}\n");
        let sites = sites_attrs(&toks);
        assert_eq!(sites[0].range, 0..6);
    }
}
