//! `wj build --target wgsl` on `.wjsl` files: text includes (S0) and `use crate::m::item` modules (S1).
//! Design: docs/design/wjsl-modules.md.

use std::fs;
use std::path::Path;
use std::process::Command;

/// Run `wj build FILE --target wgsl` and return (success, wgsl-or-stderr).
fn build_wjsl(entry: &Path, out_dir: &Path) -> (bool, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args(["build"])
        .arg(entry)
        .args(["--target", "wgsl", "--no-cargo", "--output"])
        .arg(out_dir)
        .output()
        .expect("run wj");
    if !output.status.success() {
        return (false, String::from_utf8_lossy(&output.stderr).to_string());
    }
    let stem = entry.file_stem().unwrap().to_string_lossy().to_string();
    (true, fs::read_to_string(out_dir.join(format!("{stem}.wgsl"))).expect("wgsl output"))
}

#[test]
fn s0_build_resolves_text_include() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("x.wjsl"), "fn helper(a: f32) -> f32 {\n    return a * 2.0;\n}\n").unwrap();
    fs::write(
        dir.path().join("main.wjsl"),
        "use \"x.wjsl\"\n@fragment\nfn fs_main() -> @location(0) vec4<f32> {\n    return vec4<f32>(helper(1.0), 0.0, 0.0, 1.0);\n}\n",
    )
    .unwrap();
    let (ok, out) = build_wjsl(&dir.path().join("main.wjsl"), &dir.path().join("out"));
    assert!(ok, "build failed: {out}");
    assert!(out.contains("fn helper"), "{out}");
}

// ---------------------------------------------------------------------------
// S1: `use crate::module::item` between .wjsl files of one folder
// ---------------------------------------------------------------------------

const BRDF: &str = "\
fn pow5(x: f32) -> f32 {
    return x * x * x * x * x;
}

pub const MIN_F0: f32 = 0.02;

pub struct Lobe {
    weight: f32,
}

pub fn fresnel_schlick(cos_theta: f32, f0: f32) -> f32 {
    return f0 + (1.0 - f0) * pow5(1.0 - cos_theta);
}

pub fn lobe_weight(l: Lobe) -> f32 {
    return l.weight;
}

fn unused_helper() -> f32 {
    return 0.0;
}
";

const SKIN: &str = "\
use crate::brdf::fresnel_schlick
use crate::brdf::{lobe_weight, Lobe}

@group(0) @binding(0) uniform lobe: Lobe;

@fragment
fn fs_skin(@location(0) n: vec3<f32>, @location(1) v: vec3<f32>) -> @location(0) vec4<f32> {
    let f = fresnel_schlick(clamp(dot(n, v), 0.0, 1.0), 0.028);
    return vec4<f32>(vec3<f32>(f * lobe_weight(lobe)), 1.0);
}
";

fn tree(files: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    for (name, text) in files {
        fs::write(dir.path().join(name), text).unwrap();
    }
    dir
}

fn emit(dir: &Path, entry: &str) -> Result<String, String> {
    windjammer::wjsl::transpile_wjsl_file(&dir.join(entry)).map_err(|e| e.to_string())
}

/// Parse and fully validate WGSL with naga (test-only dependency).
fn naga_validate(wgsl: &str) {
    let module = naga::front::wgsl::parse_str(wgsl).unwrap_or_else(|e| panic!("naga parse: {e:?}\n{wgsl}"));
    naga::valid::Validator::new(naga::valid::ValidationFlags::all(), naga::valid::Capabilities::all())
        .validate(&module)
        .unwrap_or_else(|e| panic!("naga validate: {e:?}\n{wgsl}"));
}

#[test]
fn s1_same_folder_use_emits_one_module_matching_golden() {
    let dir = tree(&[("brdf.wjsl", BRDF), ("skin.wjsl", SKIN)]);
    let out = emit(dir.path(), "skin.wjsl").unwrap();
    assert_eq!(out.matches("fn brdf__fresnel_schlick").count(), 1, "{out}");
    assert!(out.contains("fn brdf__pow5"), "transitively reached private helper: {out}");
    assert!(!out.contains("unused_helper"), "dead code must be eliminated: {out}");
    assert!(out.contains("// from brdf.wjsl:11"), "origin comment: {out}");
    naga_validate(&out);
    let golden = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/wgsl/wjsl_modules_skin.wgsl");
    if std::env::var("UPDATE_GOLDEN").is_ok() {
        fs::write(&golden, &out).unwrap();
    }
    assert_eq!(fs::read_to_string(&golden).unwrap(), out);
}

