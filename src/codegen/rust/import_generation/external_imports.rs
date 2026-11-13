//! Heuristics for internal vs external (Cargo) crate paths when emitting `use`.

/// Known external/crate module prefixes that should NOT get ::* added
pub(in crate::codegen::rust) const EXTERNAL_CRATE_MODULES: &[&str] =
    &["engine", "windjammer_runtime", "smallvec", "serde", "std"];

pub(in crate::codegen::rust) fn is_external_crate(first_segment: &str) -> bool {
    EXTERNAL_CRATE_MODULES.contains(&first_segment)
}
