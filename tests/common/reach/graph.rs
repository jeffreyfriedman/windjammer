//! Reachability fixpoint over [`Item`]s (see `scan.rs`).
//!
//! Rules (an over-approximation: a shared name keeps every same-named item alive, so the gate can
//! miss dead code but never reports live code as dead, except code reached only through macros
//! that build identifiers, e.g. `paste!`):
//! - every item of a root file is reached; `fn main` outside a root is never reached;
//! - a pub free fn, type, const, static or macro is reached when a reached item mentions its name;
//! - a private one only when a reached item of its own module (file or child module) mentions it;
//! - an inherent method is reached when its name AND its self type are mentioned by reached items;
//! - a trait-impl method is reached when its self type is mentioned (dynamic dispatch).

use super::scan::{Item, Kind};
use std::collections::{BTreeMap, BTreeSet};

pub struct Reach {
    pub reached: Vec<bool>,
}

/// Directory prefix of the files that can see `file`'s private items (itself and its child modules).
fn module_scope(file: &str) -> String {
    for leaf in ["mod.rs", "lib.rs", "main.rs"] {
        if let Some(dir) = file.strip_suffix(leaf) {
            return dir.to_string();
        }
    }
    format!("{}/", file.trim_end_matches(".rs"))
}

#[derive(Default)]
struct Seen {
    all: BTreeSet<String>,
    by_file: BTreeMap<String, BTreeSet<String>>,
}

impl Seen {
    fn add(&mut self, it: &Item) {
        self.all.extend(it.refs.iter().cloned());
        self.by_file.entry(it.scope.clone()).or_default().extend(it.refs.iter().cloned());
    }
    fn visible(&self, it: &Item, name: &str) -> bool {
        if it.is_pub {
            return self.all.contains(name);
        }
        let scope = module_scope(&it.scope);
        self.by_file.iter().any(|(f, s)| (f == &it.scope || f.starts_with(&scope)) && s.contains(name))
    }
}

pub fn reach(items: &[Item], is_root: impl Fn(&str) -> bool) -> Reach {
    let mut reached: Vec<bool> = items.iter().map(|i| is_root(&i.file)).collect();
    let mut seen = Seen::default();
    for (i, it) in items.iter().enumerate() {
        if reached[i] {
            seen.add(it);
        }
    }
    loop {
        let mut changed = false;
        for (i, it) in items.iter().enumerate() {
            if reached[i] {
                continue;
            }
            let hit = match &it.kind {
                // `fn main` is an entry point, never a callee: another binary's `main` is not reached by name.
                Kind::Fn if it.name == "main" => false,
                Kind::Fn | Kind::Type | Kind::Other => seen.visible(it, &it.name),
                Kind::Method { self_ty } => seen.visible(it, &it.name) && seen.all.contains(self_ty),
                Kind::TraitMethod { self_ty } => seen.all.contains(self_ty),
            };
            if hit {
                reached[i] = true;
                seen.add(it);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    Reach { reached }
}
