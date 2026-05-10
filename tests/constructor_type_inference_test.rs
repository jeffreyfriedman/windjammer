use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn unique_dir() -> std::path::PathBuf {
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let pid = std::process::id();
    std::env::temp_dir().join(format!("wj_ctor_infer_{pid}_{id}"))
}

fn wj_bin() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/release/wj")
}

/// When metadata has return_type: null for Type::new(), the compiler should
/// infer that the return type is Type (constructor convention). This ensures
/// that subsequent method calls on the variable get proper type resolution
/// for string literal conversion.
#[test]
fn test_constructor_return_type_inference_with_null_metadata() {
    let dir = unique_dir();
    let src_dir = dir.join("src");
    std::fs::create_dir_all(&src_dir).unwrap();

    std::fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();

    let metadata = r#"{
        "structs": {
            "SystemCoverage": {
                "name": "String"
            }
        },
        "functions": {
            "SystemCoverage::new": {
                "params": ["Custom(\"String\")"],
                "return_type": null,
                "is_associated": false,
                "parent_type": null,
                "param_ownership": ["Owned"],
                "has_self_receiver": false,
                "is_extern": false
            },
            "SystemCoverage::register_function": {
                "params": ["Custom(\"Self\")", "Custom(\"String\")"],
                "return_type": "Custom(\"usize\")",
                "is_associated": true,
                "parent_type": "SystemCoverage",
                "param_ownership": ["MutBorrowed", "Owned"],
                "has_self_receiver": true,
                "is_extern": false
            }
        },
        "version": "0.46.2"
    }"#;

    let engine_dir = dir.join("engine");
    std::fs::create_dir_all(&engine_dir).unwrap();
    let meta_file = engine_dir.join("metadata.json");
    std::fs::write(&meta_file, metadata).unwrap();

    let source = r#"
use engine::testing::coverage::SystemCoverage

