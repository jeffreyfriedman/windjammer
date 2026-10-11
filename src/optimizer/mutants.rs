//! Deliberately unsound pass variants. They exist only so the test-suite can
//! prove it detects each class of unsoundness (mutation self-test). They are
//! never enabled by `OptimizationConfig::production()`.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mutant {
    /// copy_clone: ignore the "every declaration agrees" rule (shadowing/patterns).
    CopyCloneIgnoresShadowing,
    /// temp_clone: drop `.clone()` on any identifier receiver.
    TempCloneOnAnyIdent,
    /// macro_arg_clone: also strip clones inside `vec![..]` (moves the original).
    MacroCloneInVec,
    /// same_type_cast: drop `as T` on any identifier receiver.
    CastDropsAnyIdent,
    /// with_capacity: accept loops with `break`.
    CapacityIgnoresBreak,
    /// with_capacity: accept a start bound that is not literal zero.
    CapacityAnyStart,
    /// string_append: allow the appended arguments to mention the target.
    AppendIgnoresSelfReference,
    /// dead_helpers: remove `to_bytes` even when referenced.
    HelpersIgnoreReferences,
    /// dead_helpers: remove repr(C) from structs in files using `unsafe`.
    ReprIgnoresUnsafe,
}
