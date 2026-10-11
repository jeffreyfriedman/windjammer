//! `CompilerConfig` is explicit, plain data: `Default` is production behavior and
//! environment variables are only interpreted by `from_lookup`/`from_env`.

use std::collections::HashMap;
use std::path::PathBuf;
use windjammer::codegen::rust::IrCutoverConfig;
use windjammer::compiler_config::CompilerConfig;

fn lookup(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
    let map: HashMap<String, String> = pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    move |k| map.get(k).cloned()
}

#[test]
fn default_is_production_all_cutover_on() {
    let config = CompilerConfig::default();
    assert!(config.ir_cutover.all_enabled());
    assert!(!config.ir_coercion_engine);
    assert!(!config.legacy_multipass);
    assert!(!config.emit_aosoa_hints);
    assert!(config.stdlib_path.is_none());
    assert!(config.runtime_path.is_none());
    assert!(config.game_path.is_none());
    assert!(config.cache_locality_json.is_none());
}

#[test]
fn ir_cutover_default_equals_production_and_legacy_is_explicit() {
    assert!(IrCutoverConfig::default().all_enabled());
    assert!(IrCutoverConfig::legacy_all_off().all_disabled());
}

#[test]
fn empty_environment_equals_default() {
    let config = CompilerConfig::from_lookup(lookup(&[]));
    assert_eq!(config, CompilerConfig::default());
}

#[test]
fn from_lookup_reads_every_documented_toggle() {
    let config = CompilerConfig::from_lookup(lookup(&[
        ("WJ_IR_CUTOVER_DISABLE_OWNERSHIP", "1"),
        ("WJ_IR_CUTOVER_DISABLE_LOCALS", "1"),
        ("WJ_IR_COERCION_ENGINE", "1"),
        ("WJ_LEGACY_MULTIPASS", "true"),
        ("WJ_EMIT_AOSOA_HINTS", "1"),
        ("WINDJAMMER_STDLIB", "/s"),
        ("WINDJAMMER_RUNTIME_PATH", "/r"),
        ("game-host_PATH", "/g"),
        ("WJ_CACHE_LOCALITY_JSON", "/c.json"),
    ]));
    assert!(!config.ir_cutover.ownership);
    assert!(!config.ir_cutover.locals);
    assert!(config.ir_cutover.clones && config.ir_cutover.call_sites);
    assert!(config.ir_coercion_engine);
    assert!(config.legacy_multipass);
    assert!(config.emit_aosoa_hints);
    assert_eq!(config.stdlib_path, Some(PathBuf::from("/s")));
    assert_eq!(config.runtime_path, Some(PathBuf::from("/r")));
    assert_eq!(config.game_path, Some(PathBuf::from("/g")));
    assert_eq!(config.cache_locality_json, Some(PathBuf::from("/c.json")));
}

#[test]
fn builders_override_fields() {
    let config = CompilerConfig::default()
        .with_ir_cutover(IrCutoverConfig::legacy_all_off())
        .with_stdlib_path("/std")
        .with_runtime_path("/rt")
        .with_emit_aosoa_hints(true);
    assert!(config.ir_cutover.all_disabled());
    assert_eq!(config.stdlib_path, Some(PathBuf::from("/std")));
    assert_eq!(config.runtime_path, Some(PathBuf::from("/rt")));
    assert!(config.emit_aosoa_hints);
}

#[test]
fn cli_flag_fields_default_off_and_have_builders() {
    let config = CompilerConfig::default();
    assert!(!config.shadow_validate_strict);
    assert!(!config.skip_cargo_toml_generation);
    let config = config
        .with_shadow_validate_strict(true)
        .with_skip_cargo_toml_generation(true);
    assert!(config.shadow_validate_strict && config.skip_cargo_toml_generation);
}
