/// TDD: Transitive &mut self inference through self.field.method()
///
/// Bug: When a method calls self.field.method() and the called method requires
/// &mut self, the calling method should also be inferred as &mut self.
///
/// Pattern:
///   struct Inner { data: Vec<i32> }
///   impl Inner { fn mutate(self) { self.data.push(42) } }
///   struct Outer { inner: Inner }
///   impl Outer { fn do_work(self) { self.inner.mutate() } }
///
/// Expected: Outer::do_work generates `&mut self` (not `&self`)
/// because it transitively mutates through self.inner.
fn compile_wj_to_rust(test_name: &str, wj_source: &str) -> String {
    let dir = std::env::temp_dir().join(format!("wj_transitive_mut_{}", test_name));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(dir.join("src/test_input.wj"), wj_source).unwrap();

    let wj_binary = env!("CARGO_BIN_EXE_windjammer");
    let output = std::process::Command::new(wj_binary)
        .arg("build")
        .arg("--path")
        .arg(dir.join("src/test_input.wj"))
        .arg("--output")
        .arg(dir.join("out"))
        .output()
        .expect("Failed to run windjammer");

    if !output.status.success() {
        panic!(
            "Compilation failed:\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let output_file = dir.join("out/test_input.rs");
    std::fs::read_to_string(&output_file).unwrap_or_else(|_| {
        panic!("Output file not found at {:?}", output_file);
    })
}

#[test]
fn test_transitive_field_mut_inference_basic() {
    let source = r#"
struct MeshData {
    vertices: Vec<f32>
}

impl MeshData {
    fn add_vertex(self, v: f32) {
        self.vertices.push(v)
    }
}

struct Renderer {
    mesh: MeshData
}

impl Renderer {
    fn render(self) {
        self.mesh.add_vertex(1.0)
    }
}
"#;

    let rust_code = compile_wj_to_rust("basic", source);

    // MeshData::add_vertex should be &mut self (directly mutates)
    assert!(
        rust_code.contains("fn add_vertex(&mut self"),
        "MeshData::add_vertex should be &mut self. Generated:\n{}",
        rust_code
    );

    // Renderer::render should ALSO be &mut self (transitively mutates via self.mesh.add_vertex)
    assert!(
        rust_code.contains("fn render(&mut self"),
        "Renderer::render should be &mut self (transitive mutation through self.mesh.add_vertex). Generated:\n{}",
        rust_code
    );
}

#[test]
fn test_transitive_field_mut_inference_deeper_chain() {
    let source = r#"
struct Buffer {
    data: Vec<u8>
}

impl Buffer {
    fn write_byte(self, b: u8) {
        self.data.push(b)
    }
}

struct Stream {
    buffer: Buffer
}

impl Stream {
    fn flush(self) {
        self.buffer.write_byte(0)
    }
}

struct Connection {
    stream: Stream
}

impl Connection {
    fn send(self) {
        self.stream.flush()
    }
}
"#;

    let rust_code = compile_wj_to_rust("deeper", source);

    assert!(
        rust_code.contains("fn write_byte(&mut self"),
        "Buffer::write_byte should be &mut self. Generated:\n{}",
        rust_code
    );

    assert!(
        rust_code.contains("fn flush(&mut self"),
        "Stream::flush should be &mut self (transitive through self.buffer.write_byte). Generated:\n{}",
        rust_code
    );

    assert!(
        rust_code.contains("fn send(&mut self"),
        "Connection::send should be &mut self (transitive through self.stream.flush). Generated:\n{}",
        rust_code
    );
}

#[test]
fn test_transitive_field_mut_inference_with_trait() {
    let source = r#"
struct MeshRenderer {
    vertices: Vec<f32>
}

impl MeshRenderer {
    fn render_mesh(self) {
        self.vertices.push(1.0)
    }
}

trait RenderPort {
    fn render(self)
}

struct GameRenderer {
    mesh_renderer: MeshRenderer
}

impl RenderPort for GameRenderer {
    fn render(self) {
        self.mesh_renderer.render_mesh()
    }
}
"#;

    let rust_code = compile_wj_to_rust("trait", source);

    assert!(
        rust_code.contains("fn render_mesh(&mut self"),
        "MeshRenderer::render_mesh should be &mut self. Generated:\n{}",
        rust_code
    );

    // The trait impl should also get &mut self since it transitively mutates
    assert!(
        rust_code.contains("fn render(&mut self"),
        "GameRenderer::render should be &mut self (transitive through self.mesh_renderer.render_mesh). Generated:\n{}",
        rust_code
    );
}

#[test]
fn test_readonly_field_method_stays_borrowed() {
    let source = r#"
struct Config {
    items: Vec<string>
}

impl Config {
    fn count(self) -> i32 {
        self.items.len()
    }
}

struct App {
    config: Config
}

impl App {
    fn item_count(self) -> i32 {
        self.config.count()
    }
}
"#;

    let rust_code = compile_wj_to_rust("readonly", source);

    // Config::count should be &self (read-only, non-Copy struct)
    assert!(
        rust_code.contains("fn count(&self"),
        "Config::count should be &self (read-only). Generated:\n{}",
        rust_code
    );

    // App::item_count should also be &self (no mutation anywhere)
    assert!(
        rust_code.contains("fn item_count(&self"),
        "App::item_count should be &self (no mutation). Generated:\n{}",
        rust_code
    );
}
