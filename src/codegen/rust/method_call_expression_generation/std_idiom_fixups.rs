//! Std-method idioms an agent writes first: the call-site fixups that make them compile.

use crate::parser::*;

use crate::codegen::rust::CodeGenerator;

impl<'ast> CodeGenerator<'ast> {
    /// Adjust receiver/argument text for `cmp(&x)`, `get(..).unwrap_or(0)`, `unwrap_or(&0)`,
    /// `lines().next().unwrap_or("0")` and direct iterator adapters on collections.
    pub(super) fn mc_fix_std_idiom_call(
        &mut self,
        object: &Expression<'ast>,
        method: &str,
        arguments: &[(Option<String>, &'ast Expression<'ast>)],
        obj_str: &mut String,
        args: &mut [String],
    ) -> Option<String> {
        if let Some(lowered) = self.mc_lower_string_vec_contains(object, method, arguments, obj_str, args) {
            return Some(lowered);
        }
        // `opt.unwrap_or(&0)`: a literal behind an explicit `&` is a reference default (`Option<&T>`
        // from `map.get(k)`); the by-value call-site rules must not strip it.
        if matches!(method, "cmp" | "partial_cmp" | "eq" | "ne" | "lt" | "le" | "gt" | "ge")
            && args.len() == 1
        {
            // The comparison methods take `&Self`: an explicit `&` is the argument, not a hint.
            if let Some((_, Expression::Unary { op: UnaryOp::Ref, .. })) = arguments.first() {
                if !args[0].starts_with('&') {
                    args[0] = format!("&{}", args[0]);
                }
            }
        }
        // `map.get(k).unwrap_or(0)` (read the element by value) is lowered once, from the receiver's
        // `Option<&T>` type, in `idiom_lowering::lower_option_default_and_cmp`.
        // `opt.unwrap_or("lit")` on an `Option<&str>` (`lines().next()`, `split(..).next()`): the default
        // stays `&str`; coercing the literal to an owned `String` is a type error there.
        if method == "unwrap_or" && args.len() == 1 {
            if let Some((_, Expression::Literal { value: Literal::String(_), .. })) = arguments.first() {
                let recv_is_str_ref = obj_str.contains(".lines()")
                    || obj_str.contains(".split(")
                    || obj_str.contains(".split_whitespace()")
                    || self.infer_expression_type(object).is_some_and(|t| {
                        matches!(&t, Type::Option(inner)
                            if matches!(&**inner, Type::Reference(i) if matches!(&**i, Type::Custom(n) if n == "str") || matches!(&**i, Type::String)))
                    })
                    || Self::receiver_is_text_iterator_next(self, object);
                if recv_is_str_ref {
                    if let Some(stripped) = args[0]
                        .strip_prefix("String::from(")
                        .and_then(|r| r.strip_suffix(')'))
                    {
                        args[0] = stripped.to_string();
                    }
                }
            }
        }
        if method == "unwrap_or" && args.len() == 1 {
            if let Some((
                _,
                Expression::Unary { op: UnaryOp::Ref, operand, .. },
            )) = arguments.first()
            {
                if matches!(&**operand, Expression::Literal { .. }) && !args[0].starts_with('&') {
                    args[0] = format!("&{}", args[0]);
                }
            }
        }
        let inferred_receiver_ty = self.infer_expression_type(object);
        // Direct iteration: `v.map(f)` / `v.filter(p)` on a collection is `v.iter().map(f)` in Rust
        // (`.iter()` is never written in Windjammer). Only names a Vec/slice/map/set does not define.
        if Self::is_direct_iterator_adapter(method)
            && !Self::receiver_text_is_iterator(obj_str)
            && (Self::is_collection_literal_expr(object)
                || inferred_receiver_ty.as_ref().is_some_and(Self::type_is_iterable_collection))
        {
            *obj_str = format!("{}.iter()", *obj_str);
        }
        None
    }

    /// `Vec<string>.contains(x)` with a text argument (`&str` formal, literal, `String`): `Vec<String>::contains`
    /// only accepts `&String`, so compare elements by value: `v.iter().any(|m| *m == *x)`.
    fn mc_lower_string_vec_contains(
        &self,
        object: &Expression<'ast>,
        method: &str,
        arguments: &[(Option<String>, &'ast Expression<'ast>)],
        obj_str: &str,
        args: &[String],
    ) -> Option<String> {
        if method != "contains" || arguments.len() != 1 || args.len() != 1 {
            return None;
        }
        let recv_ty = self.infer_expression_type(object)?;
        if !self.type_is_string_element_vec(&recv_ty) {
            return None;
        }
        let arg_expr = arguments[0].1;
        let arg_is_text = matches!(arg_expr, Expression::Literal { value: crate::parser::Literal::String(_), .. })
            || self.infer_expression_type(arg_expr).is_some_and(|t| {
                crate::codegen::rust::types::is_windjammer_text_type(
                    crate::codegen::rust::collection_detection::peel_type_reference(&t),
                )
            });
        if !arg_is_text {
            return None;
        }
        let mut x = args[0].trim();
        while let Some(rest) = x.strip_prefix('&') {
            x = rest.trim_start();
        }
        let x = x.strip_suffix(".to_string()").unwrap_or(x);
        Some(format!("{obj_str}.iter().any(|m| *m == *{x})"))
    }