#[test]
fn s1_names_are_stable_when_unrelated_files_are_added_or_removed() {
    let base = tree(&[("brdf.wjsl", BRDF), ("skin.wjsl", SKIN)]);
    let more = tree(&[
        ("brdf.wjsl", BRDF),
        ("skin.wjsl", SKIN),
        ("other.wjsl", "pub fn fresnel_schlick(a: f32) -> f32 {\n    return a;\n}\n"),
        ("zzz.wjsl", "pub fn lobe_weight(a: f32) -> f32 {\n    return a;\n}\n"),
    ]);
    assert_eq!(emit(base.path(), "skin.wjsl").unwrap(), emit(more.path(), "skin.wjsl").unwrap());
}

#[test]
fn s1_qualified_use_equals_direct_import() {
    let direct = "use crate::brdf::fresnel_schlick\n@fragment\nfn fs() -> @location(0) vec4<f32> {\n    return vec4<f32>(fresnel_schlick(0.5, 0.04));\n}\n";
    let qualified = "use crate::brdf\n@fragment\nfn fs() -> @location(0) vec4<f32> {\n    return vec4<f32>(brdf::fresnel_schlick(0.5, 0.04));\n}\n";
    let a = tree(&[("brdf.wjsl", BRDF), ("e.wjsl", direct)]);
    let b = tree(&[("brdf.wjsl", BRDF), ("e.wjsl", qualified)]);
    let (oa, ob) = (emit(a.path(), "e.wjsl").unwrap(), emit(b.path(), "e.wjsl").unwrap());
    assert_eq!(oa, ob);
    naga_validate(&oa);
}

#[test]
fn s1_same_name_in_two_modules_gets_distinct_names() {
    let a = "pub fn pow5(x: f32) -> f32 {\n    return x * x * x * x * x;\n}\n";
    let b = "pub fn pow5(x: f32) -> f32 {\n    return x * x * x * x * x * 2.0;\n}\n";
    let e = "use crate::a\nuse crate::b\n@fragment\nfn fs() -> @location(0) vec4<f32> {\n    return vec4<f32>(a::pow5(0.5) + b::pow5(0.5));\n}\n";
    let d = tree(&[("a.wjsl", a), ("b.wjsl", b), ("e.wjsl", e)]);
    let out = emit(d.path(), "e.wjsl").unwrap();
    assert!(out.contains("fn a__pow5") && out.contains("fn b__pow5"), "{out}");
    naga_validate(&out);
}

#[test]
fn s1_private_item_import_is_rejected_at_the_use_site_with_the_pub_fix() {
    let d = tree(&[("brdf.wjsl", BRDF), ("e.wjsl", "use crate::brdf::unused_helper\n@fragment\nfn fs() -> @location(0) vec4<f32> {\n    return vec4<f32>(1.0);\n}\n")]);
    let err = emit(d.path(), "e.wjsl").unwrap_err();
    assert!(err.contains("e.wjsl:1:18: error[WJ0198]: `unused_helper` is private to `brdf`"), "{err}");
    assert!(err.contains("pub fn unused_helper"), "{err}");
}

#[test]
fn s1_missing_module_item_cycle_reserved_name_and_library_binding_are_rejected() {
    let frag = "@fragment\nfn fs() -> @location(0) vec4<f32> {\n    return vec4<f32>(1.0);\n}\n";
    // missing module
    let d = tree(&[("e.wjsl", &format!("use crate::nope::x\n{frag}"))]);
    let err = emit(d.path(), "e.wjsl").unwrap_err();
    assert!(err.contains("error[WJ0197]: no module `nope` in `crate` (looked for `nope.wjsl` next to `e.wjsl`)"), "{err}");
    // missing item
    let d = tree(&[("brdf.wjsl", BRDF), ("e.wjsl", &format!("use crate::brdf::nothing\n{frag}"))]);
    assert!(emit(d.path(), "e.wjsl").unwrap_err().contains("error[WJ0197]: no function, struct or const `nothing`"));
    // cycle
    let d = tree(&[
        ("a.wjsl", "use crate::b::g\npub fn f() -> f32 {\n    return g();\n}\n"),
        ("b.wjsl", "use crate::a::f\npub fn g() -> f32 {\n    return f();\n}\n"),
        ("e.wjsl", &format!("use crate::a::f\n{frag}")),
    ]);
    let err = emit(d.path(), "e.wjsl").unwrap_err();
    assert!(err.contains("error[WJ0199]: import cycle: a.wjsl -> b.wjsl -> a.wjsl"), "{err}");
    // `__` in a user identifier
    let d = tree(&[("e.wjsl", "fn bad__name() -> f32 {\n    return 1.0;\n}\n")]);
    let err = emit(d.path(), "e.wjsl").unwrap_err();
    assert!(err.contains("e.wjsl:1:4: error[WJ0201]: `bad__name` contains `__`"), "{err}");
    // binding and entry point in a library module
    let d = tree(&[
        ("lib.wjsl", "@group(0) @binding(0) uniform u: f32;\npub fn f() -> f32 {\n    return 1.0;\n}\n"),
        ("e.wjsl", &format!("use crate::lib::f\n{frag}")),
    ]);
    let err = emit(d.path(), "e.wjsl").unwrap_err();
    assert!(err.contains("lib.wjsl:1:1: error[WJ0200]: a binding is not allowed in library module `lib`"), "{err}");
}

