//! Integration tests for IrCutoverConfig flags and IR pipeline integration.

use windjammer::codegen::rust::IrCutoverConfig;

#[test]
fn cutover_default_all_off() {
    let config = IrCutoverConfig::default();
    assert!(!config.all_enabled());
    assert!(!config.call_sites);
    assert!(!config.locals);
}

#[test]
fn cutover_formal_flags_on_call_sites_on_by_default() {
    let config = IrCutoverConfig::from_env();
    assert!(config.ownership);
    assert!(config.clones);
    assert!(config.param_types);
    assert!(config.str_ref);
    assert!(config.call_sites);
    assert!(config.locals);
}

#[test]
fn cutover_all_flags_enabled() {
    let config = IrCutoverConfig {
        ownership: true,
        clones: true,
        param_types: true,
        str_ref: true,
        call_sites: true,
        locals: true,
    };
    assert!(config.all_enabled());
    assert!(!config.all_disabled());
}

#[test]
fn cutover_all_disabled() {
    let config = IrCutoverConfig::default();
    assert!(config.all_disabled());
}
