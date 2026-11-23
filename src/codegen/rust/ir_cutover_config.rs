//! `IrCutoverConfig`: which categories of codegen decisions read the IR `SafetyType` instead of the legacy analyzer
//! fields (moved out of `generator.rs`, which is over its length allowance; `generator` re-exports it).

/// Configuration for incremental IR cutover.
/// Each flag controls whether a specific category of codegen decisions reads from
/// the IR `SafetyType` (true) or the legacy `AnalyzedFunction` fields (false).
/// `Default` is production behavior (all on); use [`IrCutoverConfig::legacy_all_off`]
/// to opt into the legacy pipeline explicitly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrCutoverConfig {
    /// Read parameter ownership from `IrFunction.param_types[name].ownership`
    pub ownership: bool,
    /// Read clone requirements from `IrFunction.optimizations.clone_annotations`
    pub clones: bool,
    /// Read parameter types from `IrFunction.param_types[name].base`
    pub param_types: bool,
    /// Read str_ref optimization from `IrFunction.str_ref_params`
    pub str_ref: bool,
    /// Use IR SafetyType for call-site argument coercions (via `ir::coercion`)
    pub call_sites: bool,
    /// Read local variable types from solver-resolved IR
    pub locals: bool,
}

impl Default for IrCutoverConfig {
    fn default() -> Self {
        Self::production()
    }
}

impl IrCutoverConfig {
    /// Production behavior: every cutover flag on.
    pub fn production() -> Self {
        Self {
            ownership: true,
            clones: true,
            param_types: true,
            str_ref: true,
            call_sites: true,
            locals: true,
        }
    }

    /// Every flag off: the legacy analyzer-driven path (isolated unit tests only).
    pub fn legacy_all_off() -> Self {
        Self {
            ownership: false,
            clones: false,
            param_types: false,
            str_ref: false,
            call_sites: false,
            locals: false,
        }
    }

    pub fn all_enabled(&self) -> bool {
        self.ownership
            && self.clones
            && self.param_types
            && self.str_ref
            && self.call_sites
            && self.locals
    }

    pub fn all_disabled(&self) -> bool {
        !self.ownership
            && !self.clones
            && !self.param_types
            && !self.str_ref
            && !self.call_sites
            && !self.locals
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ir_cutover_config_legacy_all_off_is_explicit() {
        let config = IrCutoverConfig::legacy_all_off();
        assert!(!config.ownership);
        assert!(!config.clones);
        assert!(!config.param_types);
        assert!(!config.str_ref);
        assert!(!config.call_sites);
        assert!(!config.locals);
        assert!(!config.all_enabled());
    }

    #[test]
    fn test_ir_cutover_default_is_production() {
        let config = IrCutoverConfig::default();
        assert!(config.ownership);
        assert!(config.clones);
        assert!(config.param_types);
        assert!(config.str_ref);
        assert!(config.call_sites);
        assert!(config.locals);
        assert!(config.all_enabled());
    }

    #[test]
    fn test_ir_cutover_config_all_enabled() {
        let config = IrCutoverConfig {
            ownership: true,
            clones: true,
            param_types: true,
            str_ref: true,
            call_sites: true,
            locals: true,
        };
        assert!(config.all_enabled());
    }

    #[test]
    fn test_ir_cutover_config_partial_not_all() {
        let config = IrCutoverConfig {
            ownership: true,
            clones: false,
            param_types: true,
            str_ref: true,
            call_sites: false,
            locals: false,
        };
        assert!(!config.all_enabled());
    }
}
