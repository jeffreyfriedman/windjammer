//! ADR-0015 #3: `fn to_string(self) -> string` makes a type printable/interpolatable.
//!
//! For an inherent `impl T` that defines `to_string(self) -> string` (and no explicit `impl Display for T`),
//! the Rust backend emits `impl std::fmt::Display for T` that delegates to it, so `"${t}"` / `println("${t}")`
//! compile exactly like they run in the interpreter and the Go/JS backends.

use crate::parser::{ImplBlock, Item, Type};

fn is_display_trait(name: &str) -> bool {
    matches!(name, "Display" | "fmt::Display" | "std::fmt::Display")
}

fn returns_string(t: &Option<Type>) -> bool {
    matches!(t, Some(Type::String)) || matches!(t, Some(Type::Custom(n)) if n == "string" || n == "String")
}

/// Whether `block` gets a derived `Display` impl.
pub(super) fn wants_display(block: &ImplBlock, items: &[Item]) -> bool {
    if block.trait_name.is_some() || !block.type_params.is_empty() || block.type_name.contains('<') {
        return false;
    }
    let has_to_string = block.functions.iter().any(|f| {
        f.name == "to_string"
            && f.parameters.iter().all(|p| p.name == "self")
            && f.parameters.len() <= 1
            && returns_string(&f.return_type)
    });
    let explicit = items.iter().any(|i| {
        matches!(i, Item::Impl { block: b, .. }
            if b.type_name == block.type_name && b.trait_name.as_deref().is_some_and(is_display_trait))
    });
    has_to_string && !explicit
}

/// Separator after an emitted impl block, plus the derived `Display` impl when [`wants_display`] holds.
pub(super) fn after_impl(block: &ImplBlock, items: &[Item]) -> String {
    if !wants_display(block, items) {
        return "\n\n".to_string();
    }
    let t = &block.type_name;
    format!(
        "\n\nimpl std::fmt::Display for {t} {{\n    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {{\n        f.write_str(&{t}::to_string(self))\n    }}\n}}\n\n"
    )
}
