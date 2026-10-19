//! ADR-0018 feature 1: the enumerable surface is declared once (`src/stdlib_surface/enumerable.rs`); the reader-facing
//! signatures in `std/enumerable.wj` and the spec chapter must name exactly the same methods.

use windjammer::stdlib_surface::enumerable::METHODS;

#[test]
fn std_enumerable_declares_every_surface_method_and_nothing_else() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let src = std::fs::read_to_string(root.join("std/enumerable.wj")).unwrap();
    let declared: std::collections::BTreeSet<String> = src
        .lines()
        .filter_map(|l| l.trim().strip_prefix("fn "))
        .map(|r| r.split(['(', '<']).next().unwrap().to_string())
        .collect();
    let table: std::collections::BTreeSet<String> =
        METHODS.iter().map(|m| m.name.to_string()).collect();
    assert_eq!(declared, table);
}

#[test]
fn spec_chapter_mentions_every_surface_method() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let doc = std::fs::read_to_string(root.join("docs/spec/collections.md")).unwrap();
    for m in METHODS {
        assert!(
            doc.contains(&format!("`xs.{}(", m.name))
                || doc.contains(&format!("`{}`", m.name))
                || doc.contains(&format!("xs.{}()", m.name)),
            "docs/spec/collections.md does not describe `{}`",
            m.name
        );
    }
}
