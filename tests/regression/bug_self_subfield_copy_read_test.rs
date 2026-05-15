/// TDD: Reading Copy sub-fields of non-Copy self fields should NOT require owned self.
///
/// Bug: The analyzer's `expression_moves_non_copy_self_field` recurses into
/// nested field chains like `self.compositor.mesh_render_width` and checks
/// `self.compositor` (non-Copy) rather than `mesh_render_width` (Copy/u32).
/// This causes incorrect `self` (owned) inference instead of `&self`.
///
/// Root cause: For `self.a.b`, the code only checks whether `a` is Copy,
/// ignoring that `b` is the actual value being extracted. If `b` is Copy,
/// Rust allows reading it through a reference without moving `a`.
///
/// CRITICAL: The bug only manifests when the sub-field access is in a Let binding
/// (not a trailing return expression), because trailing expressions are excluded
/// by the getter-return optimization. The real-world trigger is:
///
///   fn build_graph(self) -> Graph {
///       let w = self.compositor.width    // <-- Let binding triggers false positive
///       Graph::new(w)
///   }
///
/// Expected Rust output: fn build_graph(&self) -> Graph
/// Actual (buggy) output: fn build_graph(self) -> Graph  (E0507!)
#[path = "../common/test_utils.rs"]
mod test_utils;

// The CRITICAL test: Copy sub-field access in a LET BINDING (not return position).
// This is the exact pattern from VoxelGPURenderer::build_game_shader_graph.
#[test]
fn test_let_binding_copy_subfield_of_non_copy_field_infers_borrowed_self() {
    let generated = test_utils::compile_single(
        r#"
struct Inner {
    width: int
    height: int
    name: string
}

struct Outer {
    inner: Inner
    count: int
}

impl Outer {
    fn compute_area(self) -> int {
        let w = self.inner.width
        let h = self.inner.height
        w * h
    }
}
"#,
    );

    assert!(
        generated.contains("fn compute_area(&self) -> i64"),
        "Expected `&self` when Let bindings only read Copy sub-fields of non-Copy field.\n\
         Bug: analyzer treats `let w = self.inner.width` as moving `self.inner` (non-Copy),\n\
         but `width` is i64 (Copy) — reading a Copy sub-field through a reference is fine.\n\
         Generated:\n{}",
        generated
    );
}

// Method call arguments with nested sub-field access (the other pattern from the game)
#[test]
fn test_method_arg_copy_subfield_of_non_copy_field_infers_borrowed_self() {
    let generated = test_utils::compile_single(
        r#"
struct Config {
    render_width: int
    render_height: int
    label: string
}

struct Output {
    width: int
    height: int
}

impl Output {
    fn new(w: int, h: int) -> Output {
        Output { width: w, height: h }
    }
}

struct Renderer {
    config: Config
}

impl Renderer {
    fn make_output(self) -> Output {
        let w = self.config.render_width
        let h = self.config.render_height
        Output::new(w, h)
    }
}
"#,
    );

    assert!(
        generated.contains("fn make_output(&self)"),
        "Expected `&self` for method that reads Copy sub-fields into local vars.\n\
         Generated:\n{}",
        generated
    );
}

// Assignment RHS with sub-field access
#[test]
fn test_assign_rhs_copy_subfield_infers_mut_self_not_owned() {
    let generated = test_utils::compile_single(
        r#"
struct Settings {
    width: int
    height: int
}

struct Engine {
    settings: Settings
    cached_area: int
}

impl Engine {
    fn update_cached_area(self) {
        let w = self.settings.width
        let h = self.settings.height
        self.cached_area = w * h
    }
}
"#,
    );

    assert!(
        generated.contains("fn update_cached_area(&mut self)"),
        "Expected `&mut self` when method mutates self field but only reads Copy sub-fields.\n\
         Generated:\n{}",
        generated
    );
}

// Deep nested chain in Let binding
#[test]
fn test_deep_nested_copy_subfield_let_binding() {
    let generated = test_utils::compile_single(
        r#"
struct Level3 {
    value: int
}

struct Level2 {
    deep: Level3
    label: string
}

struct Level1 {
    mid: Level2
}

impl Level1 {
    fn compute(self) -> int {
        let v = self.mid.deep.value
        v * 2
    }
}
"#,
    );

    assert!(
        generated.contains("fn compute(&self) -> i64"),
        "Expected `&self` for deep nested Copy sub-field in Let binding.\n\
         Generated:\n{}",
        generated
    );
}

// Trailing expression (getter pattern) should still work (regression check)
#[test]
fn test_trailing_return_copy_subfield_still_works() {
    let generated = test_utils::compile_single(
        r#"
struct Inner {
    width: int
    name: string
}

struct Outer {
    inner: Inner
}

impl Outer {
    fn get_width(self) -> int {
        self.inner.width
    }
}
"#,
    );

    assert!(
        generated.contains("fn get_width(&self) -> i64"),
        "Expected `&self` for trailing Copy sub-field return.\n\
         Generated:\n{}",
        generated
    );
}

// Real-world pattern: Let bindings + chained method calls with self.field.subfield args
#[test]
fn test_shader_graph_builder_pattern_with_nested_field_args() {
    let generated = test_utils::compile_single(
        r#"
struct GpuResources {
    camera_buf: u32
    gbuffer: u32
    label: string
}

struct Compositor {
    mesh_width: u32
    mesh_height: u32
    label: string
}

struct GraphBuilder {
    passes: int
}

impl GraphBuilder {
    fn new() -> GraphBuilder {
        GraphBuilder { passes: 0 }
    }

    fn bind(self, buf: u32) -> GraphBuilder {
        GraphBuilder { passes: self.passes + 1 }
    }

    fn build(self) -> GraphBuilder {
        self
    }
}

struct Renderer {
    resources: GpuResources
    compositor: Compositor
}

impl Renderer {
    fn build_graph(self, groups_x: u32, groups_y: u32) -> GraphBuilder {
        let mrw = self.compositor.mesh_width
        let mrh = self.compositor.mesh_height
        let gx = (mrw + 7) / 8
        let gy = (mrh + 7) / 8
        GraphBuilder::new()
            .bind(self.resources.camera_buf)
            .bind(self.resources.gbuffer)
            .build()
    }
}
"#,
    );

    assert!(
        generated.contains("fn build_graph(&self"),
        "Expected `&self` for builder pattern reading only Copy sub-fields.\n\
         Generated:\n{}",
        generated
    );
}

// Non-Copy sub-field move should still be detected
#[test]
fn test_let_binding_non_copy_subfield_detects_move() {
    let generated = test_utils::compile_single(
        r#"
struct Inner {
    name: string
    value: int
}

struct Outer {
    inner: Inner
}

impl Outer {
    fn take_name(self) -> string {
        let n = self.inner.name
        n
    }
}
"#,
    );

    // self.inner.name is String (non-Copy) in a Let binding → true move
    let has_owned = generated.contains("fn take_name(self) -> String");
    let has_borrowed_clone = generated.contains("fn take_name(&self) -> String");
    assert!(
        has_owned || has_borrowed_clone,
        "Expected owned self or &self+clone for non-Copy sub-field Let binding.\n\
         Generated:\n{}",
        generated
    );
}
