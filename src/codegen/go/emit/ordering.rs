//! `Ordering` (runtime `type Ordering int`, -1 / 0 / 1) and the comparison family for Go.

use super::Go;
use crate::codegen::portable::ir::*;
use crate::codegen::portable::ordering;

impl<'m> Go<'m> {
    fn user_ordering(&self) -> bool {
        self.m.enums.iter().any(|e| e.name == ordering::NAME)
    }

    /// `Ordering::Less` etc.
    pub fn ordering_const(&self, owner: &str, name: &str) -> Option<String> {
        if !ordering::is_path(owner) || self.user_ordering() {
            return None;
        }
        ordering::variant_value(name).map(|v| format!("Ordering({v})"))
    }

    /// Comparison-family methods; `None` when `name` is not one for this receiver.
    pub fn ordering_method(&mut self, recv: &PExpr, name: &str, r: &str, a: &[String]) -> Option<String> {
        if ordering::is_ordering(&recv.ty) && !self.user_ordering() {
            return Some(match name {
                "reverse" => format!("(-{r})"),
                "then" => format!("wjThen({r}, {})", a[0]),
                "then_with" => format!("wjThenWith({r}, {})", a[0]),
                "is_lt" => format!("({r} < 0)"),
                "is_le" => format!("({r} <= 0)"),
                "is_gt" => format!("({r} > 0)"),
                "is_ge" => format!("({r} >= 0)"),
                "is_eq" => format!("({r} == 0)"),
                "is_ne" => format!("({r} != 0)"),
                _ => return None,
            });
        }
        Some(match name {
            "cmp" => format!("wjCmp({r}, {})", a[0]),
            "partial_cmp" => format!("wjPartialCmp({r}, {})", a[0]),
            "sort_by" => format!("wjSortBy({r}, {})", a[0]),
            "sort_by_key" => format!("wjSortByKey({r}, {})", a[0]),
            "max_by" => format!("wjMinMaxBy({r}, {}, 1)", a[0]),
            "min_by" => format!("wjMinMaxBy({r}, {}, -1)", a[0]),
            "max_by_key" => format!("wjMinMaxByKey({r}, {}, 1)", a[0]),
            "min_by_key" => format!("wjMinMaxByKey({r}, {}, -1)", a[0]),
            _ => return None,
        })
    }
}
