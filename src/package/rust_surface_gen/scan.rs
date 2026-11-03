//! Read a crate's public free functions from its source with `syn` (stable Rust, the exact sources Cargo
//! locked). What a path call `crate::f` can reach: `pub fn` items of the crate root, and functions the root
//! re-exports with `pub use` from its own modules. Items behind a `#[cfg]` that does not hold for the enabled
//! features, and items of other crates, are reported, never guessed.

use super::cfg::{self, Cfg};
use super::map::Aliases;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use syn::{Item, UseTree, Visibility};

/// A public free function reachable as `crate::name`.
pub struct Found {
    pub name: String,
    pub item: syn::ItemFn,
}

/// What a scan saw.
#[derive(Default)]
pub struct Scan {
    pub fns: Vec<Found>,
    /// `(name, reason)` for reachable names that are not mappable functions for a reason found here.
    pub skipped: Vec<(String, String)>,
    pub aliases: Aliases,
    /// Files read (relative path, contents), the input of the source hash.
    pub files: BTreeMap<String, String>,
    /// Facts about what may be missing (macro-generated items).
    pub notes: Vec<String>,
}

/// One parsed module: its items and the directory its child modules live in.
struct Module {
    items: Vec<Item>,
    child_dir: PathBuf,
}

pub fn scan(crate_dir: &Path, lib_rel: &str, features: &BTreeSet<String>) -> Result<Scan, String> {
    let mut s = Scan::default();
    let lib = crate_dir.join(lib_rel);
    let root = read_module(crate_dir, &lib, &mut s)?;
    let root = Module { items: root, child_dir: lib.parent().unwrap_or(crate_dir).to_path_buf() };
    collect_aliases(&root.items, &mut s.aliases);
    let mut macro_calls = 0usize;
    for item in &root.items {
        match cfg::of(item_attrs(item), features) {
            Cfg::Off => continue,
            Cfg::Unknown(why) => {
                if let Some(name) = public_fn_name(item) {
                    s.skipped.push((name, format!("only compiled under `{why}`")));
                } else if matches!(item, Item::Use(u) if is_pub(&u.vis)) {
                    s.notes.push(format!("a `pub use` compiled only under `{why}` was not followed"));
                }
                continue;
            }
            Cfg::On => {}
        }
        match item {
            Item::Fn(f) if is_pub(&f.vis) => s.fns.push(Found { name: f.sig.ident.to_string(), item: f.clone() }),
            Item::Use(u) if is_pub(&u.vis) => reexports(crate_dir, &root, &u.tree, Vec::new(), features, &mut s),
            Item::Macro(m) if m.ident.is_none() && !m.mac.path.is_ident("macro_rules") => macro_calls += 1,
            _ => {}
        }
    }
    if macro_calls > 0 {
        s.notes.push(format!(
            "the crate root has {macro_calls} macro invocation(s); functions a macro defines are not visible to the generator"
        ));
    }
    Ok(s)
}

fn read_module(crate_dir: &Path, file: &Path, s: &mut Scan) -> Result<Vec<Item>, String> {
    let text = std::fs::read_to_string(file).map_err(|e| format!("cannot read {}: {e}", file.display()))?;
    let parsed = syn::parse_file(&text).map_err(|e| format!("cannot parse {}: {e}", file.display()))?;
    let rel = file.strip_prefix(crate_dir).unwrap_or(file).to_string_lossy().replace('\\', "/");
    s.files.insert(rel, text);
    Ok(parsed.items)
}

fn item_attrs(item: &Item) -> &[syn::Attribute] {
    match item {
        Item::Fn(f) => &f.attrs,
        Item::Use(u) => &u.attrs,
        Item::Mod(m) => &m.attrs,
        Item::Type(t) => &t.attrs,
        Item::Macro(m) => &m.attrs,
        _ => &[],
    }
}

fn public_fn_name(item: &Item) -> Option<String> {
    match item {
        Item::Fn(f) if is_pub(&f.vis) => Some(f.sig.ident.to_string()),
        _ => None,
    }
}

fn is_pub(v: &Visibility) -> bool {
    matches!(v, Visibility::Public(_))
}

fn collect_aliases(items: &[Item], out: &mut Aliases) {
    for item in items {
        if let Item::Type(t) = item {
            if t.generics.params.is_empty() {
                out.insert(t.ident.to_string(), (*t.ty).clone());
            }
        }
    }
}

