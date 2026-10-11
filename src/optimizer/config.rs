//! Switches for the post-codegen optimizer. Each pass is independently
//! switchable; defaults are ON only for passes whose soundness argument is
//! written down in `docs/optimization/PASSES.md`.

use super::mutants::Mutant;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptimizationConfig {
    /// Drop `.clone()` on freshly built owned temporaries (`vec![..].clone()`).
    pub temp_clone: bool,
    /// Drop `.clone()` on bindings whose every declaration is a primitive Copy type.
    pub copy_clone: bool,
    /// Drop `.clone()` on direct arguments of format-family macros.
    pub macro_arg_clone: bool,
    /// Drop `e as T as T` and `x as T` when `x` is a binding of type `T`.
    pub same_type_cast: bool,
    /// `Vec::new()` -> `Vec::with_capacity(n)` before an unconditional `0..n` push loop.
    pub with_capacity: bool,
    /// `s = format!("{}..", s, ..)` -> in-place append on a `String` binding.
    pub string_append: bool,
    /// Remove `#[inline]` from tests, `main` and large functions.
    pub inline_policy: bool,
    /// Remove file-private `#[repr(C)]` and unreferenced `to_bytes` helpers.
    pub dead_helpers: bool,
    /// Auto-arena: hoist a per-iteration `String::new()`/`Vec::new()` scratch buffer out of its loop and
    /// `clear()` it instead (PASSES.md P8). OFF by default; `wj build --auto-arena` turns it on.
    pub scratch_reuse: bool,
    /// Byte size of a function body above which `#[inline]` is removed.
    pub inline_max_body_bytes: usize,
    /// Test-only: run a deliberately unsound variant of one pass.
    #[doc(hidden)]
    pub mutant: Option<Mutant>,
}

impl OptimizationConfig {
    /// Production defaults.
    pub fn production() -> Self {
        Self {
            temp_clone: true,
            copy_clone: true,
            macro_arg_clone: true,
            same_type_cast: true,
            with_capacity: true,
            string_append: true,
            inline_policy: true,
            dead_helpers: true,
            scratch_reuse: false,
            inline_max_body_bytes: 600,
            mutant: None,
        }
    }

    /// Every pass off: `run` returns its input unchanged.
    pub fn disabled() -> Self {
        Self {
            temp_clone: false,
            copy_clone: false,
            macro_arg_clone: false,
            same_type_cast: false,
            with_capacity: false,
            string_append: false,
            inline_policy: false,
            dead_helpers: false,
            scratch_reuse: false,
            ..Self::production()
        }
    }
}

impl OptimizationConfig {
    /// Production defaults plus the opt-in auto-arena pass.
    pub fn with_auto_arena() -> Self {
        Self { scratch_reuse: true, ..Self::production() }
    }
}

impl Default for OptimizationConfig {
    fn default() -> Self {
        Self::production()
    }
}
