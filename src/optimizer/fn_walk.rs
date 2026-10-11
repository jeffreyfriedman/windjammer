//! Visit every function with a body (free, impl, trait default, nested).

use syn::visit::Visit;
use syn::{Attribute, Block, ImplItemFn, ItemFn, Signature, TraitItemFn};

pub struct FnRef<'a> {
    pub attrs: &'a [Attribute],
    pub sig: &'a Signature,
    pub body: &'a Block,
}

struct Walker<'f, 'ast> {
    f: &'f mut dyn FnMut(FnRef<'ast>),
}

impl<'f, 'ast> Visit<'ast> for Walker<'f, 'ast> {
    fn visit_item_fn(&mut self, n: &'ast ItemFn) {
        (self.f)(FnRef { attrs: &n.attrs, sig: &n.sig, body: &n.block });
        syn::visit::visit_item_fn(self, n);
    }
    fn visit_impl_item_fn(&mut self, n: &'ast ImplItemFn) {
        (self.f)(FnRef { attrs: &n.attrs, sig: &n.sig, body: &n.block });
        syn::visit::visit_impl_item_fn(self, n);
    }
    fn visit_trait_item_fn(&mut self, n: &'ast TraitItemFn) {
        if let Some(b) = &n.default {
            (self.f)(FnRef { attrs: &n.attrs, sig: &n.sig, body: b });
        }
        syn::visit::visit_trait_item_fn(self, n);
    }
}

pub fn for_each_fn<'ast>(file: &'ast syn::File, mut f: impl FnMut(FnRef<'ast>)) {
    Walker { f: &mut f }.visit_file(file);
}
