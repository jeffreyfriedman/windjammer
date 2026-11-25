//! `map.get(key).unwrap_or("")` over a map of owned text: the value is a borrowed `&String` in Rust and an
//! owned piece of text in Windjammer, so a default that is owned text is applied to a clone of the stored value.

use crate::parser::*;

use crate::codegen::rust::CodeGenerator;
use crate::ir::stdlib_tables::CoreSurface;

impl<'ast> CodeGenerator<'ast> {
    /// A call on an `Option<&T>` whose declared parameter (`std/rust_core.wji`) is the option's payload, given an owned
    /// `T` (`map.get(k).unwrap_or("")`, `outer.get(k).unwrap_or(HashMap::new())`): the call yields an owned `T`,
    /// the stored value cloned. `T` is text or a standard collection. Returns that owned type.
    pub(in crate::codegen::rust) fn ref_default_call_type(
        &self,
        object: &Expression<'ast>,
        method: &str,
        arguments: &[(Option<String>, &'ast Expression<'ast>)],
    ) -> Option<Type> {
        let [(_, arg)] = arguments else { return None };
        // only a method the surface declares on `Option` can qualify: ask that before inferring the receiver, whose
        // type walks the whole call chain (once per link, that is exponential in a long builder chain)
        self.core_surface().declared("Option", method)?;
        let literal_text = matches!(arg, Expression::Literal { value: Literal::String(_), .. });
        let new_collection = matches!(arg, Expression::Call { function, arguments, .. }
            if arguments.is_empty() && matches!(function, Expression::FieldAccess { .. } | Expression::Identifier { .. }));
        if !literal_text && !new_collection && !matches!(self.infer_expression_type(arg), Some(Type::String)) {
            return None;
        }
        let recv = self.infer_expression_type(object)?;
        let Type::Option(payload) = &recv else { return None };
        let Type::Reference(target) = payload.as_ref() else { return None };
        let head_matches = |t: &Type| crate::type_classification::type_to_registry_base(t) == crate::type_classification::type_to_registry_base(target);
        let owned_default = if new_collection {
            self.infer_expression_type(arg).is_some_and(|t| head_matches(&t))
        } else {
            crate::codegen::rust::types::is_windjammer_text_type(target)
        };
        let cloneable = crate::codegen::rust::types::is_windjammer_text_type(target)
            || crate::type_classification::type_is_vec_container(target)
            || matches!(target.as_ref(), Type::Parameterized(n, _) | Type::Custom(n) if crate::type_classification::is_map_type_name(n));
        (owned_default && cloneable && self.core_surface().method_signature(&recv, method, &[])?.params.first() == Some(payload.as_ref()))
            .then(|| target.as_ref().clone())
    }

    /// Whether `object` is `producer(..).<adapters>.next()` (or `last`, `nth`) over an iterator the surface declares to
    /// yield `&str` pieces (`split`, `lines`, `split_whitespace`), through adapters that keep the item.
    fn option_of_text_piece(&self, object: &Expression<'ast>) -> bool {
        let surface = self.core_surface();
        let keeps_item = |method: &str| {
            matches!(surface.declared(crate::ir::stdlib_tables::ITERATOR, method),
                Some((generics, _, _, Some(Type::Parameterized(_, p))))
                    if matches!((p.as_slice(), generics.first()), ([Type::Generic(x) | Type::Custom(x)], Some(t)) if x == t))
        };
        let Expression::MethodCall { object: mut cur, method, .. } = object else { return false };
        // the consumer hands back one item: `Option<T>` over the iterator's own `T`
        let hands_back_item = matches!(surface.declared(crate::ir::stdlib_tables::ITERATOR, method),
            Some((generics, _, _, Some(Type::Option(r)))) if matches!((r.as_ref(), generics.first()), (Type::Generic(x) | Type::Custom(x), Some(t)) if x == t));
        if !hands_back_item {
            return false;
        }
        while let Expression::MethodCall { object: inner, method, .. } = cur {
            if keeps_item(method) {
                cur = inner;
                continue;
            }
            return matches!(surface.unique_iterator_item(method),
                Some(Type::Reference(t)) if crate::codegen::rust::types::is_windjammer_text_type(&t));
        }
        false
    }

    /// `opt.unwrap_or(owned)` over `Option<&T>` becomes `opt.cloned().unwrap_or(owned)`.
    pub(super) fn mc_lower_ref_text_option_default(
        &self,
        object: &Expression<'ast>,
        method: &str,
        arguments: &[(Option<String>, &'ast Expression<'ast>)],
        args: &[String],
        obj_str: &str,
    ) -> Option<String> {
        if args.len() != 1 || obj_str.ends_with(".cloned()") || obj_str.ends_with(".map(|s| s.to_string())") || self.ref_default_call_type(object, method, arguments).is_none() {
            return None;
        }
        // `Option<&str>` (`strip_suffix`) owns its payload by `to_string`; `Option<&String>`/`&Vec`/`&Map` by `cloned`.
        // `text.split(sep).last()`: the declaration of the producer says its pieces are `&str` (the receiver's inferred
        // item type is a stand-in), and `Option<&str>` owns its payload by `to_string`, as `Option<&&str>` does.
        let target_is_str = self.option_of_text_piece(object)
            || matches!(self.infer_expression_type(object), Some(Type::Option(p))
                if matches!(crate::codegen::rust::collection_detection::peel_type_reference(&p), Type::Custom(n) if n == "str"));
        let own = if target_is_str { ".map(|s| s.to_string())" } else { ".cloned()" };
        // the payload is now owned text, so the default must be too: a borrowed piece of text (`&str` parameter) is copied
        let default = match arguments.first() {
            Some((_, arg)) if target_is_str && self.expression_produces_str_ref(arg) => {
                format!("{}.to_string()", args[0].strip_suffix(".clone()").unwrap_or(&args[0]))
            }
            _ => args[0].clone(),
        };
        Some(format!("{obj_str}{own}.{method}({default})"))
    }
}