pub fn validate_combat() {
    let mut sys = SystemCoverage::new("combat")
    let fire = sys.register_function("fire")
    let reload = sys.register_function("reload")
}
"#;
    std::fs::write(src_dir.join("test.wj"), source).unwrap();

    let output_dir = dir.join("output");
    std::fs::create_dir_all(&output_dir).unwrap();

    let output = Command::new(wj_bin())
        .arg("build")
        .arg(src_dir.to_str().unwrap())
        .arg("--output")
        .arg(output_dir.to_str().unwrap())
        .arg("--library")
        .arg("--no-cargo")
        .arg("--metadata")
        .arg(format!("engine={}", meta_file.display()))
        .current_dir(&dir)
        .output()
        .expect("Failed to run wj");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    eprintln!("STDERR: {}", stderr);
    eprintln!("STDOUT: {}", stdout);

    let rs_file = output_dir.join("test.rs");
    let generated = if rs_file.exists() {
        std::fs::read_to_string(&rs_file).unwrap_or_default()
    } else {
        panic!(
            "Generated .rs file not found at {:?}\nFiles in output: {:?}",
            rs_file,
            std::fs::read_dir(&output_dir)
                .map(|d| d.filter_map(|e| e.ok().map(|e| e.path()))
                    .collect::<Vec<_>>())
                .unwrap_or_default()
        );
    };

    eprintln!("GENERATED:\n{}", generated);

    assert!(
        generated.contains(r#""fire".to_string()"#),
        "String literal \"fire\" should be converted to .to_string() even when constructor metadata has return_type: null.\nGenerated:\n{}",
        generated
    );

    assert!(
        generated.contains(r#""reload".to_string()"#),
        "String literal \"reload\" should also be converted.\nGenerated:\n{}",
        generated
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// Test the real-world scenario: a long function with many statements before
/// the SystemCoverage usage, compiled as a library with multiple files.
/// This reproduces the actual breach-protocol/playtest_validation.wj pattern
/// where type inference fails for local variables deep in long functions.
#[test]
fn test_string_coercion_deep_in_long_function_multifile_library() {
    let dir = unique_dir();
    let src_dir = dir.join("src");
    std::fs::create_dir_all(&src_dir).unwrap();

    std::fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();

    let metadata = r#"{
        "structs": {
            "SystemCoverage": {
                "name": "String"
            },
            "InputRecorder": {},
            "GoldenBaseline": {}
        },
        "functions": {
            "SystemCoverage::new": {
                "params": ["Custom(\"String\")"],
                "return_type": null,
                "is_associated": false,
                "parent_type": null,
                "param_ownership": ["Owned"],
                "has_self_receiver": false,
                "is_extern": false
            },
            "SystemCoverage::register_function": {
                "params": ["Custom(\"Self\")", "Custom(\"String\")"],
                "return_type": "Custom(\"usize\")",
                "is_associated": true,
                "parent_type": "SystemCoverage",
                "param_ownership": ["MutBorrowed", "Owned"],
                "has_self_receiver": true,
                "is_extern": false
            },
            "InputRecorder::new": {
                "params": [],
                "return_type": null,
                "is_associated": false,
                "parent_type": null,
                "param_ownership": [],
                "has_self_receiver": false,
                "is_extern": false
            },
            "InputRecorder::start": {
                "params": ["Custom(\"Self\")"],
                "return_type": null,
                "is_associated": true,
                "parent_type": "InputRecorder",
                "param_ownership": ["MutBorrowed"],
                "has_self_receiver": true,
                "is_extern": false
            },
            "GoldenBaseline::from_pixels": {
                "params": ["Custom(\"String\")", "Custom(\"Vec<f32>\")"],
                "return_type": "Custom(\"GoldenBaseline\")",
                "is_associated": false,
                "parent_type": null,
                "param_ownership": ["Owned", "Owned"],
                "has_self_receiver": false,
                "is_extern": false
            }
        },
        "version": "0.46.2"
    }"#;

    let engine_dir = dir.join("engine");
    std::fs::create_dir_all(&engine_dir).unwrap();
    let meta_file = engine_dir.join("metadata.json");
    std::fs::write(&meta_file, metadata).unwrap();

    // File 1: helper functions (simulates multi-file library context)
    let helpers_source = r#"
pub fn make_solid_buffer(r: f32, g: f32, b: f32, count: i32) -> Vec<f32> {
    let mut pixels = Vec::new()
    let mut i = 0
    while i < count {
        pixels.push(r)
        pixels.push(g)
        pixels.push(b)
        pixels.push(1.0)
        i = i + 1
    }
    pixels
}
"#;
    std::fs::write(src_dir.join("helpers.wj"), helpers_source).unwrap();

    // File 2: the actual test file with deep usage (200+ lines before SystemCoverage)
    let test_source = r#"
use engine::testing::input_recorder::InputRecorder
use engine::testing::golden_image::GoldenBaseline
use engine::testing::coverage::SystemCoverage

pub fn run_self_tests() -> bool {
    let mut pass_count = 0
    let mut fail_count = 0

    // Section 1: Input recorder tests (lots of statements before Coverage)
    let mut recorder = InputRecorder::new()
    recorder.start()
    let x = 10
    let y = 20
    let z = x + y
    if z > 0 {
        pass_count = pass_count + 1
    }

    // Section 2: more tests
    let a = 1.0
    let b = 2.0
    let c = a + b
    if c > 2.5 {
        pass_count = pass_count + 1
    }

    // Section 3: SystemCoverage (deep in function, should still get .to_string())
    let mut sys = SystemCoverage::new("combat")
    let fire = sys.register_function("fire")
    let reload = sys.register_function("reload")

    // Section 4: GoldenBaseline (static method with string param)
    let buf = Vec::new()
    let baseline = GoldenBaseline::from_pixels("test_view", buf)

    pass_count > fail_count
}
"#;
    std::fs::write(src_dir.join("validation.wj"), test_source).unwrap();

    let output_dir = dir.join("output");
    std::fs::create_dir_all(&output_dir).unwrap();

    let output = Command::new(wj_bin())
        .arg("build")
        .arg(src_dir.to_str().unwrap())
        .arg("--output")
        .arg(output_dir.to_str().unwrap())
        .arg("--library")
        .arg("--no-cargo")
        .arg("--metadata")
        .arg(format!("engine={}", meta_file.display()))
        .current_dir(&dir)
        .output()
        .expect("Failed to run wj");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    eprintln!("STDERR: {}", stderr);
    eprintln!("STDOUT: {}", stdout);

    let rs_file = output_dir.join("validation.rs");
    let generated = if rs_file.exists() {
        std::fs::read_to_string(&rs_file).unwrap_or_default()
    } else {
        let all_files: Vec<_> = std::fs::read_dir(&output_dir)
            .map(|d| d.filter_map(|e| e.ok().map(|e| e.path())).collect())
            .unwrap_or_default();
        panic!(
            "Generated .rs file not found at {:?}\nFiles in output: {:?}",
            rs_file, all_files
        );
    };

    eprintln!("GENERATED validation.rs:\n{}", generated);

    // sys.register_function("fire") should get .to_string() because:
    // 1. sys is inferred as SystemCoverage (constructor convention)
    // 2. register_function expects Owned String
    assert!(
        generated.contains(r#""fire".to_string()"#),
        "register_function(\"fire\") should have .to_string() in multi-file library context.\nGenerated:\n{}",
        generated
    );

    assert!(
        generated.contains(r#""reload".to_string()"#),
        "register_function(\"reload\") should have .to_string() in multi-file library context.\nGenerated:\n{}",
        generated
    );

    // GoldenBaseline::from_pixels("test_view", ...) - static method, string param
    assert!(
        generated.contains(r#""test_view".to_string()"#),
        "from_pixels(\"test_view\") should have .to_string() for owned String param.\nGenerated:\n{}",
        generated
    );

    let _ = std::fs::remove_dir_all(&dir);
}
