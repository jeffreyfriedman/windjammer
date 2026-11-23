//! Rust's privacy rule on resolved definitions: the one check `wj check`, `wj build` and the interpreter share.
//!
//! An item not declared `pub` is visible in its own module and in that module's descendants (within one crate).
//! Using it anywhere else is an error: calling it, importing it with `use`, or re-exporting it with `pub use`.

use super::collect::Collected;
use super::def::{Def, DefId, DefKind};
use super::lookup::Found;
use super::res::{CallKind, Res};
use super::ResolvedProgram;
use crate::source_map::Location;

/// How the private item was reached.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrivateUseKind {
    /// A qualified or imported call / constant / static use.
    Call,
    /// `use path::Item`.
    Import,
    /// `pub use path::Item`: the module would expose an item it cannot name publicly.
    ReExport,
}

/// One use of a private item from a module that may not see it.
#[derive(Debug, Clone)]
pub struct PrivateUse {
    pub def: DefId,
    pub kind: PrivateUseKind,
    /// The path as written at the use site (`mesh_generator::generate_naive_mesh`, `shapes::hidden`).
    pub written: String,
    pub location: Option<Location>,
}

/// rustc's noun for a definition kind (E0603 wording).
pub fn noun(kind: DefKind) -> &'static str {
    match kind {
        DefKind::Fn | DefKind::ExternFn => "function",
        DefKind::Struct => "struct",
        DefKind::Enum => "enum",
        DefKind::Trait => "trait",
        DefKind::Const => "constant",
        DefKind::Static => "static",
        DefKind::Module => "module",
        DefKind::TypeAlias => "type alias",
        DefKind::Method { .. } => "method",
        DefKind::AssocFn { .. } => "associated function",
        _ => "item",
    }
}

/// The declaration keyword that makes the item public (`pub fn`, `pub struct`, ...).
pub fn pub_declaration(kind: DefKind) -> &'static str {
    match kind {
        DefKind::Struct => "pub struct",
        DefKind::Enum => "pub enum",
        DefKind::Trait => "pub trait",
        DefKind::Const => "pub const",
        DefKind::Static => "pub static",
        DefKind::Module => "pub mod",
        DefKind::TypeAlias => "pub type",
        _ => "pub fn",
    }
}

impl Collected<'_> {
    fn owning_module(&self, d: &Def) -> DefId {
        match d.kind {
            DefKind::Module => self.parent_of(d.id).unwrap_or(d.id),
            _ => d.module,
        }
    }

    fn is_ancestor_or_self(&self, anc: DefId, mut m: DefId) -> bool {
        loop {
            if m == anc {
                return true;
            }
            if self.crate_roots.contains(&m) {
                return false;
            }
            match self.parent_of(m) {
                Some(p) => m = p,
                None => return false,
            }
        }
    }

    fn crate_of(&self, mut m: DefId) -> DefId {
        loop {
            if self.crate_roots.contains(&m) {
                return m;
            }
            match self.parent_of(m) {
                Some(p) => m = p,
                None => return m,
            }
        }
    }

    /// Visible from module `from`: public items everywhere (file modules are always `pub mod`), private ones only
    /// in their module's subtree within the same crate.
    pub fn is_visible_from(&self, d: &Def, from: DefId) -> bool {
        if d.is_pub || matches!(d.kind, DefKind::Variant { .. }) {
            return true;
        }
        let owner = self.owning_module(d);
        self.crate_of(owner) == self.crate_of(from) && self.is_ancestor_or_self(owner, from)
    }
}

/// Every use of a private item outside its visibility: imports and re-exports first (in module order), then calls.
pub fn private_uses(r: &ResolvedProgram<'_>) -> Vec<PrivateUse> {
    let c = &r.collected;
    let mut out = Vec::new();
    let mut scopes: Vec<_> = c.modules.iter().collect();
    scopes.sort_by_key(|(d, _)| **d);
    for (module, scope) in scopes {
        for u in &scope.uses {
            let mut found: Vec<Found> = c.resolve_type_path(*module, &u.segments);
            found.extend(c.lookup_path_value(*module, &u.segments));
            let bad = found.iter().find_map(|f| match f {
                Found::Def(d) if !c.is_visible_from(c.defs.get(*d), *module) => Some(*d),
                _ => None,
            });
            if let Some(def) = bad {
                let kind = if u.is_pub { PrivateUseKind::ReExport } else { PrivateUseKind::Import };
                out.push(PrivateUse { def, kind, written: u.segments.join("::"), location: u.location.clone() });
            }
        }
    }
    // a call through a private import or re-export is reported once, at the import
    let reported: Vec<DefId> = out.iter().map(|u| u.def).collect();
    for call in &r.calls {
        if call.kind != CallKind::Path && call.kind != CallKind::Free {
            continue;
        }
        let Res::Def(id) = &call.res else { continue };
        let d = c.defs.get(*id);
        if !matches!(d.kind, DefKind::Fn | DefKind::ExternFn | DefKind::AssocFn { .. } | DefKind::Const | DefKind::Static) {
            continue;
        }
        if !c.is_visible_from(d, call.module) && !reported.contains(id) {
            out.push(PrivateUse { def: *id, kind: PrivateUseKind::Call, written: call.callee.clone(), location: call.location.clone() });
        }
    }
    out
}

impl PrivateUse {
    /// `function `a::b::f` is private` (the path as written): the same wording in `wj check`, `wj build` and the interpreter.
    pub fn message(&self, r: &ResolvedProgram<'_>) -> String {
        let d = r.defs().get(self.def);
        format!("{} `{}` is private", noun(d.kind), self.written)
    }

    /// The fix: declare the item public, naming it and its file.
    pub fn help(&self, r: &ResolvedProgram<'_>) -> String {
        let d = r.defs().get(self.def);
        let file = d.location.as_ref().map(|l| format!(" in {}", l.file.display())).unwrap_or_default();
        format!("declare it `{} {}`{file}", pub_declaration(d.kind), d.name)
    }
}
