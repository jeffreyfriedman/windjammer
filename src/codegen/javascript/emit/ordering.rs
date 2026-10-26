//! `Ordering` (-1 / 0 / 1 Numbers) and the comparison family for JavaScript.

use super::Js;
use crate::codegen::portable::ir::*;
use crate::codegen::portable::ordering;
use crate::codegen::portable::ty::Ty;

impl<'m> Js<'m> {
    /// The built-in `Ordering` is in use (the program does not declare its own).
    pub fn builtin_ordering(&self, t: &Ty) -> bool {
        ordering::is_ordering(t) && !self.m.enums.iter().any(|e| e.name == ordering::NAME)
    }

    /// `Ordering::Less` etc.
    pub fn ordering_const(&self, owner: &str, name: &str) -> Option<String> {
        if !ordering::is_path(owner) || self.m.enums.iter().any(|e| e.name == ordering::NAME) {
            return None;
        }
        ordering::variant_value(name).map(|v| v.to_string())
    }

    /// Comparison-family methods; `None` when `name` is not one for this receiver.
    pub fn ordering_method(&mut self, recv: &PExpr, name: &str, r: &str, a: &[String]) -> Option<String> {
        if self.builtin_ordering(&recv.ty) {
            return Some(match name {
                "reverse" => format!("(0 - {r})"),
                "then" => format!("({r} !== 0 ? {r} : {})", a[0]),
                "then_with" => format!("({r} !== 0 ? {r} : ({})())", a[0]),
                "is_lt" => format!("({r} < 0)"),
                "is_le" => format!("({r} <= 0)"),
                "is_gt" => format!("({r} > 0)"),
                "is_ge" => format!("({r} >= 0)"),
                "is_eq" => format!("({r} === 0)"),
                "is_ne" => format!("({r} !== 0)"),
                _ => return None,
            });
        }
        Some(match name {
            "cmp" => format!("$cmp({r}, {})", a[0]),
            "partial_cmp" => format!("$pcmp({r}, {})", a[0]),
            "sort_by" => format!("$sortBy({r}, {})", a[0]),
            "sort_by_key" => format!("$sortBy({r}, (x, y) => $cmp(({0})(x), ({0})(y)))", a[0]),
            "max_by" => format!("$minmaxBy({r}, {}, 1)", a[0]),
            "min_by" => format!("$minmaxBy({r}, {}, -1)", a[0]),
            "max_by_key" => format!("$minmaxBy({r}, (x, y) => $cmp(({0})(x), ({0})(y)), 1)", a[0]),
            "min_by_key" => format!("$minmaxBy({r}, (x, y) => $cmp(({0})(x), ({0})(y)), -1)", a[0]),
            _ => return None,
        })
    }
}
