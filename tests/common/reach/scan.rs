//! Item extraction for the dead-capability gate (name-based call graph over `syn` ASTs).
//!
//! Every function, method, type, trait, const, static and macro becomes an [`Item`] with the set of
//! identifiers its definition mentions (`refs`). Test-only code (`#[cfg(test)]`, `#[test]`) is
//! skipped: a capability reachable only from tests is exactly what the gate must report.

use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    Fn,
    /// Method of an inherent impl: reachable only if its name is used AND its self type is reached.
    Method { self_ty: String },
    /// Method of a trait impl: reachable as soon as its self type is reached (dynamic dispatch).
    TraitMethod { self_ty: String },
    Type,
    Other,
}

#[derive(Debug, Clone)]
pub struct Item {
    pub file: String,
    /// Module file whose privacy scope the item belongs to (differs from `file` for `include!`d files).
    pub scope: String,
    pub name: String,
    pub kind: Kind,
    pub line: usize,
    pub is_pub: bool,
    pub refs: BTreeSet<String>,
}

fn is_test_attr(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|a| {
        let p = a.path();
        if p.is_ident("test") {
            return true;
        }
        if p.is_ident("cfg") {
            let s = a.meta.to_token_stream_string();
            return s.contains("test") && !s.contains("not (test") && !s.contains("not(test");
        }
        false
    })
}

trait MetaString {
    fn to_token_stream_string(&self) -> String;
}
impl MetaString for syn::Meta {
    fn to_token_stream_string(&self) -> String {
        quote::ToTokens::to_token_stream(self).to_string()
    }
}

/// Collects every identifier, including those inside macro invocations (token streams).
#[derive(Default)]
struct Idents(BTreeSet<String>);

impl Idents {
    fn tokens(&mut self, ts: proc_macro2::TokenStream) {
        for t in ts {
            match t {
                proc_macro2::TokenTree::Ident(i) => {
                    self.0.insert(i.to_string());
                }
                proc_macro2::TokenTree::Group(g) => self.tokens(g.stream()),
                // `#[serde(with = "module")]`, `serialize_with = "path::f"`: names in strings count.
                proc_macro2::TokenTree::Literal(l) => {
                    let s = l.to_string();
                    if s.starts_with('"') {
                        for w in s.split(|c: char| !(c.is_alphanumeric() || c == '_')).filter(|w| !w.is_empty()) {
                            self.0.insert(w.to_string());
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

fn refs_of<T: quote::ToTokens>(node: &T) -> BTreeSet<String> {
    // Token-level collection is exact for identifiers and also covers macro bodies.
    let mut ids = Idents::default();
    ids.tokens(node.to_token_stream());
    ids.0
}

fn type_name(ty: &syn::Type) -> String {
    match ty {
        syn::Type::Path(p) => p.path.segments.last().map(|s| s.ident.to_string()).unwrap_or_default(),
        syn::Type::Reference(r) => type_name(&r.elem),
        _ => String::new(),
    }
}

fn is_pub(v: &syn::Visibility) -> bool {
    !matches!(v, syn::Visibility::Inherited)
}

struct Collector<'f> {
    file: &'f str,
    items: Vec<Item>,
}

impl Collector<'_> {
    fn push(&mut self, name: String, kind: Kind, span: proc_macro2::Span, is_pub: bool, refs: BTreeSet<String>) {
        self.items.push(Item { file: self.file.to_string(), scope: self.file.to_string(), name, kind, line: span.start().line, is_pub, refs });
    }

    fn items(&mut self, items: &[syn::Item]) {
        for it in items {
            self.item(it);
        }
    }

    fn item(&mut self, it: &syn::Item) {
        use syn::Item as I;
        match it {
            I::Fn(f) if !is_test_attr(&f.attrs) => {
                let refs = refs_of(f);
                self.push(f.sig.ident.to_string(), Kind::Fn, f.sig.ident.span(), is_pub(&f.vis), refs)
            }
            I::Mod(m) if !is_test_attr(&m.attrs) => {
                if let Some((_, inner)) = &m.content {
                    self.items(inner);
                }
            }
            I::Impl(im) if !is_test_attr(&im.attrs) => self.imp(im),
            I::Struct(s) if !is_test_attr(&s.attrs) => {
                self.push(s.ident.to_string(), Kind::Type, s.ident.span(), is_pub(&s.vis), refs_of(s))
            }
            I::Enum(e) if !is_test_attr(&e.attrs) => {
                self.push(e.ident.to_string(), Kind::Type, e.ident.span(), is_pub(&e.vis), refs_of(e))
            }
            I::Type(t) if !is_test_attr(&t.attrs) => {
                self.push(t.ident.to_string(), Kind::Type, t.ident.span(), is_pub(&t.vis), refs_of(t))
            }
            I::Trait(t) if !is_test_attr(&t.attrs) => {
                self.push(t.ident.to_string(), Kind::Type, t.ident.span(), is_pub(&t.vis), refs_of(t))
            }
            I::Const(c) if !is_test_attr(&c.attrs) => {
                self.push(c.ident.to_string(), Kind::Other, c.ident.span(), is_pub(&c.vis), refs_of(c))
            }
            I::Static(s) if !is_test_attr(&s.attrs) => {
                self.push(s.ident.to_string(), Kind::Other, s.ident.span(), is_pub(&s.vis), refs_of(s))
            }
            I::Macro(m) if !is_test_attr(&m.attrs) => {
                if let Some(id) = &m.ident {
                    self.push(id.to_string(), Kind::Other, id.span(), true, refs_of(m))
                }
            }
            _ => {}
        }
    }

    fn imp(&mut self, im: &syn::ItemImpl) {
        let self_ty = type_name(&im.self_ty);
        let trait_impl = im.trait_.is_some();
        // The impl header (generics, trait path) belongs to every method.
        let mut header = refs_of(&im.self_ty);
        if let Some((_, p, _)) = &im.trait_ {
            header.extend(refs_of(p));
        }
        for ii in &im.items {
            match ii {
                syn::ImplItem::Fn(f) if !is_test_attr(&f.attrs) => {
                    let mut refs = refs_of(f);
                    refs.extend(header.iter().cloned());
                    let kind = if trait_impl {
                        Kind::TraitMethod { self_ty: self_ty.clone() }
                    } else {
                        Kind::Method { self_ty: self_ty.clone() }
                    };
                    self.push(f.sig.ident.to_string(), kind, f.sig.ident.span(), trait_impl || is_pub(&f.vis), refs)
                }
                syn::ImplItem::Const(c) => {
                    self.push(c.ident.to_string(), Kind::Other, c.ident.span(), is_pub(&c.vis), refs_of(c))
                }
                _ => {}
            }
        }
    }
}

/// Parse one file. A file that `syn` cannot parse yields an `Err` (reported, never silently skipped).
pub fn scan_source(file: &str, src: &str) -> Result<Vec<Item>, String> {
    let ast = syn::parse_file(src).map_err(|e| format!("{file}: {e}"))?;
    if is_test_attr(&ast.attrs) {
        return Ok(Vec::new());
    }
    let mut c = Collector { file, items: Vec::new() };
    c.items(&ast.items);
    Ok(c.items)
}
