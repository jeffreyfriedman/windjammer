use std::process::Command;

fn compile_wj(source: &str) -> String {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("test.wj");
    let output_dir = dir.path().join("out");
    std::fs::create_dir_all(&output_dir).unwrap();
    std::fs::write(&input, source).unwrap();

    let result = Command::new(env!("CARGO_BIN_EXE_windjammer"))
        .args(["build", "--path", input.to_str().unwrap(), "--output", output_dir.to_str().unwrap()])
        .output()
        .expect("failed to run wj");

    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        panic!("wj build failed: {}", stderr);
    }

    let rs_file = output_dir.join("test.rs");
    std::fs::read_to_string(&rs_file).unwrap_or_else(|_| {
        let entries: Vec<_> = std::fs::read_dir(&output_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.path().display().to_string())
            .collect();
        panic!("Could not find test.rs, directory contains: {:?}", entries);
    })
}

#[test]
fn test_read_only_param_same_type_as_return_infers_borrow() {
    // duplicate_uv_coords reads src via .len() and indexing, creates a new Vec, returns it.
    // Even though param type (Vec<i32>) matches return type (Vec<i32>),
    // the param is NOT consumed — it should be &Vec<i32>.
    let source = r#"
struct Coord {
    u: f32,
    v: f32,
}

fn duplicate_coords(src: Vec<Coord>) -> Vec<Coord> {
    let mut o = Vec::new()
    let mut i = 0
    while i < src.len() {
        let c = src[i]
        o.push(Coord { u: c.u, v: c.v })
        i = i + 1
    }
    o
}

fn main() {
    let data = Vec::new()
    let dup = duplicate_coords(data)
    let again = duplicate_coords(data)
}
"#;
    let output = compile_wj(source);
    // src should be borrowed (&Vec<Coord>) because it's only read
    assert!(
        output.contains("src: &Vec<Coord>") || output.contains("src: &[Coord]"),
        "Expected src to be borrowed (&Vec<Coord> or &[Coord]), got:\n{}",
        output
    );
    // main should NOT need .clone() on data since duplicate_coords borrows
    assert!(
        !output.contains("data.clone()"),
        "data should not need .clone() since duplicate_coords borrows:\n{}",
        output
    );
}

#[test]
fn test_param_returned_directly_stays_owned() {
    // When param is directly returned, it should stay Owned
    let source = r#"
fn identity(data: Vec<i32>) -> Vec<i32> {
    data
}
"#;
    let output = compile_wj(source);
    assert!(
        output.contains("data: Vec<i32>"),
        "Expected data to be owned (Vec<i32>) since it's returned directly, got:\n{}",
        output
    );
}

#[test]
fn test_param_transformed_and_returned_stays_owned() {
    // When param is assigned to a local that's returned, it should stay Owned
    let source = r#"
fn transform(mut data: Vec<i32>) -> Vec<i32> {
    data.push(42)
    data
}
"#;
    let output = compile_wj(source);
    assert!(
        output.contains("data: Vec<i32>") || output.contains("data: &mut Vec<i32>"),
        "Expected data to be owned or &mut since it's mutated and returned, got:\n{}",
        output
    );
}
