//! `@json(...)` options (ADR-0027), read by the Rust backend and the interpreter alike so both engines
//! decode the same wire format.
//!
//! - struct: `@json(allow_unknown)` accepts fields the struct does not declare (decoding denies them by default)
//! - field:  `@json(rename = "userName")` names the field on the wire

use super::core::{Decorator, Expression};
use super::literals::Literal;

/// `@json(allow_unknown)` on a struct.
pub fn json_allows_unknown(decorators: &[Decorator<'_>]) -> bool {
    json_args(decorators).any(|(k, e)| k == "allow_unknown" || matches!(e, Expression::Identifier { name, .. } if name == "allow_unknown"))
}

/// `@json(rename = "wire_name")` on a field.
pub fn json_rename(decorators: &[Decorator<'_>]) -> Option<String> {
    json_args(decorators).find_map(|(k, e)| match e {
        Expression::Literal { value: Literal::String(s), .. } if k == "rename" => Some(s.clone()),
        _ => None,
    })
}

fn json_args<'a>(decorators: &'a [Decorator<'_>]) -> impl Iterator<Item = (&'a str, &'a Expression<'a>)> + 'a {
    decorators.iter().filter(|d| d.name == "json").flat_map(|d| d.arguments.iter().map(|(k, e)| (k.as_str(), &**e)))
}
