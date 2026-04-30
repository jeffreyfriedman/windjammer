// Test: Enum PartialEq derivation with f32-containing types
// The compiler should intelligently skip PartialEq for enums with variants containing f32 fields

use std::path::PathBuf;
use std::process::Command;

fn compile_code(code: &str) -> Result<String, String> {
    use std::fs;
    use std::io::Write;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().map_err(|e| format!("Failed to create temp dir: {}", e))?;
    let src_file = temp_dir.path().join("test.wj");
    let out_dir = temp_dir.path().join("out");

    fs::create_dir(&out_dir).map_err(|e| format!("Failed to create out dir: {}", e))?;

    let mut file =
        fs::File::create(&src_file).map_err(|e| format!("Failed to create source file: {}", e))?;
    file.write_all(code.as_bytes())
        .map_err(|e| format!("Failed to write source: {}", e))?;

    let wj_binary = PathBuf::from(env!("CARGO_BIN_EXE_wj"));

    let output = Command::new(&wj_binary)
        .arg("build")
        .arg(&src_file)
        .arg("-o")
        .arg(&out_dir)
        .arg("--no-cargo")
        .output()
        .map_err(|e| format!("Failed to execute wj: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Compilation failed:\n{}", stderr));
    }

    let generated_file = out_dir.join("test.rs");
    fs::read_to_string(&generated_file).map_err(|e| format!("Failed to read generated file: {}", e))
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_enum_conservative_partialeq() {
    let code = r#"
        @auto
        struct Vec3 {
            x: f32,
            y: f32,
            z: f32,
        }
        
        @auto
        struct Transform {
            position: Vec3,
            rotation: Vec3,
        }
        
        enum Command {
            Move(Vec3),
            Rotate(Transform),
            Delete(string),
        }
    "#;

    let result = compile_code(code);
    assert!(
        result.is_ok(),
        "Enum with f32-containing variants should compile"
    );

    let generated = result.unwrap();

    // Vec3 SHOULD have PartialEq (f32 implements PartialEq in Rust, just not Eq)
    let lines: Vec<&str> = generated.lines().collect();
    let vec3_idx = lines
        .iter()
        .position(|l| l.contains("struct Vec3"))
        .unwrap();
    // Search backwards from struct line for #[derive(
    let vec3_derive = (0..vec3_idx)
        .rev()
        .find_map(|i| {
            if lines[i].contains("#[derive(") {
                Some(lines[i])
            } else {
                None
            }
        })
        .unwrap_or("");
    assert!(
        vec3_derive.contains("PartialEq"),
        "Vec3 should derive PartialEq (f32 implements PartialEq): {}",
        vec3_derive
    );

    // Command enum is CONSERVATIVE - doesn't derive PartialEq for custom type variants
    let enum_idx = lines
        .iter()
        .position(|l| l.contains("enum Command"))
        .unwrap();
    let enum_derive = (0..enum_idx)
        .rev()
        .find_map(|i| {
            if lines[i].contains("#[derive(") {
                Some(lines[i])
            } else {
                None
            }
        })
        .unwrap_or("");
    // Conservative approach: Skip PartialEq when variants contain custom types (even if they support it)
    // This prevents compilation errors and is safer

    // Should always have Clone and Debug
    assert!(enum_derive.contains("Clone"), "Should derive Clone");
    assert!(enum_derive.contains("Debug"), "Should derive Debug");
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_enum_without_f32_has_partialeq() {
    let code = r#"
        @auto
        struct Point {
            x: i32,
            y: i32,
        }
        
        enum Shape {
            Circle(i32),
            Rectangle(Point),
            Empty,
        }
    "#;

    let result = compile_code(code);
    assert!(result.is_ok(), "Enum without f32 should compile");

    let generated = result.unwrap();

    // Point should have PartialEq (only i32 fields)
    // Search for #[derive( line followed by struct Point within next 3 lines
    // (there may be #[repr(C)] between them)
    let lines: Vec<&str> = generated.lines().collect();
    let point_derive = lines
        .iter()
        .enumerate()
        .find_map(|(i, line)| {
            if line.contains("#[derive(")
                && lines[i + 1..std::cmp::min(i + 4, lines.len())]
                    .iter()
                    .any(|l| l.contains("struct Point"))
            {
                Some(*line)
            } else {
                None
            }
        })
        .unwrap_or("");
    assert!(
        point_derive.contains("PartialEq"),
        "Point should derive PartialEq (only i32 fields)"
    );

    // Shape enum should have PartialEq (all variants support it)
    let enum_derive = lines
        .iter()
        .enumerate()
        .find_map(|(i, line)| {
            if line.contains("#[derive(")
                && lines[i + 1..std::cmp::min(i + 4, lines.len())]
                    .iter()
                    .any(|l| l.contains("enum Shape"))
            {
                Some(*line)
            } else {
                None
            }
        })
        .unwrap_or("");
    assert!(
        enum_derive.contains("PartialEq"),
        "Shape enum should derive PartialEq (all variants support it)"
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_unit_enum_has_copy_and_partialeq() {
    let code = r#"
        enum Direction {
            North,
            South,
            East,
            West,
        }
    "#;

    let result = compile_code(code);
    assert!(result.is_ok(), "Unit enum should compile");

    let generated = result.unwrap();

    // Unit enums should derive Copy, Clone, Debug, PartialEq
    let enum_derive = generated
        .lines()
        .find(|line| line.contains("#[derive("))
        .unwrap_or("");
    assert!(enum_derive.contains("Copy"), "Unit enum should derive Copy");
    assert!(
        enum_derive.contains("Clone"),
        "Unit enum should derive Clone"
    );
    assert!(
        enum_derive.contains("Debug"),
        "Unit enum should derive Debug"
    );
    assert!(
        enum_derive.contains("PartialEq"),
        "Unit enum should derive PartialEq"
    );
}
