// TDD Test: Codegen Should Correctly Infer usize in Comparisons
// Bug: expression_produces_usize() has incomplete coverage, causing incorrect
//      casts in comparisons. Four patterns fail:
//      1. Method return type: frame_count() returns usize but not recognized
//      2. Nested field access: self.config.max_size -> usize not recognized
//      3. Non-self field access: asset.data_size -> usize not recognized
//      4. usize variable from method: free_slots() returns usize but not recognized
//
// Root Cause: expression_produces_usize() only handles .len()/.count()/.capacity()
//             for methods, only self.field for field access, and ignores parameters.
// Fix: Use infer_expression_type() as fallback to check if expression is usize.

#[path = "test_utils.rs"]
mod test_utils;

// Test 1: usize parameter compared with usize field
#[test]
fn test_usize_param_compared_with_usize_field() {
    let (generated, ok) = test_utils::compile_single_check(
        r#"
struct Config {
    max_size: usize,
}

struct Loader {
    config: Config,
}

impl Loader {
    fn validate_size(self, size_bytes: usize) -> bool {
        size_bytes > self.config.max_size
    }
}
"#,
    );
    let err = if !ok { generated.as_str() } else { "" };

    println!("Generated:\n{}", generated);
    if !ok {
        println!("Errors:\n{}", err);
    }

    // Should NOT cast either side to i64 - both are usize
    assert!(
        !generated.contains("as i64"),
        "Should not cast usize to i64 when both sides are usize.\nGenerated:\n{}",
        generated
    );
    assert!(ok, "Generated Rust should compile.\nErrors:\n{}", err);
}

// Test 2: usize method return compared with usize variable
#[test]
fn test_usize_method_return_compared_with_usize_var() {
    let (generated, ok) = test_utils::compile_single_check(
        r#"
struct Animation {
    frames: Vec<int>,
}

impl Animation {
    fn frame_count(self) -> usize {
        self.frames.len()
    }
}

struct Controller {
    current_frame: usize,
}

impl Controller {
    fn is_past_end(self, animation: &Animation) -> bool {
        let frame_count = animation.frame_count()
        self.current_frame >= frame_count
    }
}
"#,
    );
    let err = if !ok { generated.as_str() } else { "" };

    println!("Generated:\n{}", generated);
    if !ok {
        println!("Errors:\n{}", err);
    }

    assert!(ok, "Generated Rust should compile.\nErrors:\n{}", err);
}

// Test 3: non-self field access with usize type
#[test]
fn test_non_self_usize_field_in_comparison() {
    let (generated, ok) = test_utils::compile_single_check(
        r#"
struct Asset {
    data_size: usize,
}

struct AssetManager {
    assets: Vec<Asset>,
}

impl AssetManager {
    fn get_large_assets(self, min_size: usize) -> int {
        let mut count = 0
        for asset in &self.assets {
            if asset.data_size >= min_size {
                count = count + 1
            }
        }
        count
    }
}
"#,
    );
    let err = if !ok { generated.as_str() } else { "" };

    println!("Generated:\n{}", generated);
    if !ok {
        println!("Errors:\n{}", err);
    }

    assert!(ok, "Generated Rust should compile.\nErrors:\n{}", err);
}

// Test 4: usize from method call compared with usize variable
#[test]
fn test_usize_method_result_in_comparison() {
    let (generated, ok) = test_utils::compile_single_check(
        r#"
struct Inventory {
    slots: Vec<int>,
}

impl Inventory {
    fn free_slots(self) -> usize {
        let mut count: usize = 0
        for slot in &self.slots {
            if *slot == 0 {
                count = count + 1
            }
        }
        count
    }

    fn has_space(self, needed: usize) -> bool {
        let free = self.free_slots()
        needed <= free
    }
}
"#,
    );
    let err = if !ok { generated.as_str() } else { "" };

    println!("Generated:\n{}", generated);
    if !ok {
        println!("Errors:\n{}", err);
    }

    assert!(ok, "Generated Rust should compile.\nErrors:\n{}", err);
}