/// Follow `pub use <path>` from the root: `self::m::f`, `crate::m::{f, g as h}`, `m::*`.
fn reexports(crate_dir: &Path, root: &Module, tree: &UseTree, prefix: Vec<String>, features: &BTreeSet<String>, s: &mut Scan) {
    match tree {
        UseTree::Path(p) => {
            let mut next = prefix;
            next.push(p.ident.to_string());
            reexports(crate_dir, root, &p.tree, next, features, s);
        }
        UseTree::Group(g) => g.items.iter().for_each(|t| reexports(crate_dir, root, t, prefix.clone(), features, s)),
        UseTree::Name(n) => import(crate_dir, root, &prefix, Some((n.ident.to_string(), n.ident.to_string())), features, s),
        UseTree::Rename(r) => import(crate_dir, root, &prefix, Some((r.ident.to_string(), r.rename.to_string())), features, s),
        UseTree::Glob(_) => import(crate_dir, root, &prefix, None, features, s),
    }
}

/// Bring `prefix::name as alias` (or `prefix::*` when `name` is `None`) into the root.
fn import(crate_dir: &Path, root: &Module, prefix: &[String], name: Option<(String, String)>, features: &BTreeSet<String>, s: &mut Scan) {
    let path: Vec<&str> = prefix.iter().map(String::as_str).skip_while(|p| *p == "self" || *p == "crate").collect();
    let external = prefix.first().is_some_and(|h| h != "self" && h != "crate" && !root_has_mod(root, h));
    if external || path.is_empty() {
        if let Some((_, alias)) = name {
            if external {
                s.skipped.push((alias, format!("re-exported from `{}`, outside this crate", prefix.join("::"))));
            }
        }
        return;
    }
    let Some(module) = descend(crate_dir, root, &path, features, s) else {
        if let Some((_, alias)) = name {
            s.skipped.push((alias, format!("re-exported from `{}`, a module the generator could not read", prefix.join("::"))));
        }
        return;
    };
    for item in &module.items {
        let Item::Fn(f) = item else { continue };
        let fname = f.sig.ident.to_string();
        let alias = match &name {
            Some((n, a)) if *n == fname => a.clone(),
            Some(_) => continue,
            None if is_pub(&f.vis) => fname.clone(),
            None => continue,
        };
        match cfg::of(&f.attrs, features) {
            Cfg::On => s.fns.push(Found { name: alias, item: f.clone() }),
            Cfg::Unknown(why) => s.skipped.push((alias, format!("only compiled under `{why}`"))),
            Cfg::Off => {}
        }
    }
}

fn root_has_mod(root: &Module, name: &str) -> bool {
    root.items.iter().any(|i| matches!(i, Item::Mod(m) if m.ident == name))
}

/// The module at `path` below the root, parsed (inline `mod m { .. }`, `m.rs`, `m/mod.rs`, `#[path]`).
fn descend(crate_dir: &Path, root: &Module, path: &[&str], features: &BTreeSet<String>, s: &mut Scan) -> Option<Module> {
    let mut items = root.items.clone();
    let mut dir = root.child_dir.clone();
    for seg in path {
        let m = items.iter().find_map(|i| match i {
            Item::Mod(m) if m.ident == *seg && cfg::of(&m.attrs, features) == Cfg::On => Some(m.clone()),
            _ => None,
        })?;
        if let Some((_, inline)) = m.content {
            items = inline;
            dir = dir.join(seg);
            continue;
        }
        let explicit = m.attrs.iter().find_map(path_attr);
        let candidates = match &explicit {
            Some(p) => vec![dir.join(p)],
            None => vec![dir.join(format!("{seg}.rs")), dir.join(seg).join("mod.rs")],
        };
        let file = candidates.into_iter().find(|c| c.is_file())?;
        items = read_module(crate_dir, &file, s).ok()?;
        dir = if explicit.is_some() || file.ends_with("mod.rs") { file.parent()?.to_path_buf() } else { dir.join(seg) };
        collect_aliases(&items, &mut s.aliases);
    }
    Some(Module { items, child_dir: dir })
}

fn path_attr(a: &syn::Attribute) -> Option<String> {
    let syn::Meta::NameValue(nv) = &a.meta else { return None };
    if !nv.path.is_ident("path") {
        return None;
    }
    match &nv.value {
        syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(s), .. }) => Some(s.value()),
        _ => None,
    }
}