    /// `x[a..b]` (desugared to `.slice(a, b)`): bounds are `usize`, so bare non-negative literals and
    /// `as usize` for other integers.
    pub(super) fn mc_slice_expr(
        &self,
        obj_str: &str,
        args: &[String],
        arguments: &[(Option<String>, &'ast Expression<'ast>)],
    ) -> String {
        // Slice bounds are `usize`: bare non-negative literals, `as usize` for other integers.
        let mut bounds = args.to_vec();
        for (i, bound) in bounds.iter_mut().enumerate() {
            if let Some((_, arg_expr)) = arguments.get(i) {
                match arg_expr {
                    Expression::Literal { value: Literal::Int(n), .. } if *n >= 0 => {
                        *bound = n.to_string();
                    }
                    other => self.maybe_cast_index_to_usize(bound, other),
                }
            }
        }
        format!("&{}[{}..{}]", obj_str, bounds[0], bounds[1])
    }

    /// Iterator-only methods (no inherent `Vec`/slice/map/set method of the same name).
    fn is_direct_iterator_adapter(method: &str) -> bool {
        matches!(
            method,
            "map" | "filter" | "filter_map" | "flat_map" | "zip" | "take" | "skip" | "any" | "all"
                | "sum" | "product" | "count" | "fold" | "position" | "find" | "find_map"
                | "max_by_key" | "min_by_key" | "max_by" | "min_by" | "partition" | "unzip"
                | "chain" | "take_while" | "skip_while" | "for_each" | "step_by" | "rev"
                | "cloned" | "copied" | "flatten" | "inspect" | "scan" | "reduce"
        )
    }

    /// `it.next()` where `it` was bound to a `lines()` / `split(..)` / `split_whitespace()` iterator.
    fn receiver_is_text_iterator_next(&self, object: &Expression<'ast>) -> bool {
        let Expression::MethodCall { object: inner, method, .. } = object else {
            return false;
        };
        if method != "next" {
            return false;
        }
        match &**inner {
            Expression::Identifier { name, .. } => self.idiom.text_iterator_locals.contains(name),
            Expression::MethodCall { method: m, .. } => {
                matches!(m.as_str(), "lines" | "split" | "split_whitespace")
            }
            _ => false,
        }
    }

    fn receiver_text_is_iterator(obj_str: &str) -> bool {
        const ITER_SUFFIXES: &[&str] = &[
            ".iter()", ".iter_mut()", ".into_iter()", ".chars()", ".bytes()", ".lines()",
            ".keys()", ".values()", ".drain(..)", ".split_whitespace()",
        ];
        ITER_SUFFIXES.iter().any(|s| obj_str.ends_with(s))
            || (obj_str.ends_with(')')
                && [".map(", ".filter(", ".enumerate(", ".zip(", ".rev(", ".take(", ".skip(",
                    ".chain(", ".cloned(", ".copied(", ".filter_map(", ".flat_map(", ".peekable(",
                    ".take_while(", ".skip_while(", ".step_by(", ".windows(", ".chunks(", ".split("]
                    .iter()
                    .any(|m| obj_str.contains(m)))
    }

    fn is_collection_literal_expr(object: &Expression<'_>) -> bool {
        matches!(object, Expression::Array { .. })
            || matches!(object, Expression::MacroInvocation { name, .. } if name == "vec")
    }

    fn type_is_iterable_collection(ty: &Type) -> bool {
        match ty {
            Type::Vec(_) | Type::Array(..) => true,
            Type::Reference(inner) | Type::MutableReference(inner) => Self::type_is_iterable_collection(inner),
            Type::Parameterized(name, _) | Type::Custom(name) => {
                let leaf = crate::type_classification::type_name_leaf(name);
                matches!(leaf, "Vec" | "VecDeque" | "HashSet" | "BTreeSet" | "HashMap" | "BTreeMap")
            }
            _ => false,
        }
    }

    /// `k` is an owned `String` local (`let k = "x"`), not yet a reference in the emitted Rust: an explicit
    /// `&k` collection-key argument must keep its `&` (`map.get(&k)`).
    pub(super) fn is_owned_text_local(&self, operand: &Expression<'ast>) -> bool {
        let Expression::Identifier { name, .. } = operand else {
            return false;
        };
        let is_rust_ref = self.identifier_already_ref(name)
            || self.emitted_rust_ref_formals.contains(name.as_str())
            || self.str_ref_optimized_params.contains(name.as_str())
            || self.inferred_borrowed_params.contains(name.as_str())
            || self.borrowed_iterator_vars.contains(name);
        !is_rust_ref
            && self
                .local_var_types
                .get(name.as_str())
                .is_some_and(crate::codegen::rust::types::is_windjammer_text_type)
    }
}