#[test]
fn s1_output_is_location_agnostic() {
    let a = tree(&[("brdf.wjsl", BRDF), ("skin.wjsl", SKIN)]);
    let outer = tempfile::tempdir().unwrap();
    let nested = outer.path().join("some").join("deeper").join("shaders_elsewhere");
    fs::create_dir_all(&nested).unwrap();
    fs::write(nested.join("brdf.wjsl"), BRDF).unwrap();
    fs::write(nested.join("skin.wjsl"), SKIN).unwrap();
    assert_eq!(emit(a.path(), "skin.wjsl").unwrap(), emit(&nested, "skin.wjsl").unwrap());
}

#[test]
fn s1_build_cli_writes_one_module_and_validate_wjsl_accepts_library_folders() {
    let d = tree(&[("brdf.wjsl", BRDF), ("skin.wjsl", SKIN)]);
    let (ok, out) = build_wjsl(&d.path().join("skin.wjsl"), &d.path().join("out"));
    assert!(ok, "{out}");
    assert!(out.contains("brdf__fresnel_schlick"));
    let lib_only = tree(&[("brdf.wjsl", BRDF)]);
    let status = Command::new(env!("CARGO_BIN_EXE_wj")).arg("validate-wjsl").arg(lib_only.path()).output().unwrap();
    assert!(status.status.success(), "{}", String::from_utf8_lossy(&status.stderr));
    let both = Command::new(env!("CARGO_BIN_EXE_wj")).arg("validate-wjsl").arg(d.path()).output().unwrap();
    assert!(both.status.success(), "{}", String::from_utf8_lossy(&both.stderr));
}

#[test]
fn s1_module_qualified_names_are_user_functions_for_the_undefined_function_check() {
    // WJ0195 (undefined function) must accept `brdf__fresnel_schlick` and still reject a truly undefined call.
    let d = tree(&[("brdf.wjsl", BRDF), ("skin.wjsl", SKIN)]);
    assert!(emit(d.path(), "skin.wjsl").is_ok());
    let bad = SKIN.replace("fresnel_schlick(clamp", "fresnel_schlik(clamp");
    let d = tree(&[("brdf.wjsl", BRDF), ("skin.wjsl", &bad)]);
    let err = emit(d.path(), "skin.wjsl").unwrap_err();
    assert!(err.contains("WJ0195") || err.contains("fresnel_schlik"), "{err}");
}

#[test]
fn s1_import_and_mod_keywords_get_a_message_that_says_what_to_write() {
    let frag = "@fragment\nfn fs() -> @location(0) vec4<f32> {\n    return vec4<f32>(1.0);\n}\n";
    let d = tree(&[("e.wjsl", &format!("import \"../lib/brdf.wjsl\";\n{frag}"))]);
    assert_eq!(
        emit(d.path(), "e.wjsl").unwrap_err(),
        "e.wjsl:1:1: error[WJ0197]: WJSL has no `import`: write `use crate::module::item` (module import, same folder) or `use \"path\"` (text include)\n  fix: replace the line with a `use` import"
    );
    let d = tree(&[("e.wjsl", &format!("// c\nmod brdf;\n{frag}"))]);
    assert_eq!(
        emit(d.path(), "e.wjsl").unwrap_err(),
        "e.wjsl:2:1: error[WJ0197]: WJSL has no `mod`: a module is a file; write `use crate::module::item` to import from `module.wjsl` in the same folder\n  fix: replace the line with a `use` import"
    );
}
