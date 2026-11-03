//! `#[cfg(..)]` on crate items, evaluated for the features the build enables.
//!
//! `test` and `doc` are off (not part of a dependency's API); `feature = ".."` follows the enabled set;
//! `not`/`all`/`any` combine. Anything that depends on the target (`unix`, `target_os`, ...) is `Unknown`:
//! the generator reports such an item instead of assuming the host's answer is every user's answer.

use std::collections::BTreeSet;
use syn::punctuated::Punctuated;
use syn::{Attribute, Meta, Token};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cfg {
    On,
    Off,
    /// Depends on the target; the text is the predicate as written.
    Unknown(String),
}

/// The combined `#[cfg]` of an item's attributes.
pub fn of(attrs: &[Attribute], features: &BTreeSet<String>) -> Cfg {
    let mut acc = Cfg::On;
    for a in attrs.iter().filter(|a| a.path().is_ident("cfg")) {
        let Meta::List(list) = &a.meta else { continue };
        let Ok(inner) = list.parse_args::<Meta>() else {
            return Cfg::Unknown(list.tokens.to_string());
        };
        acc = and(acc, eval(&inner, features));
    }
    acc
}

fn and(a: Cfg, b: Cfg) -> Cfg {
    match (a, b) {
        (Cfg::Off, _) | (_, Cfg::Off) => Cfg::Off,
        (Cfg::Unknown(w), _) | (_, Cfg::Unknown(w)) => Cfg::Unknown(w),
        _ => Cfg::On,
    }
}

fn or(a: Cfg, b: Cfg) -> Cfg {
    match (a, b) {
        (Cfg::On, _) | (_, Cfg::On) => Cfg::On,
        (Cfg::Unknown(w), _) | (_, Cfg::Unknown(w)) => Cfg::Unknown(w),
        _ => Cfg::Off,
    }
}

fn eval(m: &Meta, features: &BTreeSet<String>) -> Cfg {
    let unknown = || Cfg::Unknown(format!("cfg({})", super::map::show(m)));
    match m {
        Meta::Path(p) if p.is_ident("test") || p.is_ident("doc") || p.is_ident("miri") => Cfg::Off,
        Meta::NameValue(nv) if nv.path.is_ident("feature") => match &nv.value {
            syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(s), .. }) => {
                if features.contains(&s.value()) {
                    Cfg::On
                } else {
                    Cfg::Off
                }
            }
            _ => unknown(),
        },
        Meta::List(l) => {
            let Ok(args) = l.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated) else { return unknown() };
            let vals: Vec<Cfg> = args.iter().map(|a| eval(a, features)).collect();
            if l.path.is_ident("not") && vals.len() == 1 {
                match &vals[0] {
                    Cfg::On => Cfg::Off,
                    Cfg::Off => Cfg::On,
                    Cfg::Unknown(_) => unknown(),
                }
            } else if l.path.is_ident("all") {
                vals.into_iter().fold(Cfg::On, and)
            } else if l.path.is_ident("any") {
                vals.into_iter().fold(Cfg::Off, or)
            } else {
                unknown()
            }
        }
        _ => unknown(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg_of(src: &str, feats: &[&str]) -> Cfg {
        let f: syn::ItemFn = syn::parse_str(&format!("{src} fn f() {{}}")).unwrap();
        of(&f.attrs, &feats.iter().map(|s| s.to_string()).collect())
    }

    #[test]
    fn evaluation() {
        assert_eq!(cfg_of("", &[]), Cfg::On);
        assert_eq!(cfg_of("#[cfg(test)]", &[]), Cfg::Off);
        assert_eq!(cfg_of("#[cfg(feature = \"std\")]", &["std"]), Cfg::On);
        assert_eq!(cfg_of("#[cfg(feature = \"std\")]", &[]), Cfg::Off);
        assert_eq!(cfg_of("#[cfg(not(feature = \"std\"))]", &[]), Cfg::On);
        assert_eq!(cfg_of("#[cfg(any(test, feature = \"a\"))]", &["a"]), Cfg::On);
        assert_eq!(cfg_of("#[cfg(all(unix, feature = \"a\"))]", &[]), Cfg::Off);
        assert!(matches!(cfg_of("#[cfg(unix)]", &[]), Cfg::Unknown(w) if w == "cfg(unix)"));
        assert!(matches!(cfg_of("#[cfg(target_os = \"linux\")]", &[]), Cfg::Unknown(_)));
    }
}
