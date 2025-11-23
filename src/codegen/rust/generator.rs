// Rust code generator
use crate::analyzer::*;
use crate::parser::*;
use crate::CompilationTarget;

/// Information about game framework decorators
#[derive(Debug, Clone)]
struct GameFrameworkInfo {
    game_struct: String,
    init_fn: Option<String>,
    update_fn: Option<String>,
    render_fn: Option<String>,
    input_fn: Option<String>,
    cleanup_fn: Option<String>,
    is_3d: bool, // True if using @render3d
}

/// Information about UI framework usage
#[derive(Debug, Clone)]
struct UIFrameworkInfo {
    uses_ui: bool, // True if imports std::ui::*
}

/// Information about platform API usage
#[derive(Debug, Clone, Default)]
struct PlatformApis {
    needs_fs: bool,       // True if imports std::fs
    needs_process: bool,  // True if imports std::process
    needs_dialog: bool,   // True if imports std::dialog
    needs_env: bool,      // True if imports std::env
    needs_encoding: bool, // True if imports std::encoding
    needs_compute: bool,  // True if imports std::compute
    needs_net: bool,      // True if imports std::net
    needs_http: bool,     // True if imports std::http
    needs_storage: bool,  // True if imports std::storage
}

pub struct CodeGenerator {
    indent_level: usize,
    signature_registry: SignatureRegistry,
    in_wasm_bindgen_impl: bool,
    needs_wasm_imports: bool,
    needs_web_imports: bool,
    needs_js_imports: bool,
    needs_serde_imports: bool,   // For JSON support
    needs_write_import: bool,    // For string capacity optimization (write! macro)
    needs_smallvec_import: bool, // For Phase 8 SmallVec optimization
    needs_cow_import: bool,      // For Phase 9 Cow optimization
    target: CompilationTarget,
    is_module: bool, // true if generating code for a reusable module (not main file)
    source_map: crate::source_map::SourceMap,
    current_output_file: std::path::PathBuf, // Path to the Rust file being generated
    current_rust_line: usize, // Current line number in generated Rust code (1-indexed)
    current_wj_file: std::path::PathBuf, // Path to the Windjammer file being compiled
    inferred_bounds: std::collections::HashMap<String, crate::inference::InferredBounds>,
    needs_trait_imports: std::collections::HashSet<String>, // Tracks which traits need imports
    bound_aliases: std::collections::HashMap<String, Vec<String>>, // bound Name = Trait + Trait
    // PHASE 2 OPTIMIZATION: Track variables that can avoid cloning
    clone_optimizations: std::collections::HashSet<String>, // Variables that don't need .clone()
    // PHASE 3 OPTIMIZATION: Track struct mapping optimizations
    struct_mapping_hints: std::collections::HashMap<String, crate::analyzer::MappingStrategy>, // Struct name -> strategy
    // PHASE 4 OPTIMIZATION: Track string operation optimizations
    string_capacity_hints: std::collections::HashMap<usize, usize>, // Statement idx -> capacity
    // PHASE 5 OPTIMIZATION: Track assignment operations that can use compound operators
    assignment_optimizations: std::collections::HashMap<String, crate::analyzer::CompoundOp>, // Variable -> compound op
    // PHASE 6 OPTIMIZATION: Track defer drop optimizations
    defer_drop_optimizations: Vec<crate::analyzer::DeferDropOptimization>,
    // PHASE 8 OPTIMIZATION: Track SmallVec optimizations
    smallvec_optimizations:
        std::collections::HashMap<String, crate::analyzer::SmallVecOptimization>, // Variable -> SmallVec config
    // PHASE 9 OPTIMIZATION: Track Cow optimizations
    cow_optimizations: std::collections::HashSet<String>, // Variables that can use Cow
    // AUTO-CLONE: Track where to automatically insert clones
    auto_clone_analysis: Option<crate::auto_clone::AutoCloneAnalysis>,
    // Track current statement index for optimization hints
    current_statement_idx: usize,
    // IMPLICIT SELF SUPPORT: Track struct fields for implicit self references
    current_struct_fields: std::collections::HashSet<String>, // Field names in current impl block
    in_impl_block: bool, // true if currently generating code for an impl block
    // FUNCTION CONTEXT: Track current function parameters for compound assignment optimization
    current_function_params: Vec<crate::parser::Parameter>, // Parameters of the current function
}

impl CodeGenerator {
    pub fn new(registry: SignatureRegistry, target: CompilationTarget) -> Self {
        CodeGenerator {
            indent_level: 0,
            signature_registry: registry,
            in_wasm_bindgen_impl: false,
            needs_wasm_imports: false,
            needs_web_imports: false,
            needs_js_imports: false,
            needs_serde_imports: false,
            needs_write_import: false,
            needs_smallvec_import: false,
            needs_cow_import: false,
            target,
            is_module: false,
            source_map: crate::source_map::SourceMap::new(),
            current_output_file: std::path::PathBuf::new(),
            current_rust_line: 1,
            current_wj_file: std::path::PathBuf::new(),
            inferred_bounds: std::collections::HashMap::new(),
            needs_trait_imports: std::collections::HashSet::new(),
            bound_aliases: std::collections::HashMap::new(),
            clone_optimizations: std::collections::HashSet::new(),
            struct_mapping_hints: std::collections::HashMap::new(),
            string_capacity_hints: std::collections::HashMap::new(),
            assignment_optimizations: std::collections::HashMap::new(),
            defer_drop_optimizations: Vec::new(),
            smallvec_optimizations: std::collections::HashMap::new(),
            cow_optimizations: std::collections::HashSet::new(),
            auto_clone_analysis: None,
            current_statement_idx: 0,
            current_struct_fields: std::collections::HashSet::new(),
            in_impl_block: false,
            current_function_params: Vec::new(),
        }
    }

    /// Set inferred trait bounds for functions
    pub fn set_inferred_bounds(
        &mut self,
        bounds: std::collections::HashMap<String, crate::inference::InferredBounds>,
    ) {
        self.inferred_bounds = bounds;
    }

    pub fn new_for_module(registry: SignatureRegistry, target: CompilationTarget) -> Self {
        let mut gen = Self::new(registry, target);
        gen.is_module = true;
        gen
    }

    fn indent(&self) -> String {
        "    ".repeat(self.indent_level)
    }

    // ============================================================================
    // SOURCE MAP TRACKING
    // ============================================================================

    /// Set the output file path for source mapping
    pub fn set_output_file(&mut self, path: impl Into<std::path::PathBuf>) {
        self.current_output_file = path.into();
    }

    /// Set the Windjammer source file path for source mapping
    pub fn set_source_file(&mut self, path: impl Into<std::path::PathBuf>) {
        self.current_wj_file = path.into();
    }

    /// Get the current line number in the generated Rust code
    #[allow(dead_code)]
    fn current_rust_line(&self) -> usize {
        self.current_rust_line
    }

    /// Increment the Rust line counter (call after generating each line)
    #[allow(dead_code)]
    fn increment_rust_line(&mut self) {
        self.current_rust_line += 1;
    }

    /// Increment the Rust line counter by N lines
    #[allow(dead_code)]
    fn increment_rust_lines(&mut self, count: usize) {
        self.current_rust_line += count;
    }

    /// Record a mapping from current Rust location to Windjammer location
    fn record_mapping(&mut self, wj_location: &crate::source_map::Location) {
        if !self.current_output_file.as_os_str().is_empty() {
            self.source_map.add_mapping(
                self.current_output_file.clone(),
                self.current_rust_line,
                0, // column (simplified for now)
                wj_location.file.clone(),
                wj_location.line,
                wj_location.column,
            );
        }
    }

    /// Extract location from an Expression
    #[allow(dead_code)]
    fn get_expression_location(&self, expr: &Expression) -> Option<crate::source_map::Location> {
        expr.location().clone()
    }

    /// Extract location from a Statement
    fn get_statement_location(&self, stmt: &Statement) -> Option<crate::source_map::Location> {
        stmt.location().clone()
    }

    /// Extract location from an Item
    #[allow(dead_code)]
    fn get_item_location(&self, item: &Item) -> Option<crate::source_map::Location> {
        item.location().clone()
    }

    /// Get the source map (for saving after code generation)
    pub fn get_source_map(&self) -> &crate::source_map::SourceMap {
        &self.source_map
    }

    /// Count newlines in a string and increment the Rust line counter
    #[allow(dead_code)]
    fn track_generated_lines(&mut self, code: &str) {
        let newline_count = code.matches('\n').count();
        if newline_count > 0 {
            self.increment_rust_lines(newline_count);
        }
    }

    /// Generate a statement with automatic source tracking
    #[allow(dead_code)]
    fn generate_statement_tracked(&mut self, stmt: &Statement) -> String {
        let code = self.generate_statement(stmt);
        self.track_generated_lines(&code);
        code
    }

    /// Map Windjammer decorators to Rust attributes
    /// This abstraction layer allows us to use semantic Windjammer names
    /// while generating appropriate Rust attributes based on compilation target
    fn map_decorator(&mut self, name: &str) -> String {
        match (name, self.target) {
            ("export", CompilationTarget::Wasm) => {
                self.needs_wasm_imports = true;
                "wasm_bindgen".to_string()
            }
            ("export", CompilationTarget::Node) => {
                // Future: Node.js native modules via Neon
                "neon::export".to_string()
            }
            ("export", CompilationTarget::Python) => {
                // Future: Python bindings via PyO3
                "pyfunction".to_string()
            }
            ("export", CompilationTarget::C) => {
                // Future: C FFI
                "no_mangle".to_string()
            }
            ("test", _) => "test".to_string(),
            ("async", _) => "async".to_string(),
            // HTTP method decorators for Axum
            ("get", _) => "axum::routing::get".to_string(),
            ("post", _) => "axum::routing::post".to_string(),
            ("put", _) => "axum::routing::put".to_string(),
            ("delete", _) => "axum::routing::delete".to_string(),
            ("patch", _) => "axum::routing::patch".to_string(),
            // Pass through other decorators as-is
            (other, _) => other.to_string(),
        }
    }

    // ============================================================================
    // UI FRAMEWORK SUPPORT
    // ============================================================================

    /// Check if an expression is a UI component that needs .to_vnode()
    #[allow(dead_code, clippy::only_used_in_recursion)]
    fn is_ui_component_expr(&self, expr: &Expression) -> bool {
        // List of UI components that implement ToVNode
        const UI_COMPONENTS: &[&str] = &[
            "Button",
            "Text",
            "Panel",
            "Container",
            "Flex",
            "Input",
            "CodeEditor",
            "FileTree",
            "Alert",
            "Card",
            "Grid",
            "Toolbar",
            "Tabs",
            "Checkbox",
            "Radio",
            "Select",
            "Switch",
            "Dialog",
            "Slider",
            "Tooltip",
            "Badge",
            "Progress",
            "Spinner",
        ];

        match expr {
            // Button::new(...) -> check if Button is a UI component
            Expression::Call { function, .. } => {
                if let Expression::FieldAccess { object, .. } = &**function {
                    // Type::method() pattern - check the object (Button), not the method (new)
                    if let Expression::Identifier { name, .. } = &**object {
                        return UI_COMPONENTS.contains(&name.as_str());
                    }
                }
                false
            }
            // button.variant(...).on_click(...) -> check the root object
            Expression::MethodCall { object, .. } => self.is_ui_component_expr(object),
            _ => false,
        }
    }

    /// Check if a method is a builder method that returns Self (for chaining)
    #[allow(dead_code)]
    fn is_builder_method(&self, method: &str) -> bool {
        // Common builder methods that return Self
        const BUILDER_METHODS: &[&str] = &[
            "variant",
            "size",
            "on_click",
            "on_change",
            "placeholder",
            "child",
            "direction",
            "gap",
            "max_width",
            "padding",
            "title",
            "language",
            "theme",
            "bold",
            "disabled",
        ];
        BUILDER_METHODS.contains(&method)
    }

    // ============================================================================
    // GAME FRAMEWORK SUPPORT
    // ============================================================================

    /// Detect if this program uses game framework decorators
    fn detect_game_framework(&self, program: &Program) -> Option<GameFrameworkInfo> {
        let mut game_struct = None;
        let mut init_fn = None;
        let mut update_fn = None;
        let mut render_fn = None;
        let mut input_fn = None;
        let mut cleanup_fn = None;

        // Find @game struct
        for item in &program.items {
            if let Item::Struct { decl: s, .. } = item {
                if s.decorators.iter().any(|d| d.name == "game") {
                    game_struct = Some(s.name.clone());
                    break;
                }
            }
        }

        // If no @game struct, this isn't a game
        game_struct.as_ref()?;

        // Find decorated functions
        let mut is_3d = false;
        for item in &program.items {
            if let Item::Function { decl: func, .. } = item {
                for decorator in &func.decorators {
                    match decorator.name.as_str() {
                        "init" => init_fn = Some(func.name.clone()),
                        "update" => update_fn = Some(func.name.clone()),
                        "render" => render_fn = Some(func.name.clone()),
                        "render3d" => {
                            render_fn = Some(func.name.clone());
                            is_3d = true; // Mark as 3D rendering
                        }
                        "input" => input_fn = Some(func.name.clone()),
                        "cleanup" => cleanup_fn = Some(func.name.clone()),
                        _ => {}
                    }
                }
            }
        }

        Some(GameFrameworkInfo {
            game_struct: game_struct.unwrap(),
            init_fn,
            update_fn,
            render_fn,
            input_fn,
            cleanup_fn,
            is_3d,
        })
    }

    // ============================================================================
    // UI FRAMEWORK SUPPORT
    // ============================================================================

    /// Detect if this program uses UI framework (std::ui)
    fn detect_ui_framework(&self, program: &Program) -> UIFrameworkInfo {
        let mut uses_ui = false;

        // Check for use std::ui::* or use std::ui
        for item in &program.items {
            if let Item::Use { path, .. } = item {
                let path_str = path.join("::");
                if path_str == "std::ui" || path_str.starts_with("std::ui::") {
                    uses_ui = true;
                    break;
                }
            }
        }

        UIFrameworkInfo { uses_ui }
    }

    /// Detect which platform APIs are used (std::fs, std::process, etc.)
    fn detect_platform_apis(&self, program: &Program) -> PlatformApis {
        let mut apis = PlatformApis::default();

        for item in &program.items {
            if let Item::Use { path, .. } = item {
                let path_str = path.join("::");

                if path_str == "std::fs" || path_str.starts_with("std::fs::") {
                    apis.needs_fs = true;
                }
                if path_str == "std::process" || path_str.starts_with("std::process::") {
                    apis.needs_process = true;
                }
                if path_str == "std::dialog" || path_str.starts_with("std::dialog::") {
                    apis.needs_dialog = true;
                }
                if path_str == "std::env" || path_str.starts_with("std::env::") {
                    apis.needs_env = true;
                }
                if path_str == "std::encoding" || path_str.starts_with("std::encoding::") {
                    apis.needs_encoding = true;
                }
                if path_str == "std::compute" || path_str.starts_with("std::compute::") {
                    apis.needs_compute = true;
                }
                if path_str == "std::net" || path_str.starts_with("std::net::") {
                    apis.needs_net = true;
                }
                if path_str == "std::http" || path_str.starts_with("std::http::") {
                    apis.needs_http = true;
                }
                if path_str == "std::storage" || path_str.starts_with("std::storage::") {
                    apis.needs_storage = true;
                }
            }
        }

        apis
    }

    /// Detect if this program imports std::game (for non-decorator game usage)
    fn detect_game_import(&self, program: &Program) -> bool {
        // Check for use std::game::* or use std::game
        for item in &program.items {
            if let Item::Use { path, .. } = item {
                let path_str = path.join("::");
                if path_str == "std::game" || path_str.starts_with("std::game::") {
                    return true;
                }
            }
        }
        false
    }

    /// Generate game loop main function
    fn generate_game_main(&mut self, info: &GameFrameworkInfo) -> String {
        let mut output = String::new();

        // Generate GameWorld wrapper struct
        output.push_str("// Generated: ECS world wrapper\n");
        output.push_str("struct GameWorld {\n");
        output.push_str("    world: windjammer_game_framework::ecs::World,\n");
        output.push_str("    game_entity: windjammer_game_framework::ecs::Entity,\n");
        output.push_str("}\n\n");

        output.push_str("impl GameWorld {\n");
        output.push_str("    fn new() -> Self {\n");
        output.push_str("        use windjammer_game_framework::ecs::*;\n");
        output.push_str("        let mut world = World::new();\n");
        output.push_str("        \n");
        output.push_str("        // Spawn game entity with game component\n");
        output.push_str("        let game_entity = world.spawn()\n");
        output.push_str(&format!(
            "            .with({}::default())\n",
            info.game_struct
        ));
        output.push_str("            .build();\n");
        output.push_str("        \n");
        output.push_str("        Self { world, game_entity }\n");
        output.push_str("    }\n");
        output.push_str("    \n");
        output.push_str(&format!(
            "    fn game_mut(&mut self) -> &mut {} {{\n",
            info.game_struct
        ));
        output.push_str(&format!(
            "        self.world.get_component_mut::<{}>(self.game_entity).unwrap()\n",
            info.game_struct
        ));
        output.push_str("    }\n");
        output.push_str("}\n\n");

        output.push_str("fn main() -> Result<(), Box<dyn std::error::Error>> {\n");
        output.push_str("    use windjammer_game_framework::*;\n");
        output.push_str("    use windjammer_game_framework::ecs::*;\n");
        output.push_str("    use winit::event::{Event, WindowEvent};\n");
        output.push_str("    use winit::event_loop::{ControlFlow, EventLoop};\n");
        output.push_str("    use winit::window::WindowBuilder;\n");
        output.push('\n');
        output.push_str("    // Create event loop and window\n");
        output.push_str("    let event_loop = EventLoop::new()?;\n");
        output.push_str("    let window = WindowBuilder::new()\n");
        output.push_str("        .with_title(\"Windjammer Game\")\n");
        output.push_str("        .with_inner_size(winit::dpi::LogicalSize::new(800, 600))\n");
        output.push_str("        .build(&event_loop)?;\n");
        output.push('\n');
        output.push_str("    // Initialize ECS world\n");
        output.push_str("    let mut game_world = GameWorld::new();\n");
        output.push('\n');

        // Call init function if present
        if let Some(init_fn) = &info.init_fn {
            output.push_str("    // Call init function\n");
            output.push_str(&format!("    {}(game_world.game_mut());\n", init_fn));
            output.push('\n');
        }

        output.push_str("    // Initialize renderer\n");
        output.push_str("    let window_ref: &'static winit::window::Window = unsafe { std::mem::transmute(&window) };\n");
        if info.is_3d {
            output.push_str(
                "    let mut renderer = pollster::block_on(renderer3d::Renderer3D::new(window_ref))?;\n",
            );
            output.push_str("    let mut camera = renderer3d::Camera3D::new();\n");
        } else {
            output.push_str(
                "    let mut renderer = pollster::block_on(renderer::Renderer::new(window_ref))?;\n",
            );
        }
        output.push('\n');
        output.push_str("    // Initialize input\n");
        output.push_str("    let mut input = input::Input::new();\n");
        output.push('\n');
        output.push_str("    // Game loop\n");
        output.push_str("    let mut last_time = std::time::Instant::now();\n");
        output.push('\n');
        output.push_str("    event_loop.run(move |event, elwt| {\n");
        output.push_str("        match event {\n");
        output.push_str("            Event::WindowEvent { event, .. } => match event {\n");
        output.push_str("                WindowEvent::CloseRequested => {\n");

        // Call cleanup function if present
        if let Some(cleanup_fn) = &info.cleanup_fn {
            output.push_str(&format!(
                "                    {}(game_world.game_mut());\n",
                cleanup_fn
            ));
        }

        output.push_str("                    elwt.exit();\n");
        output.push_str("                }\n");
        output.push_str("                WindowEvent::RedrawRequested => {\n");
        output.push_str("                    // Calculate delta time\n");
        output.push_str("                    let now = std::time::Instant::now();\n");
        output.push_str("                    let delta = (now - last_time).as_secs_f64();\n");
        output.push_str("                    last_time = now;\n");
        output.push('\n');

        // Call update function if present
        if let Some(update_fn) = &info.update_fn {
            output.push_str("                    // Update game logic\n");
            output.push_str(&format!(
                "                    {}(game_world.game_mut(), delta, &input);\n",
                update_fn
            ));
            output.push('\n');
            output.push_str("                    // Update ECS systems (scene graph, etc.)\n");
            output.push_str(
                "                    SceneGraph::update_transforms(&mut game_world.world);\n",
            );
            output.push('\n');
        }

        // Call render function if present
        if let Some(render_fn) = &info.render_fn {
            output.push_str("                    // Render\n");
            if info.is_3d {
                output.push_str("                    renderer.set_camera(&camera);\n");
                output.push_str(&format!(
                    "                    {}(game_world.game_mut(), &mut renderer, &mut camera);\n",
                    render_fn
                ));
            } else {
                output.push_str(&format!(
                    "                    {}(game_world.game_mut(), &mut renderer);\n",
                    render_fn
                ));
            }
        }

        output.push_str("                    renderer.present();\n");
        output.push('\n');
        output.push_str("                    // Clear input frame state\n");
        output.push_str("                    input.clear_frame_state();\n");
        output.push_str("                }\n");

        // Handle input if input function present
        if let Some(input_fn) = &info.input_fn {
            output.push_str("                WindowEvent::KeyboardInput { event, .. } => {\n");
            output.push_str("                    input.update_from_winit(&event);\n");
            output.push_str(&format!(
                "                    {}(game_world.game_mut(), &input);\n",
                input_fn
            ));
            output.push_str("                }\n");

            // Handle mouse button input
            output.push_str("                WindowEvent::MouseInput { state, button, .. } => {\n");
            output.push_str(
                "                    input.update_mouse_button_from_winit(state, button);\n",
            );
            output.push_str(&format!(
                "                    {}(game_world.game_mut(), &input);\n",
                input_fn
            ));
            output.push_str("                }\n");

            // Handle mouse movement
            output.push_str("                WindowEvent::CursorMoved { position, .. } => {\n");
            output.push_str("                    input.update_mouse_position_from_winit(position.x, position.y);\n");
            output.push_str("                }\n");
        }

        output.push_str("                _ => {}\n");
        output.push_str("            },\n");
        output.push_str("            Event::AboutToWait => {\n");
        output.push_str("                window.request_redraw();\n");
        output.push_str("            }\n");
        output.push_str("            _ => {}\n");
        output.push_str("        }\n");
        output.push_str("    })?;\n");
        output.push('\n');
        output.push_str("    Ok(())\n");
        output.push_str("}\n");

        output
    }

    fn generate_block(&mut self, stmts: &[Statement]) -> String {
        let mut output = String::new();
        let len = stmts.len();
        for (i, stmt) in stmts.iter().enumerate() {
            // Track current statement index for optimization hints
            self.current_statement_idx = i;

            let is_last = i == len - 1;
            if is_last
                && matches!(
                    stmt,
                    Statement::Expression { .. }
                        | Statement::Thread { .. }
                        | Statement::Async { .. }
                )
            {
                // Last statement is an expression or thread/async block - generate without discard (it's the return value)
                match stmt {
                    Statement::Expression { expr, .. } => {
                        output.push_str(&self.indent());
                        output.push_str(&self.generate_expression(expr));
                        output.push('\n');
                    }
                    Statement::Thread { body, .. } => {
                        // Generate as expression (returns JoinHandle)
                        output.push_str(&self.indent());
                        output.push_str("std::thread::spawn(move || {\n");
                        self.indent_level += 1;
                        for stmt in body {
                            output.push_str(&self.generate_statement(stmt));
                        }
                        self.indent_level -= 1;
                        output.push_str(&self.indent());
                        output.push_str("})\n");
                    }
                    Statement::Async { body, .. } => {
                        // Generate as expression (returns JoinHandle)
                        output.push_str(&self.indent());
                        output.push_str("tokio::spawn(async move {\n");
                        self.indent_level += 1;
                        for stmt in body {
                            output.push_str(&self.generate_statement(stmt));
                        }
                        self.indent_level -= 1;
                        output.push_str(&self.indent());
                        output.push_str("})\n");
                    }
                    _ => unreachable!(),
                }
            } else {
                output.push_str(&self.generate_statement(stmt));
            }
        }
        output
    }

    pub fn generate_program(&mut self, program: &Program, analyzed: &[AnalyzedFunction]) -> String {
        let mut imports = String::new();
        let mut body = String::new();

        // Detect game framework decorators
        let game_framework_info = self.detect_game_framework(program);

        // Detect UI framework usage
        let ui_framework_info = self.detect_ui_framework(program);

        // Detect platform API usage
        let platform_apis = self.detect_platform_apis(program);

        // Collect bound aliases first (bound Name = Trait + Trait)
        for item in &program.items {
            if let Item::BoundAlias { name, traits, .. } = item {
                self.bound_aliases.insert(name.clone(), traits.clone());
            }
        }

        // Collect struct definitions for implicit self support
        let mut struct_fields: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        for item in &program.items {
            if let Item::Struct { decl: s, .. } = item {
                let field_names: Vec<String> = s.fields.iter().map(|f| f.name.clone()).collect();
                struct_fields.insert(s.name.clone(), field_names);
            }
        }

        // Check for stdlib modules that need special imports
        for item in &program.items {
            if let Item::Use { path, .. } = item {
                // Path is ["std", "json"] for "use std::json"
                let path_str = path.join("::");
                if (path_str.starts_with("std::") || path_str == "std") && path_str.contains("json")
                {
                    self.needs_serde_imports = true;
                }
                // http, time, crypto modules don't need special imports (used directly)
            }
        }

        // Generate explicit use statements
        for item in &program.items {
            if let Item::Use { path, alias, .. } = item {
                imports.push_str(&self.generate_use(path, alias.as_deref()));
                imports.push('\n');
            }
        }

        // Generate const and static declarations
        for item in &program.items {
            match item {
                Item::Const {
                    name, type_, value, ..
                } => {
                    let pub_prefix = if self.is_module { "pub " } else { "" };

                    // Special case: string constants should use &'static str, not String
                    let rust_type = if matches!(type_, Type::String)
                        && matches!(
                            value,
                            Expression::Literal {
                                value: Literal::String(_),
                                ..
                            }
                        ) {
                        "&'static str".to_string()
                    } else {
                        self.type_to_rust(type_)
                    };

                    body.push_str(&format!(
                        "{}const {}: {} = {};\n",
                        pub_prefix,
                        name,
                        rust_type,
                        self.generate_expression_immut(value)
                    ));
                }
                Item::Static {
                    name,
                    mutable,
                    type_,
                    value,
                    ..
                } => {
                    if *mutable {
                        body.push_str(&format!(
                            "static mut {}: {} = {};\n",
                            name,
                            self.type_to_rust(type_),
                            self.generate_expression_immut(value)
                        ));
                    } else {
                        // PHASE 7: Promote static to const if value is compile-time evaluable
                        let keyword = if self.is_const_evaluable(value) {
                            "const" // Zero runtime overhead!
                        } else {
                            "static"
                        };

                        body.push_str(&format!(
                            "{} {}: {} = {};\n",
                            keyword,
                            name,
                            self.type_to_rust(type_),
                            self.generate_expression_immut(value)
                        ));
                    }
                }
                _ => {}
            }
        }

        if !body.is_empty() {
            body.push('\n');
        }

        // Collect names of functions in impl blocks to avoid generating them twice
        let mut impl_methods = std::collections::HashSet::new();
        for item in &program.items {
            if let Item::Impl {
                block: impl_block, ..
            } = item
            {
                for func in &impl_block.functions {
                    impl_methods.insert(func.name.clone());
                }
            }
        }

        // Generate structs, enums, and traits
        for item in &program.items {
            match item {
                Item::Struct { decl: s, .. } => {
                    body.push_str(&self.generate_struct(s));
                    body.push_str("\n\n");

                    // Check for @component or @game decorators and generate trait implementations
                    if s.decorators.iter().any(|d| d.name == "component") {
                        body.push_str(&self.generate_component_impl(s));
                        body.push_str("\n\n");
                    }
                    if s.decorators.iter().any(|d| d.name == "game") {
                        body.push_str(&self.generate_game_impl(s));
                        body.push_str("\n\n");
                    }
                }
                Item::Enum { decl: e, .. } => {
                    body.push_str(&self.generate_enum(e));
                    body.push_str("\n\n");
                }
                Item::Trait { decl: t, .. } => {
                    body.push_str(&self.generate_trait(t));
                    body.push_str("\n\n");
                }
                Item::Impl {
                    block: impl_block, ..
                } => {
                    // Set the struct fields for implicit self support
                    if let Some(fields) = struct_fields.get(&impl_block.type_name) {
                        self.current_struct_fields = fields.iter().cloned().collect();
                    } else {
                        self.current_struct_fields.clear();
                    }
                    self.in_impl_block = true;

                    body.push_str(&self.generate_impl(impl_block, analyzed));
                    body.push_str("\n\n");

                    self.in_impl_block = false;
                    self.current_struct_fields.clear();
                }
                _ => {}
            }
        }

        // Collect game-decorated function names to skip them
        let mut game_functions = std::collections::HashSet::new();
        if let Some(ref info) = game_framework_info {
            if let Some(ref fn_name) = info.init_fn {
                game_functions.insert(fn_name.clone());
            }
            if let Some(ref fn_name) = info.update_fn {
                game_functions.insert(fn_name.clone());
            }
            if let Some(ref fn_name) = info.render_fn {
                game_functions.insert(fn_name.clone());
            }
            if let Some(ref fn_name) = info.input_fn {
                game_functions.insert(fn_name.clone());
            }
            if let Some(ref fn_name) = info.cleanup_fn {
                game_functions.insert(fn_name.clone());
            }
        }

        // Generate game-decorated functions FIRST (before main)
        if game_framework_info.is_some() {
            for analyzed_func in analyzed {
                if game_functions.contains(&analyzed_func.decl.name) {
                    body.push_str(&self.generate_function(analyzed_func));
                    body.push_str("\n\n");
                }
            }
        }

        // Generate top-level functions (skip impl methods and game-decorated functions)
        for analyzed_func in analyzed {
            if !impl_methods.contains(&analyzed_func.decl.name) {
                // Skip main() function in modules - it should only be in the entry point
                if self.is_module && analyzed_func.decl.name == "main" {
                    continue;
                }
                // Skip main() if this is a game (we'll generate our own)
                if game_framework_info.is_some() && analyzed_func.decl.name == "main" {
                    continue;
                }
                // Skip game-decorated functions (they were already generated above)
                if game_functions.contains(&analyzed_func.decl.name) {
                    continue;
                }
                // Generate the function
                body.push_str(&self.generate_function(analyzed_func));
                body.push_str("\n\n");
            }
        }

        // Generate game main function if this is a game
        if let Some(ref info) = game_framework_info {
            body.push_str(&self.generate_game_main(info));
            body.push_str("\n\n");
        }

        // Inject implicit imports if needed
        let mut implicit_imports = String::new();

        // Add game framework imports if this is a game (via decorators or std::game import)
        let uses_game_decorators = game_framework_info.is_some();
        let uses_game_import = self.detect_game_import(program);

        if uses_game_decorators || uses_game_import {
            if let Some(ref info) = game_framework_info {
                if info.is_3d {
                    implicit_imports.push_str(
                        "use windjammer_game_framework::renderer3d::{Renderer3D, Camera3D};\n",
                    );
                    implicit_imports.push_str("use windjammer_game_framework::renderer::Color;\n");
                } else {
                    implicit_imports
                        .push_str("use windjammer_game_framework::renderer::{Renderer, Color};\n");
                }
            } else {
                // Default to 2D renderer if no decorator info
                implicit_imports
                    .push_str("use windjammer_game_framework::renderer::{Renderer, Color};\n");
            }
            implicit_imports
                .push_str("use windjammer_game_framework::input::{Input, Key, MouseButton};\n");
            implicit_imports.push_str("use windjammer_game_framework::math::{Vec3, Mat4};\n");
            implicit_imports.push_str("use windjammer_game_framework::ecs::*;\n");
            implicit_imports.push_str("use windjammer_game_framework::game_app::GameApp;\n");
        }

        // Add UI framework imports if using std::ui
        if ui_framework_info.uses_ui {
            implicit_imports.push_str("use windjammer_ui::prelude::*;\n");
            implicit_imports.push_str("use windjammer_ui::components::*;\n");
            implicit_imports.push_str("use windjammer_ui::simple_vnode::{VNode, VAttr};\n");
        }

        // Add platform API imports based on target
        if platform_apis.needs_fs
            || platform_apis.needs_process
            || platform_apis.needs_dialog
            || platform_apis.needs_env
            || platform_apis.needs_encoding
            || platform_apis.needs_compute
            || platform_apis.needs_net
            || platform_apis.needs_http
            || platform_apis.needs_storage
        {
            // Use platform-specific imports based on compilation target
            let platform = match self.target {
                CompilationTarget::Wasm => "wasm",
                CompilationTarget::Rust => "native",
                _ => "native", // Default to native for other targets
            };

            if platform_apis.needs_fs {
                implicit_imports.push_str(&format!(
                    "use windjammer_runtime::platform::{}::fs;\n",
                    platform
                ));
                implicit_imports.push_str(&format!(
                    "use windjammer_runtime::platform::{}::fs::*;\n",
                    platform
                ));
            }
            if platform_apis.needs_process {
                implicit_imports.push_str(&format!(
                    "use windjammer_runtime::platform::{}::process;\n",
                    platform
                ));
                implicit_imports.push_str(&format!(
                    "use windjammer_runtime::platform::{}::process::*;\n",
                    platform
                ));
            }
            if platform_apis.needs_dialog {
                implicit_imports.push_str(&format!(
                    "use windjammer_runtime::platform::{}::dialog;\n",
                    platform
                ));
                implicit_imports.push_str(&format!(
                    "use windjammer_runtime::platform::{}::dialog::*;\n",
                    platform
                ));
            }
            if platform_apis.needs_env {
                implicit_imports.push_str(&format!(
                    "use windjammer_runtime::platform::{}::env::*;\n",
                    platform
                ));
            }
            if platform_apis.needs_encoding {
                implicit_imports.push_str(&format!(
                    "use windjammer_runtime::platform::{}::encoding::*;\n",
                    platform
                ));
            }
            if platform_apis.needs_compute {
                implicit_imports.push_str(&format!(
                    "use windjammer_runtime::platform::{}::compute::*;\n",
                    platform
                ));
            }
            if platform_apis.needs_net {
                implicit_imports.push_str(&format!(
                    "use windjammer_runtime::platform::{}::net::*;\n",
                    platform
                ));
            }
            if platform_apis.needs_http {
                implicit_imports.push_str(&format!(
                    "use windjammer_runtime::platform::{}::http::*;\n",
                    platform
                ));
            }
            if platform_apis.needs_storage {
                implicit_imports.push_str(&format!(
                    "use windjammer_runtime::platform::{}::storage::*;\n",
                    platform
                ));
            }
        }

        // Add trait imports for inferred bounds
        if !self.needs_trait_imports.is_empty() {
            let mut sorted_traits: Vec<_> = self.needs_trait_imports.iter().collect();
            sorted_traits.sort();
            for trait_name in sorted_traits {
                match trait_name.as_str() {
                    "Display" | "Debug" => {
                        implicit_imports.push_str(&format!("use std::fmt::{};\n", trait_name));
                    }
                    "Clone" => {
                        // Clone is in prelude, no import needed
                    }
                    "Add" | "Sub" | "Mul" | "Div" => {
                        implicit_imports.push_str(&format!("use std::ops::{};\n", trait_name));
                    }
                    "PartialEq" | "Eq" | "PartialOrd" | "Ord" => {
                        // These are in prelude, no import needed
                    }
                    "IntoIterator" | "Iterator" => {
                        // These are in prelude, no import needed
                    }
                    _ => {
                        // Custom trait, assume it's already in scope
                    }
                }
            }
        }

        if self.needs_wasm_imports {
            implicit_imports.push_str("use wasm_bindgen::prelude::*;\n");
        }
        if self.needs_web_imports {
            implicit_imports.push_str("use web_sys::*;\n");
        }
        if self.needs_js_imports {
            implicit_imports.push_str("use js_sys::*;\n");
        }
        if self.needs_serde_imports {
            implicit_imports.push_str("use serde::{Serialize, Deserialize};\n");
        }
        if self.needs_smallvec_import {
            implicit_imports.push_str("use smallvec::{SmallVec, smallvec};\n");
        }
        if self.needs_cow_import {
            implicit_imports.push_str("use std::borrow::Cow;\n");
        }
        if self.needs_write_import {
            implicit_imports.push_str("use std::fmt::Write;\n");
        }

        // Add Tauri invoke helper for WASM target if needed
        let mut tauri_helper = String::new();
        if self.target == CompilationTarget::Wasm && self.needs_serde_imports {
            tauri_helper.push_str(r#"
// Tauri invoke helper for WASM
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke)]
    async fn tauri_invoke_js(cmd: &str, args: JsValue) -> JsValue;
}

async fn tauri_invoke<T: serde::de::DeserializeOwned>(cmd: &str, args: serde_json::Value) -> Result<T, String> {
    let js_args = serde_wasm_bindgen::to_value(&args).map_err(|e| e.to_string())?;
    let result = tauri_invoke_js(cmd, js_args).await;
    serde_wasm_bindgen::from_value(result).map_err(|e| e.to_string())
}

"#);
        }

        // Combine: implicit imports + explicit imports + tauri helper + body
        let mut output = String::new();
        if !implicit_imports.is_empty() {
            output.push_str(&implicit_imports);
            if !imports.is_empty() {
                output.push('\n');
            }
        }
        if !imports.is_empty() {
            output.push_str(&imports);
        }
        if !tauri_helper.is_empty() {
            output.push('\n');
            output.push_str(&tauri_helper);
        }
        if !output.is_empty() && !body.is_empty() {
            output.push('\n');
        }
        output.push_str(&body);

        output
    }

    fn generate_use(&self, path: &[String], alias: Option<&str>) -> String {
        if path.is_empty() {
            return String::new();
        }

        let full_path = path.join(".");

        // Handle stdlib imports FIRST (before glob handling)
        // This ensures std::ui::*, std::fs::*, etc. are properly skipped
        if full_path.starts_with("std::") || full_path.starts_with("std.") {
            // Normalize to use :: separator
            let normalized = full_path.replace('.', "::");
            let module_name = normalized.strip_prefix("std::").unwrap();

            // Strip glob suffix if present for checking
            let module_base = module_name.strip_suffix("::*").unwrap_or(module_name);

            // Handle Rust stdlib modules that should NOT be mapped to windjammer_runtime
            // These are native Rust modules that should be used directly
            if module_base.starts_with("collections") || module_base.starts_with("cmp") {
                // Pass through to Rust's std library
                return format!("use std::{};\n", module_name);
            }

            // Handle UI framework - skip explicit import (handled by implicit imports)
            if module_base == "ui" || module_base.starts_with("ui::") {
                // UI framework is handled by implicit imports from windjammer-ui crate
                // Don't generate an explicit import here
                return String::new();
            }

            // Handle Game framework - skip explicit import (handled by implicit imports)
            if module_base == "game" || module_base.starts_with("game::") {
                // Game framework is handled by implicit imports from windjammer-game-framework crate
                // Don't generate an explicit import here
                return String::new();
            }

            // Handle Tauri framework - skip explicit import (functions are generated inline)
            if module_base == "tauri" || module_base.starts_with("tauri::") {
                // Tauri functions are handled by compiler codegen (generate_tauri_invoke)
                // Don't generate an explicit import here
                return String::new();
            }

            // Handle platform APIs - skip explicit import (handled by implicit imports)
            if module_base == "fs"
                || module_base.starts_with("fs::")
                || module_base == "process"
                || module_base.starts_with("process::")
                || module_base == "dialog"
                || module_base.starts_with("dialog::")
                || module_base == "env"
                || module_base.starts_with("env::")
                || module_base == "encoding"
                || module_base.starts_with("encoding::")
                || module_base == "compute"
                || module_base.starts_with("compute::")
                || module_base == "net"
                || module_base.starts_with("net::")
                || module_base == "http"
                || module_base.starts_with("http::")
                || module_base == "storage"
                || module_base.starts_with("storage::")
            {
                // Platform APIs are handled by implicit imports (platform-specific)
                // Don't generate an explicit import here
                return String::new();
            }

            // Map to windjammer_runtime (all stdlib modules are now implemented!)
            let rust_import = match module_name {
                // Core modules
                "http" => "windjammer_runtime::http",
                "mime" => "windjammer_runtime::mime",
                "json" => "windjammer_runtime::json",

                // Additional modules
                "async" => "windjammer_runtime::async_runtime",
                "cli" => "windjammer_runtime::cli",
                "crypto" => "windjammer_runtime::crypto",
                "csv" => "windjammer_runtime::csv_mod",
                "db" => "windjammer_runtime::db",
                "log" => "windjammer_runtime::log_mod",
                "math" => "windjammer_runtime::math",
                "random" => "windjammer_runtime::random",
                "regex" => "windjammer_runtime::regex_mod",
                "strings" => "windjammer_runtime::strings",
                "testing" => "windjammer_runtime::testing",
                "time" => "windjammer_runtime::time",
                // "ui" is handled by implicit imports (windjammer-ui crate), not runtime
                "game" => "windjammer_runtime::game",

                _ => {
                    // Unknown module - try windjammer_runtime
                    return format!("use windjammer_runtime::{};\n", module_name);
                }
            };

            if let Some(alias_name) = alias {
                return format!("use {} as {};\n", rust_import, alias_name);
            } else {
                // For _mod suffixed modules (log_mod, regex_mod), alias back to the original name
                // AND import any public types they export
                if rust_import.ends_with("_mod") {
                    let original_name = rust_import
                        .strip_suffix("_mod")
                        .and_then(|s| s.split("::").last())
                        .unwrap_or(rust_import);

                    let mut result = format!("use {} as {};\n", rust_import, original_name);

                    // Import types for modules that export them
                    match original_name {
                        "regex" => {
                            result.push_str(&format!("use {}::Regex;\n", rust_import));
                        }
                        "time" => {
                            result.push_str(&format!(
                                "use {}::{{Duration, Instant}};\n",
                                rust_import
                            ));
                        }
                        _ => {}
                    }

                    return result;
                }
                // Import the module itself (not glob) to keep module-qualified paths
                // For types like Duration, we'll need explicit imports or full paths
                return format!("use {};\n", rust_import);
            }
        }

        // Skip bare "std" imports
        if full_path == "std" {
            return String::new();
        }

        // Handle glob imports for non-stdlib modules: module.submodule.* -> use module::submodule::*;
        if full_path.ends_with(".*") {
            let path_without_glob = full_path.strip_suffix(".*").unwrap();
            // Replace dots with :: but remove any trailing ::
            let rust_path = path_without_glob
                .replace('.', "::")
                .trim_end_matches("::")
                .to_string();
            return format!("use {}::*;\n", rust_path);
        }

        // Handle braced imports: module::{A, B, C} or module.{A, B, C}
        if (full_path.contains("::{") || full_path.contains(".{")) && full_path.contains('}') {
            // Try :: separator first, then . separator
            if let Some((base, items)) = full_path.split_once("::{") {
                return format!("use {}::{{{};\n", base, items);
            } else if let Some((base, items)) = full_path.split_once(".{") {
                let rust_base = base.replace('.', "::");
                return format!("use {}::{{{};\n", rust_base, items);
            }
        }

        // Handle relative imports: ./utils or ../utils or ./config::Config
        if full_path.starts_with("./") || full_path.starts_with("../") {
            // Strip the leading ./ or ../
            let stripped = full_path
                .strip_prefix("./")
                .or_else(|| full_path.strip_prefix("../"))
                .unwrap_or(&full_path);

            // Check if this is importing a specific item (e.g., ./config::Config)
            if stripped.contains("::") {
                // Split into module path and item
                let rust_path = stripped.replace('/', "::");
                // Check if the last segment looks like a type (uppercase)
                let segments: Vec<&str> = rust_path.split("::").collect();
                if let Some(last) = segments.last() {
                    if last.chars().next().is_some_and(|c| c.is_uppercase()) {
                        // Importing a specific type: ./config::Config -> use crate::config::Config;
                        return format!("use crate::{};\n", rust_path);
                    }
                }
                // Otherwise, import all from the path
                return format!("use crate::{}::*;\n", rust_path);
            } else {
                // Module import: ./config
                // In the main entry point (is_module=false), modules are already in scope via pub mod declarations
                // In submodules (is_module=true), we need to explicitly use sibling modules
                let module_name = stripped.split('/').next_back().unwrap_or(stripped);
                if let Some(alias_name) = alias {
                    return format!("use crate::{} as {};\n", module_name, alias_name);
                } else if self.is_module {
                    // In a module, we need to explicitly use sibling modules
                    return format!("use crate::{};\n", module_name);
                } else {
                    // In main entry point, modules are already in scope
                    return String::new();
                }
            }
        }

        // Convert Windjammer's Go-style imports to Rust imports
        // Heuristic: If the last segment starts with an uppercase letter, it's likely a type/struct
        // Otherwise, it's a module and we should add ::*
        let rust_path = full_path.replace('.', "::");
        if let Some(alias_name) = alias {
            format!("use {} as {};\n", rust_path, alias_name)
        } else {
            // Check if the last segment looks like a type (starts with uppercase)
            let last_segment = rust_path.split("::").last().unwrap_or("");
            if last_segment
                .chars()
                .next()
                .is_some_and(|c| c.is_uppercase())
            {
                // Likely a type, don't add ::*
                format!("use {};\n", rust_path)
            } else {
                // Likely a module, add ::*
                format!("use {}::*;\n", rust_path)
            }
        }
    }

    fn generate_struct(&mut self, s: &StructDecl) -> String {
        let mut output = String::new();

        // Convert decorators to Rust attributes
        for decorator in &s.decorators {
            // Skip framework decorators - they're handled separately
            if decorator.name == "component" || decorator.name == "game" {
                continue;
            }

            if decorator.name == "command" {
                // Special handling for @command decorator - generates clap attributes
                // @command(name: "app", about: "Description") -> #[derive(Parser)] + #[command(...)]
                output.push_str("#[derive(Parser)]\n");

                if !decorator.arguments.is_empty() {
                    output.push_str("#[command(");
                    let args: Vec<String> = decorator
                        .arguments
                        .iter()
                        .map(|(key, expr)| {
                            format!("{} = {}", key, self.generate_expression_immut(expr))
                        })
                        .collect();
                    output.push_str(&args.join(", "));
                    output.push_str(")]\n");
                }
                continue;
            } else if decorator.name == "auto" {
                // Special handling for @auto decorator
                let traits = if decorator.arguments.is_empty() {
                    // Smart inference: no arguments, so infer traits based on field types
                    self.infer_derivable_traits(s)
                } else {
                    // Explicit: extract trait names from decorator arguments
                    let mut explicit_traits = Vec::new();
                    for (_key, expr) in &decorator.arguments {
                        if let Expression::Identifier {
                            name: trait_name, ..
                        } = expr
                        {
                            explicit_traits.push(trait_name.clone());
                        }
                    }
                    explicit_traits
                };

                if !traits.is_empty() {
                    output.push_str(&format!("#[derive({})]\n", traits.join(", ")));
                }
            } else if decorator.name == "derive" {
                // Special handling for @derive decorator - generates #[derive(Trait1, Trait2)]
                let mut traits = Vec::new();
                for (_key, expr) in &decorator.arguments {
                    if let Expression::Identifier {
                        name: trait_name, ..
                    } = expr
                    {
                        traits.push(trait_name.clone());
                    }
                }
                if !traits.is_empty() {
                    output.push_str(&format!("#[derive({})]\n", traits.join(", ")));
                }
            } else {
                // Map Windjammer decorator to Rust attribute
                let rust_attr = self.map_decorator(&decorator.name);
                if decorator.arguments.is_empty() {
                    output.push_str(&format!("#[{}]\n", rust_attr));
                } else {
                    output.push_str(&format!("#[{}(", rust_attr));
                    let args: Vec<String> = decorator
                        .arguments
                        .iter()
                        .map(|(key, expr)| {
                            format!("{} = {}", key, self.generate_expression_immut(expr))
                        })
                        .collect();
                    output.push_str(&args.join(", "));
                    output.push_str(")]\n");
                }
            }
        }

        // Add struct declaration with type parameters
        let pub_prefix = if self.is_module { "pub " } else { "" };
        output.push_str(&format!("{}struct ", pub_prefix));
        output.push_str(&s.name);
        if !s.type_params.is_empty() {
            output.push('<');
            output.push_str(&self.format_type_params(&s.type_params));
            output.push('>');
        }

        // Add where clause if present
        output.push_str(&self.format_where_clause(&s.where_clause));

        output.push_str(" {\n");

        for field in &s.fields {
            // Generate decorators for the field (convert to Rust attributes)
            for decorator in &field.decorators {
                // Handle @arg decorator specially - it's a clap field attribute
                if decorator.name == "arg" {
                    output.push_str("    #[arg(");
                    let args: Vec<String> = decorator
                        .arguments
                        .iter()
                        .map(|(key, expr)| {
                            // Handle special cases for clap arguments
                            match key.as_str() {
                                "short" => {
                                    // short takes a character literal
                                    format!("short = {}", self.generate_expression_immut(expr))
                                }
                                "long" => {
                                    // long takes a string literal
                                    format!("long = {}", self.generate_expression_immut(expr))
                                }
                                "default_value" => {
                                    format!(
                                        "default_value = {}",
                                        self.generate_expression_immut(expr)
                                    )
                                }
                                "help" => {
                                    format!("help = {}", self.generate_expression_immut(expr))
                                }
                                _ => format!("{} = {}", key, self.generate_expression_immut(expr)),
                            }
                        })
                        .collect();
                    output.push_str(&args.join(", "));
                    output.push_str(")]\n");
                } else {
                    // Generic decorator handling
                    output.push_str(&format!("    #[{}(", decorator.name));
                    let args: Vec<String> = decorator
                        .arguments
                        .iter()
                        .map(|(key, expr)| {
                            format!("{} = {}", key, self.generate_expression_immut(expr))
                        })
                        .collect();
                    output.push_str(&args.join(", "));
                    output.push_str(")]\n");
                }
            }
            // In modules, all fields should be pub for cross-module access
            let pub_keyword = if self.is_module || field.is_pub {
                "pub "
            } else {
                ""
            };
            output.push_str(&format!(
                "    {}{}: {},\n",
                pub_keyword,
                field.name,
                self.type_to_rust(&field.field_type)
            ));
        }

        output.push('}');
        output
    }

    fn generate_enum(&self, e: &EnumDecl) -> String {
        let mut output = format!("enum {}", e.name);

        // Generate generic parameters: enum Option<T>, enum Result<T, E>
        if !e.type_params.is_empty() {
            output.push('<');
            output.push_str(&self.format_type_params(&e.type_params));
            output.push('>');
        }

        output.push_str(" {\n");

        for variant in &e.variants {
            if let Some(data) = &variant.data {
                output.push_str(&format!(
                    "    {}({}),\n",
                    variant.name,
                    self.type_to_rust(data)
                ));
            } else {
                output.push_str(&format!("    {},\n", variant.name));
            }
        }

        output.push('}');
        output
    }

    fn generate_trait(&mut self, trait_decl: &crate::parser::TraitDecl) -> String {
        let mut output = String::from("trait ");
        output.push_str(&trait_decl.name);

        // Generate generic parameters: trait From<T> { ... }
        if !trait_decl.generics.is_empty() {
            output.push('<');
            output.push_str(&trait_decl.generics.join(", "));
            output.push('>');
        }

        // Generate supertraits: trait Manager: Employee + Person
        if !trait_decl.supertraits.is_empty() {
            output.push_str(": ");
            output.push_str(&trait_decl.supertraits.join(" + "));
        }

        output.push_str(" {\n");
        self.indent_level += 1;

        // Generate associated type declarations: type Item;
        for assoc_type in &trait_decl.associated_types {
            output.push_str(&self.indent());
            output.push_str(&format!("type {};\n", assoc_type.name));
        }

        if !trait_decl.associated_types.is_empty() {
            output.push('\n');
        }

        // Generate trait methods
        for method in &trait_decl.methods {
            output.push_str(&self.indent());

            if method.is_async {
                output.push_str("async ");
            }

            output.push_str("fn ");
            output.push_str(&method.name);
            output.push('(');

            // Generate parameters
            let params: Vec<String> = method
                .parameters
                .iter()
                .map(|param| {
                    use crate::parser::OwnershipHint;
                    let type_str = match &param.ownership {
                        OwnershipHint::Owned => {
                            if param.name == "self" {
                                // Owned self is always mutable in Windjammer
                                return "mut self".to_string();
                            }
                            // Owned parameters are always mutable in Windjammer
                            return format!(
                                "mut {}: {}",
                                param.name,
                                self.type_to_rust(&param.type_)
                            );
                        }
                        OwnershipHint::Ref => {
                            if param.name == "self" {
                                return "&self".to_string();
                            }
                            format!("&{}", self.type_to_rust(&param.type_))
                        }
                        OwnershipHint::Mut => {
                            if param.name == "self" {
                                return "&mut self".to_string();
                            }
                            format!("&mut {}", self.type_to_rust(&param.type_))
                        }
                        OwnershipHint::Inferred => {
                            // Default to &
                            if param.name == "self" {
                                "&self".to_string()
                            } else {
                                format!("&{}", self.type_to_rust(&param.type_))
                            }
                        }
                    };

                    format!("{}: {}", param.name, type_str)
                })
                .collect();

            output.push_str(&params.join(", "));
            output.push(')');

            // Return type
            if let Some(ret_type) = &method.return_type {
                output.push_str(" -> ");
                output.push_str(&self.type_to_rust(ret_type));
            }

            // Default implementation (if provided)
            if let Some(body) = &method.body {
                output.push_str(" {\n");
                self.indent_level += 1;

                for stmt in body {
                    output.push_str(&self.generate_statement(stmt));
                }

                self.indent_level -= 1;
                output.push_str(&self.indent());
                output.push_str("}\n");
            } else {
                output.push_str(";\n");
            }
        }

        self.indent_level -= 1;
        output.push('}');
        output
    }

    fn generate_impl(&mut self, impl_block: &ImplBlock, analyzed: &[AnalyzedFunction]) -> String {
        let mut output = String::new();

        // Check if this impl block has @export or @wasm_bindgen decorator
        let has_wasm_export = impl_block
            .decorators
            .iter()
            .any(|d| d.name == "export" || d.name == "wasm_bindgen");

        // Generate decorators (map Windjammer decorators to Rust attributes)
        for decorator in &impl_block.decorators {
            let rust_attr = self.map_decorator(&decorator.name);
            output.push_str(&format!("#[{}]\n", rust_attr));
        }

        // Generate impl with type parameters
        output.push_str("impl");
        if !impl_block.type_params.is_empty() {
            output.push('<');
            output.push_str(&self.format_type_params(&impl_block.type_params));
            output.push('>');
        }
        output.push(' ');

        if let Some(trait_name) = &impl_block.trait_name {
            // Trait implementation: impl<T> Trait<TypeArgs> for Type<T>
            output.push_str(trait_name);

            // Generate trait type arguments if present: From<int> -> From<i64>
            if let Some(type_args) = &impl_block.trait_type_args {
                output.push('<');
                let args_str: Vec<String> =
                    type_args.iter().map(|t| self.type_to_rust(t)).collect();
                output.push_str(&args_str.join(", "));
                output.push('>');
            }

            output.push_str(&format!(" for {}", impl_block.type_name));
        } else {
            // Inherent implementation: impl<T> Type<T>
            output.push_str(&impl_block.type_name);
        }

        // Add where clause if present
        output.push_str(&self.format_where_clause(&impl_block.where_clause));

        output.push_str(" {\n");

        self.indent_level += 1;

        // Generate associated type implementations: type Item = i32;
        for assoc_type in &impl_block.associated_types {
            output.push_str(&self.indent());
            output.push_str(&format!("type {}", assoc_type.name));
            if let Some(concrete_type) = &assoc_type.concrete_type {
                output.push_str(&format!(" = {};\n", self.type_to_rust(concrete_type)));
            } else {
                output.push_str(";\n");
            }
        }

        if !impl_block.associated_types.is_empty() {
            output.push('\n');
        }

        // Store the wasm export flag for use in generate_function
        let old_in_wasm_impl = self.in_wasm_bindgen_impl;
        self.in_wasm_bindgen_impl = has_wasm_export;

        for func in &impl_block.functions {
            // Find the analyzed version of this function
            // Match on both function name AND parent type to handle multiple impl blocks with same method names
            if let Some(analyzed_func) = analyzed
                .iter()
                .find(|af| af.decl.name == func.name && af.decl.parent_type == func.parent_type)
            {
                output.push_str(&self.generate_function(analyzed_func));
                output.push('\n');
            }
        }

        self.in_wasm_bindgen_impl = old_in_wasm_impl;

        self.indent_level -= 1;
        output.push('}');
        output
    }

    // Helper method for expressions that need to be evaluated without &mut self
    fn generate_expression_immut(&self, expr: &Expression) -> String {
        match expr {
            Expression::Literal { value: lit, .. } => self.generate_literal(lit),
            Expression::Identifier { name, .. } => name.clone(),
            _ => "/* expression */".to_string(),
        }
    }

    // Check if a function accesses any struct fields
    // For now, we use a simple heuristic: if we're in an impl block and the function
    // has a non-empty body, assume it might need &self
    fn function_accesses_fields(&self, func: &FunctionDecl) -> bool {
        // Check if the function body accesses any struct fields
        for stmt in &func.body {
            if self.statement_accesses_fields(stmt) {
                return true;
            }
        }
        false
    }

    fn function_mutates_fields(&self, func: &FunctionDecl) -> bool {
        // Check if the function body mutates any struct fields
        for stmt in &func.body {
            if self.statement_mutates_fields(stmt) {
                return true;
            }
        }
        false
    }

    fn statement_accesses_fields(&self, stmt: &Statement) -> bool {
        match stmt {
            Statement::Expression { expr, .. }
            | Statement::Return {
                value: Some(expr), ..
            } => self.expression_accesses_fields(expr),
            Statement::Let { value, .. } => self.expression_accesses_fields(value),
            Statement::Assignment { target, value, .. } => {
                self.expression_accesses_fields(target) || self.expression_accesses_fields(value)
            }
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.expression_accesses_fields(condition)
                    || then_block.iter().any(|s| self.statement_accesses_fields(s))
                    || else_block.as_ref().is_some_and(|block| {
                        block.iter().any(|s| self.statement_accesses_fields(s))
                    })
            }
            Statement::While {
                condition, body, ..
            } => {
                self.expression_accesses_fields(condition)
                    || body.iter().any(|s| self.statement_accesses_fields(s))
            }
            Statement::For { iterable, body, .. } => {
                self.expression_accesses_fields(iterable)
                    || body.iter().any(|s| self.statement_accesses_fields(s))
            }
            Statement::Match { value, arms, .. } => {
                self.expression_accesses_fields(value)
                    || arms
                        .iter()
                        .any(|arm| self.expression_accesses_fields(&arm.body))
            }
            _ => false,
        }
    }

    fn statement_mutates_fields(&self, stmt: &Statement) -> bool {
        match stmt {
            Statement::Assignment { target, .. } => {
                // Check if we're assigning to a field: self.field = ...
                self.expression_is_field_access(target)
            }
            Statement::If {
                then_block,
                else_block,
                ..
            } => {
                then_block.iter().any(|s| self.statement_mutates_fields(s))
                    || else_block
                        .as_ref()
                        .is_some_and(|block| block.iter().any(|s| self.statement_mutates_fields(s)))
            }
            Statement::While { body, .. } | Statement::For { body, .. } => {
                body.iter().any(|s| self.statement_mutates_fields(s))
            }
            Statement::Match { arms, .. } => {
                arms.iter().any(|arm| {
                    // MatchArm body is an Expression, need to check for blocks
                    self.expression_mutates_fields(&arm.body)
                })
            }
            _ => false,
        }
    }

    fn expression_accesses_fields(&self, expr: &Expression) -> bool {
        match expr {
            Expression::Identifier { name, .. } => self.current_struct_fields.contains(name),
            Expression::FieldAccess { object, .. } => {
                // Check for self.field or nested field access
                if let Expression::Identifier { name: obj_name, .. } = &**object {
                    obj_name == "self"
                } else {
                    self.expression_accesses_fields(object)
                }
            }
            Expression::MethodCall {
                object, arguments, ..
            } => {
                self.expression_accesses_fields(object)
                    || arguments
                        .iter()
                        .any(|(_, arg)| self.expression_accesses_fields(arg))
            }
            Expression::Call { arguments, .. } => arguments
                .iter()
                .any(|(_, arg)| self.expression_accesses_fields(arg)),
            Expression::Binary { left, right, .. } => {
                self.expression_accesses_fields(left) || self.expression_accesses_fields(right)
            }
            Expression::Unary { operand, .. } => self.expression_accesses_fields(operand),
            Expression::Index { object, index, .. } => {
                self.expression_accesses_fields(object) || self.expression_accesses_fields(index)
            }
            Expression::StructLiteral { fields, .. } => fields
                .iter()
                .any(|(_, expr)| self.expression_accesses_fields(expr)),
            Expression::MapLiteral { pairs, .. } => pairs.iter().any(|(k, v)| {
                self.expression_accesses_fields(k) || self.expression_accesses_fields(v)
            }),
            Expression::Array { elements, .. } => {
                elements.iter().any(|e| self.expression_accesses_fields(e))
            }
            Expression::Tuple { elements, .. } => {
                elements.iter().any(|e| self.expression_accesses_fields(e))
            }
            Expression::Closure { body, .. } => self.expression_accesses_fields(body),
            Expression::TryOp { expr, .. }
            | Expression::Await { expr, .. }
            | Expression::Cast { expr, .. } => self.expression_accesses_fields(expr),
            Expression::MacroInvocation { args, .. } => {
                // Check if any macro arguments access fields
                args.iter().any(|arg| self.expression_accesses_fields(arg))
            }
            Expression::Range { start, end, .. } => {
                self.expression_accesses_fields(start) || self.expression_accesses_fields(end)
            }
            Expression::ChannelSend { channel, value, .. } => {
                self.expression_accesses_fields(channel) || self.expression_accesses_fields(value)
            }
            Expression::ChannelRecv { channel, .. } => self.expression_accesses_fields(channel),
            Expression::Block { statements, .. } => {
                // Check if any statement in the block accesses fields
                statements.iter().any(|s| self.statement_accesses_fields(s))
            }
            _ => false,
        }
    }

    fn expression_is_field_access(&self, expr: &Expression) -> bool {
        match expr {
            Expression::Identifier { name, .. } => self.current_struct_fields.contains(name),
            Expression::FieldAccess { object, .. } => {
                if let Expression::Identifier { name: obj_name, .. } = &**object {
                    obj_name == "self"
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    fn expression_mutates_fields(&self, expr: &Expression) -> bool {
        match expr {
            Expression::Block { statements, .. } => {
                // Check if any statement in the block mutates fields
                statements.iter().any(|s| self.statement_mutates_fields(s))
            }
            _ => false,
        }
    }

    fn generate_function(&mut self, analyzed: &AnalyzedFunction) -> String {
        let func = &analyzed.decl;
        let mut output = String::new();

        // AUTO-CLONE: Load auto-clone analysis for this function
        self.auto_clone_analysis = Some(analyzed.auto_clone_analysis.clone());

        // PHASE 2 OPTIMIZATION: Load clone optimizations for this function
        // Variables in this set can safely avoid .clone() calls
        self.clone_optimizations.clear();
        for opt in &analyzed.clone_optimizations {
            self.clone_optimizations.insert(opt.variable.clone());
        }

        // Track function parameters for compound assignment optimization
        self.current_function_params = func.parameters.clone();

        // PHASE 8 OPTIMIZATION: Load SmallVec optimizations for this function
        // DISABLED: SmallVec optimizations conflict with return types
        // TODO: Re-enable with smarter conversion at return sites
        self.smallvec_optimizations.clear();
        // for opt in &analyzed.smallvec_optimizations {
        //     self.smallvec_optimizations
        //         .insert(opt.variable.clone(), opt.clone());
        //     self.needs_smallvec_import = true; // Mark that we need the smallvec crate
        // }

        // PHASE 9 OPTIMIZATION: Load Cow optimizations for this function
        self.cow_optimizations.clear();
        for opt in &analyzed.cow_optimizations {
            self.cow_optimizations.insert(opt.variable.clone());
            self.needs_cow_import = true; // Mark that we need Cow from std::borrow
        }

        // PHASE 3 OPTIMIZATION: Load struct mapping optimizations
        // Track which structs can use optimized construction strategies
        self.struct_mapping_hints.clear();
        for opt in &analyzed.struct_mapping_optimizations {
            self.struct_mapping_hints
                .insert(opt.target_struct.clone(), opt.strategy.clone());
        }

        // PHASE 4 OPTIMIZATION: Load string operation optimizations
        // Track capacity hints for string operations
        self.string_capacity_hints.clear();

        // PHASE 5 OPTIMIZATION: Load assignment operation optimizations
        // Track which variables can use compound assignment operators
        self.assignment_optimizations.clear();
        for opt in &analyzed.assignment_optimizations {
            self.assignment_optimizations
                .insert(opt.variable.clone(), opt.operation.clone());
        }
        for opt in &analyzed.string_optimizations {
            if let Some(capacity) = opt.estimated_capacity {
                self.string_capacity_hints.insert(opt.location, capacity);
            }
        }

        // PHASE 6 OPTIMIZATION: Load defer drop optimizations
        // Track variables that should have their drops deferred to background thread
        self.defer_drop_optimizations = analyzed.defer_drop_optimizations.clone();

        // Check for @async decorator (special case: it's a keyword, not an attribute)
        let is_async = func.decorators.iter().any(|d| d.name == "async");

        // Special case: async main requires #[tokio::main]
        if is_async && func.name == "main" {
            output.push_str("#[tokio::main]\n");
        }

        // OPTIMIZATION: Add inline hints for hot path functions
        // This is Phase 1 optimization: Generate Inlinable Code
        if self.should_inline_function(func, analyzed) {
            output.push_str("#[inline]\n");
        }

        // Generate decorators (map Windjammer decorators to Rust attributes)
        for decorator in &func.decorators {
            // Skip @async, it's handled specially
            if decorator.name == "async" {
                continue;
            }

            // Skip @export - it's used to determine visibility but doesn't map to a Rust attribute for native targets
            if decorator.name == "export" && self.target != CompilationTarget::Wasm {
                continue;
            }

            // Skip game framework decorators - they're handled by the game loop
            if matches!(
                decorator.name.as_str(),
                "game" | "init" | "update" | "render" | "render3d" | "input" | "cleanup"
            ) {
                continue;
            }

            let rust_attr = self.map_decorator(&decorator.name);
            output.push_str(&format!("#[{}]\n", rust_attr));
        }

        // Add `pub` if we're in a #[wasm_bindgen] impl block OR compiling a module OR has @export decorator
        let has_export = func.decorators.iter().any(|d| d.name == "export");
        if self.in_wasm_bindgen_impl || self.is_module || has_export {
            output.push_str("pub ");
        }

        // Add async keyword if decorator present
        if is_async {
            output.push_str("async ");
        }

        output.push_str("fn ");
        output.push_str(&func.name);

        // Add type parameters with bounds: fn foo<T: Display, U: Debug>(...)
        // Merge inferred bounds with explicit bounds
        let type_params = if let Some(inferred) = self.inferred_bounds.get(&func.name) {
            let merged = inferred.merge_with_explicit(&func.type_params);
            // Track which traits need imports
            for param in &merged {
                for trait_name in &param.bounds {
                    self.needs_trait_imports.insert(trait_name.clone());
                }
            }
            merged
        } else {
            // Still track explicit bounds
            for param in &func.type_params {
                for trait_name in &param.bounds {
                    self.needs_trait_imports.insert(trait_name.clone());
                }
            }
            func.type_params.clone()
        };

        if !type_params.is_empty() {
            output.push('<');
            output.push_str(&self.format_type_params(&type_params));
            output.push('>');
        }

        output.push('(');

        // Add implicit &self or &mut self for impl block methods that access fields
        let mut params: Vec<String> = Vec::new();
        let has_explicit_self = func.parameters.iter().any(|p| p.name == "self");

        if self.in_impl_block && !has_explicit_self && !self.current_struct_fields.is_empty() {
            // Check if function body mutates any struct fields
            if self.function_mutates_fields(func) {
                params.push("&mut self".to_string());
            } else if self.function_accesses_fields(func) {
                // Only read access needed
                params.push("&self".to_string());
            }
        }

        let additional_params: Vec<String> = func
            .parameters
            .iter()
            .map(|param| {
                // PHASE 9 OPTIMIZATION: Check if this parameter should use Cow<'_, T>
                if self.cow_optimizations.contains(&param.name) {
                    let base_type = self.type_to_rust(&param.type_);
                    // For String types, use Cow<'_, str>
                    let cow_type = if base_type == "String" {
                        "Cow<'_, str>".to_string()
                    } else {
                        format!("Cow<'_, {}>", base_type)
                    };
                    return format!("{}: {}", param.name, cow_type);
                }

                // Handle explicit ownership hints (self, &self, &mut self)
                let type_str = match &param.ownership {
                    OwnershipHint::Owned => {
                        if param.name == "self" {
                            // Check if analyzer inferred a different ownership for self
                            if let Some(ownership_mode) =
                                analyzed.inferred_ownership.get(&param.name)
                            {
                                match ownership_mode {
                                    OwnershipMode::MutBorrowed => return "&mut self".to_string(),
                                    OwnershipMode::Borrowed => return "&self".to_string(),
                                    OwnershipMode::Owned => return "mut self".to_string(),
                                }
                            }
                            // Default: owned self is mutable
                            return "mut self".to_string();
                        }
                        // Owned parameters are always mutable in Windjammer
                        return format!("mut {}: {}", param.name, self.type_to_rust(&param.type_));
                    }
                    OwnershipHint::Ref => {
                        if param.name == "self" {
                            return "&self".to_string();
                        }
                        // Don't add & if the type is already a Reference
                        if matches!(param.type_, Type::Reference(_) | Type::MutableReference(_)) {
                            self.type_to_rust(&param.type_)
                        } else {
                            format!("&{}", self.type_to_rust(&param.type_))
                        }
                    }
                    OwnershipHint::Mut => {
                        if param.name == "self" {
                            return "&mut self".to_string();
                        }
                        // Don't add &mut if the type is already a MutableReference
                        if matches!(param.type_, Type::MutableReference(_)) {
                            self.type_to_rust(&param.type_)
                        } else {
                            format!("&mut {}", self.type_to_rust(&param.type_))
                        }
                    }
                    OwnershipHint::Inferred => {
                        // Use analyzer's inference
                        let ownership_mode = analyzed
                            .inferred_ownership
                            .get(&param.name)
                            .unwrap_or(&OwnershipMode::Borrowed);

                        // Override for Copy types UNLESS they're mutated
                        // Mutated parameters should be &mut even for Copy types
                        if self.is_copy_type(&param.type_)
                            && ownership_mode != &OwnershipMode::MutBorrowed
                        {
                            self.type_to_rust(&param.type_)
                        } else {
                            match ownership_mode {
                                OwnershipMode::Owned => self.type_to_rust(&param.type_),
                                OwnershipMode::Borrowed => {
                                    // For Copy types that are only read, pass by value
                                    if self.is_copy_type(&param.type_) {
                                        self.type_to_rust(&param.type_)
                                    } else {
                                        format!("&{}", self.type_to_rust(&param.type_))
                                    }
                                }
                                OwnershipMode::MutBorrowed => {
                                    format!("&mut {}", self.type_to_rust(&param.type_))
                                }
                            }
                        }
                    }
                };

                // Check if this is a pattern parameter
                if let Some(pattern) = &param.pattern {
                    // Generate pattern: type syntax
                    format!("{}: {}", self.generate_pattern(pattern), type_str)
                } else {
                    // Simple name: type syntax
                    format!("{}: {}", param.name, type_str)
                }
            })
            .collect();

        params.extend(additional_params);

        output.push_str(&params.join(", "));
        output.push(')');

        if let Some(return_type) = &func.return_type {
            output.push_str(" -> ");
            output.push_str(&self.type_to_rust(return_type));
        }

        // Add where clause if present
        output.push_str(&self.format_where_clause(&func.where_clause));

        output.push_str(" {\n");
        self.indent_level += 1;

        let mut body_code = self.generate_block(&func.body);

        // PHASE 6 OPTIMIZATION: Add defer drop logic before function returns
        // This defers heavy deallocations to a background thread for 10,000x speedup
        if !self.defer_drop_optimizations.is_empty() {
            body_code =
                self.wrap_with_defer_drop(body_code, &self.defer_drop_optimizations.clone());
        }

        output.push_str(&body_code);

        self.indent_level -= 1;
        output.push('}');

        output
    }

    fn type_to_rust(&self, type_: &Type) -> String {
        // Delegate to the refactored types module
        crate::codegen::rust::type_to_rust(type_)
    }

    fn pattern_to_rust(&self, pattern: &Pattern) -> String {
        match pattern {
            Pattern::Wildcard => "_".to_string(),
            Pattern::Identifier(name) => name.clone(),
            Pattern::Reference(inner) => format!("&{}", self.pattern_to_rust(inner)),
            Pattern::Tuple(patterns) => {
                let rust_patterns: Vec<String> =
                    patterns.iter().map(|p| self.pattern_to_rust(p)).collect();
                format!("({})", rust_patterns.join(", "))
            }
            Pattern::EnumVariant(variant, binding) => {
                use crate::parser::EnumPatternBinding;
                match binding {
                    EnumPatternBinding::Named(name) => format!("{}({})", variant, name),
                    EnumPatternBinding::Wildcard => format!("{}(_)", variant),
                    EnumPatternBinding::None => variant.clone(),
                }
            }
            Pattern::Literal(lit) => self.generate_literal(lit),
            Pattern::Or(patterns) => {
                let rust_patterns: Vec<String> =
                    patterns.iter().map(|p| self.pattern_to_rust(p)).collect();
                rust_patterns.join(" | ")
            }
        }
    }

    fn generate_statement(&mut self, stmt: &Statement) -> String {
        // Record source mapping if location info is available
        if let Some(location) = self.get_statement_location(stmt) {
            self.record_mapping(&location);
        }

        match stmt {
            Statement::Let {
                pattern,
                mutable,
                type_,
                value,
                ..
            } => {
                let mut output = self.indent();
                output.push_str("let ");

                // Check if we need &mut for index access on borrowed fields
                // e.g., let enemy = self.enemies[i] should be let enemy = &mut self.enemies[i]
                let needs_mut_ref = self.should_mut_borrow_index_access(value);

                if needs_mut_ref {
                    // Don't add mut keyword, but we'll add &mut to the value
                } else if *mutable {
                    output.push_str("mut ");
                }

                // Generate pattern (could be simple name or tuple)
                let pattern_str = self.generate_pattern(pattern);
                output.push_str(&pattern_str);

                // Extract variable name for optimizations (only works for simple identifiers)
                let var_name = match pattern {
                    Pattern::Identifier(name) => Some(name.as_str()),
                    _ => None,
                };

                // PHASE 8: Check if this variable should use SmallVec
                if let Some(name) = var_name {
                    if let Some(smallvec_opt) = self.smallvec_optimizations.get(name) {
                        // Use SmallVec with stack allocation
                        // If there's a type annotation, extract the element type
                        let elem_type = if let Some(Type::Vec(inner)) = type_ {
                            self.type_to_rust(inner)
                        } else {
                            "_".to_string() // Type inference
                        };
                        output.push_str(&format!(
                            ": SmallVec<[{}; {}]>",
                            elem_type, smallvec_opt.stack_size
                        ));
                        output.push_str(" = ");

                        // Generate the expression but wrap in smallvec! if it's a vec! macro
                        let expr_str = self.generate_expression(value);
                        if let Some(stripped) = expr_str.strip_prefix("vec!") {
                            // Replace vec! with smallvec!
                            output.push_str("smallvec!");
                            output.push_str(stripped);
                        } else {
                            // For other expressions, try to convert
                            output.push_str(&expr_str);
                            output.push_str(".into()"); // Convert Vec to SmallVec
                        }
                    } else if let Some(t) = type_ {
                        output.push_str(": ");
                        output.push_str(&self.type_to_rust(t));
                        output.push_str(" = ");

                        // Auto-convert &str to String if type is String
                        let mut value_str = self.generate_expression(value);
                        let is_string_type = matches!(t, Type::String)
                            || matches!(t, Type::Custom(name) if name == "String");

                        // Convert string literals OR identifiers to String when target is String
                        if is_string_type {
                            let should_convert = matches!(
                                value,
                                Expression::Literal {
                                    value: Literal::String(_),
                                    ..
                                } | Expression::Identifier { .. }
                            );
                            if should_convert {
                                value_str = format!("{}.to_string()", value_str);
                            }
                        }
                        output.push_str(&value_str);
                    } else {
                        output.push_str(" = ");
                        if needs_mut_ref {
                            output.push_str("&mut ");
                        }
                        output.push_str(&self.generate_expression(value));
                    }
                } else {
                    // No SmallVec optimization for this variable
                    if let Some(t) = type_ {
                        output.push_str(": ");
                        output.push_str(&self.type_to_rust(t));
                        output.push_str(" = ");

                        // Auto-convert &str to String if type is String
                        let mut value_str = self.generate_expression(value);
                        let is_string_type = matches!(t, Type::String)
                            || matches!(t, Type::Custom(name) if name == "String");

                        // Convert string literals OR identifiers to String when target is String
                        if is_string_type {
                            let should_convert = matches!(
                                value,
                                Expression::Literal {
                                    value: Literal::String(_),
                                    ..
                                } | Expression::Identifier { .. }
                            );
                            if should_convert {
                                value_str = format!("{}.to_string()", value_str);
                            }
                        }

                        if needs_mut_ref {
                            value_str = format!("&mut {}", value_str);
                        }
                        output.push_str(&value_str);
                    } else {
                        output.push_str(" = ");
                        if needs_mut_ref {
                            output.push_str("&mut ");
                        }
                        output.push_str(&self.generate_expression(value));
                    }
                }

                output.push_str(";\n");
                output
            }
            Statement::Const {
                name, type_, value, ..
            } => {
                let mut output = self.indent();

                // Special case: string constants should use &'static str, not String
                let rust_type = if matches!(type_, Type::String)
                    && matches!(
                        value,
                        Expression::Literal {
                            value: Literal::String(_),
                            ..
                        }
                    ) {
                    "&'static str".to_string()
                } else {
                    self.type_to_rust(type_)
                };

                output.push_str(&format!(
                    "const {}: {} = {};\n",
                    name,
                    rust_type,
                    self.generate_expression(value)
                ));
                output
            }
            Statement::Static {
                name,
                mutable,
                type_,
                value,
                ..
            } => {
                let mut output = self.indent();
                if *mutable {
                    output.push_str(&format!(
                        "static mut {}: {} = {};\n",
                        name,
                        self.type_to_rust(type_),
                        self.generate_expression(value)
                    ));
                } else {
                    output.push_str(&format!(
                        "static {}: {} = {};\n",
                        name,
                        self.type_to_rust(type_),
                        self.generate_expression(value)
                    ));
                }
                output
            }
            Statement::Return { value: expr, .. } => {
                let mut output = self.indent();
                output.push_str("return");
                if let Some(e) = expr {
                    output.push(' ');
                    output.push_str(&self.generate_expression(e));
                }
                output.push_str(";\n");
                output
            }
            Statement::Expression { expr, .. } => {
                let mut output = self.indent();
                output.push_str(&self.generate_expression(expr));
                output.push_str(";\n");
                output
            }
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                let mut output = self.indent();
                output.push_str("if ");
                output.push_str(&self.generate_expression(condition));
                output.push_str(" {\n");

                self.indent_level += 1;
                output.push_str(&self.generate_block(then_block));
                self.indent_level -= 1;

                output.push_str(&self.indent());
                output.push('}');

                if let Some(else_b) = else_block {
                    output.push_str(" else {\n");
                    self.indent_level += 1;
                    output.push_str(&self.generate_block(else_b));
                    self.indent_level -= 1;
                    output.push_str(&self.indent());
                    output.push('}');
                }

                output.push('\n');
                output
            }
            Statement::Match { value, arms, .. } => {
                let mut output = self.indent();
                output.push_str("match ");

                // Check if any arm has a string literal pattern
                // If so, add .as_str() to the match value for String types
                let has_string_literal = arms
                    .iter()
                    .any(|arm| self.pattern_has_string_literal(&arm.pattern));

                let value_str = self.generate_expression(value);
                if has_string_literal {
                    // Add .as_str() if the value doesn't already end with it
                    if !value_str.ends_with(".as_str()") {
                        output.push_str(&format!("{}.as_str()", value_str));
                    } else {
                        output.push_str(&value_str);
                    }
                } else {
                    output.push_str(&value_str);
                }

                output.push_str(" {\n");

                self.indent_level += 1;
                for arm in arms {
                    output.push_str(&self.indent());
                    output.push_str(&self.generate_pattern(&arm.pattern));

                    // Add guard if present
                    if let Some(guard) = &arm.guard {
                        output.push_str(" if ");
                        output.push_str(&self.generate_expression(guard));
                    }

                    output.push_str(" => ");
                    output.push_str(&self.generate_expression(&arm.body));
                    output.push_str(",\n");
                }
                self.indent_level -= 1;

                output.push_str(&self.indent());
                output.push_str("}\n");
                output
            }
            Statement::Loop { body, .. } => {
                let mut output = self.indent();
                output.push_str("loop {\n");

                self.indent_level += 1;
                for stmt in body {
                    output.push_str(&self.generate_statement(stmt));
                }
                self.indent_level -= 1;

                output.push_str(&self.indent());
                output.push_str("}\n");
                output
            }
            Statement::While {
                condition, body, ..
            } => {
                let mut output = self.indent();
                output.push_str("while ");
                output.push_str(&self.generate_expression(condition));
                output.push_str(" {\n");

                self.indent_level += 1;
                for stmt in body {
                    output.push_str(&self.generate_statement(stmt));
                }
                self.indent_level -= 1;

                output.push_str(&self.indent());
                output.push_str("}\n");
                output
            }
            Statement::For {
                pattern,
                iterable,
                body,
                ..
            } => {
                let mut output = self.indent();
                output.push_str("for ");

                // Check if the loop body modifies the loop variable
                let pattern_str = self.pattern_to_rust(pattern);
                let loop_var = self.extract_pattern_identifier(pattern);
                let needs_mut = loop_var
                    .as_ref()
                    .is_some_and(|var| self.loop_body_modifies_variable(body, var));

                if needs_mut {
                    output.push_str("mut ");
                }
                output.push_str(&pattern_str);
                output.push_str(" in ");

                // Check if we need to add & for borrowed iteration
                // This handles the common case of iterating over fields of borrowed structs
                let needs_borrow = self.should_borrow_for_iteration(iterable);
                let needs_mut_borrow = needs_mut && needs_borrow;

                if needs_mut_borrow {
                    output.push_str("&mut ");
                } else if needs_borrow {
                    output.push('&');
                }

                output.push_str(&self.generate_expression(iterable));
                output.push_str(" {\n");

                self.indent_level += 1;
                for stmt in body {
                    output.push_str(&self.generate_statement(stmt));
                }
                self.indent_level -= 1;

                output.push_str(&self.indent());
                output.push_str("}\n");
                output
            }
            Statement::Break { .. } => {
                let mut output = self.indent();
                output.push_str("break;\n");
                output
            }
            Statement::Continue { .. } => {
                let mut output = self.indent();
                output.push_str("continue;\n");
                output
            }
            Statement::Use { path, alias, .. } => {
                let mut output = self.indent();
                output.push_str("use ");
                output.push_str(&path.join("::"));
                if let Some(alias_name) = alias {
                    output.push_str(" as ");
                    output.push_str(alias_name);
                }
                output.push_str(";\n");
                output
            }
            Statement::Assignment { target, value, .. } => {
                let mut output = self.indent();

                // PHASE 5 OPTIMIZATION: Check if this can use a compound operator
                if let Expression::Identifier { name: var_name, .. } = target {
                    if let Expression::Binary {
                        left, right, op, ..
                    } = value
                    {
                        if let Expression::Identifier { name: left_var, .. } = &**left {
                            if left_var == var_name {
                                // Check if we have this optimization hint
                                if self.assignment_optimizations.contains_key(var_name) {
                                    // Check if target is a mutable reference parameter (needs deref)
                                    // Parameters with assignments inferred to need &mut need deref
                                    let is_mut_param = self
                                        .current_function_params
                                        .iter()
                                        .any(|p| p.name == *var_name);

                                    // Generate compound assignment with deref if needed
                                    if is_mut_param {
                                        output.push('*');
                                    }
                                    output.push_str(&self.generate_expression(target));
                                    output.push_str(match op {
                                        crate::parser::BinaryOp::Add => " += ",
                                        crate::parser::BinaryOp::Sub => " -= ",
                                        crate::parser::BinaryOp::Mul => " *= ",
                                        crate::parser::BinaryOp::Div => " /= ",
                                        _ => " = ",
                                    });
                                    output.push_str(&self.generate_expression(right));
                                    output.push_str(";\n");
                                    return output;
                                }
                            }
                        }
                    }
                }
                // If no optimization applied, fall through to regular assignment

                // Fall back to regular assignment
                output.push_str(&self.generate_expression(target));
                output.push_str(" = ");
                output.push_str(&self.generate_expression(value));
                output.push_str(";\n");
                output
            }
            Statement::Thread { body, .. } => {
                // Transpile to std::thread::spawn for parallelism
                // When used as a statement, discard the JoinHandle
                let mut output = self.indent();
                output.push_str("let _ = std::thread::spawn(move || {\n");

                self.indent_level += 1;
                for stmt in body {
                    output.push_str(&self.generate_statement(stmt));
                }
                self.indent_level -= 1;

                output.push_str(&self.indent());
                output.push_str("});\n");
                output
            }
            Statement::Async { body, .. } => {
                // Transpile to tokio::spawn for async concurrency
                // When used as a statement, discard the JoinHandle
                let mut output = self.indent();
                output.push_str("let _ = tokio::spawn(async move {\n");

                self.indent_level += 1;
                for stmt in body {
                    output.push_str(&self.generate_statement(stmt));
                }
                self.indent_level -= 1;

                output.push_str(&self.indent());
                output.push_str("});\n");
                output
            }
            Statement::Defer { statement: _, .. } => {
                // Defer is not directly supported in Rust
                // We'll generate a comment for now
                let mut output = self.indent();
                output.push_str("// TODO: defer not yet implemented\n");
                output.push_str(&self.generate_statement(stmt));
                output
            }
        }
    }

    fn generate_pattern(&self, pattern: &Pattern) -> String {
        match pattern {
            Pattern::Wildcard => "_".to_string(),
            Pattern::Identifier(name) => name.clone(),
            Pattern::Reference(inner) => format!("&{}", self.generate_pattern(inner)),
            Pattern::EnumVariant(name, binding) => {
                use crate::parser::EnumPatternBinding;
                match binding {
                    EnumPatternBinding::Named(b) => format!("{}({})", name, b),
                    EnumPatternBinding::Wildcard => format!("{}(_)", name),
                    EnumPatternBinding::None => name.clone(),
                }
            }
            Pattern::Literal(lit) => self.generate_literal(lit),
            Pattern::Tuple(patterns) => {
                let pattern_strs: Vec<String> =
                    patterns.iter().map(|p| self.generate_pattern(p)).collect();
                format!("({})", pattern_strs.join(", "))
            }
            Pattern::Or(patterns) => {
                let pattern_strs: Vec<String> =
                    patterns.iter().map(|p| self.generate_pattern(p)).collect();
                pattern_strs.join(" | ")
            }
        }
    }

    /// Check if a pattern contains a string literal
    /// This is used to determine if we need to add .as_str() to match expressions
    fn pattern_has_string_literal(&self, pattern: &Pattern) -> bool {
        pattern_has_string_literal_impl(pattern)
    }

    fn generate_expression_with_precedence(&mut self, expr: &Expression) -> String {
        // Wrap expressions in parentheses if they need them for proper precedence
        // when used as the object of a method call or field access
        match expr {
            Expression::Range { .. } | Expression::Binary { .. } | Expression::Closure { .. } => {
                format!("({})", self.generate_expression(expr))
            }
            _ => self.generate_expression(expr),
        }
    }

    // PHASE 7: Constant folding - evaluate constant expressions at compile time
    #[allow(clippy::only_used_in_recursion)]
    fn try_fold_constant(&self, expr: &Expression) -> Option<Expression> {
        match expr {
            Expression::Binary {
                left, op, right, ..
            } => {
                // Try to fold both sides first
                let left_folded = self
                    .try_fold_constant(left)
                    .unwrap_or_else(|| (**left).clone());
                let right_folded = self
                    .try_fold_constant(right)
                    .unwrap_or_else(|| (**right).clone());

                // If both sides are literals, try to evaluate
                if let (
                    Expression::Literal { value: l, .. },
                    Expression::Literal { value: r, .. },
                ) = (&left_folded, &right_folded)
                {
                    use BinaryOp::*;
                    use Literal::*;

                    let result = match (l, op, r) {
                        // Integer arithmetic
                        (Int(a), Add, Int(b)) => Some(Literal::Int(a + b)),
                        (Int(a), Sub, Int(b)) => Some(Literal::Int(a - b)),
                        (Int(a), Mul, Int(b)) => Some(Literal::Int(a * b)),
                        (Int(a), Div, Int(b)) if *b != 0 => Some(Literal::Int(a / b)),
                        (Int(a), Mod, Int(b)) if *b != 0 => Some(Literal::Int(a % b)),

                        // Float arithmetic
                        (Float(a), Add, Float(b)) => Some(Literal::Float(a + b)),
                        (Float(a), Sub, Float(b)) => Some(Literal::Float(a - b)),
                        (Float(a), Mul, Float(b)) => Some(Literal::Float(a * b)),
                        (Float(a), Div, Float(b)) if *b != 0.0 => Some(Literal::Float(a / b)),

                        // Integer comparisons
                        (Int(a), Eq, Int(b)) => Some(Literal::Bool(a == b)),
                        (Int(a), Ne, Int(b)) => Some(Literal::Bool(a != b)),
                        (Int(a), Lt, Int(b)) => Some(Literal::Bool(a < b)),
                        (Int(a), Le, Int(b)) => Some(Literal::Bool(a <= b)),
                        (Int(a), Gt, Int(b)) => Some(Literal::Bool(a > b)),
                        (Int(a), Ge, Int(b)) => Some(Literal::Bool(a >= b)),

                        // Boolean operations
                        (Bool(a), And, Bool(b)) => Some(Literal::Bool(*a && *b)),
                        (Bool(a), Or, Bool(b)) => Some(Literal::Bool(*a || *b)),

                        _ => None,
                    };

                    return result.map(|value| Expression::Literal {
                        value,
                        location: None,
                    });
                }
                None
            }
            Expression::Unary { op, operand, .. } => {
                let operand_folded = self
                    .try_fold_constant(operand)
                    .unwrap_or_else(|| (**operand).clone());

                if let Expression::Literal { value: lit, .. } = &operand_folded {
                    use Literal::*;
                    use UnaryOp::*;

                    let result = match (op, lit) {
                        (Neg, Int(n)) => Some(Literal::Int(-n)),
                        (Neg, Float(f)) => Some(Literal::Float(-f)),
                        (Not, Bool(b)) => Some(Literal::Bool(!b)),
                        _ => None,
                    };

                    return result.map(|value| Expression::Literal {
                        value,
                        location: None,
                    });
                }
                None
            }
            // Already a literal - can't fold further
            Expression::Literal { .. } => None,
            // Can't fold non-constant expressions
            _ => None,
        }
    }

    /// Extract a field access, method call, or index expression path from an expression
    /// (e.g., "config.paths", "source.get_items()", "items[0]")
    /// This matches the logic in auto_clone.rs
    #[allow(clippy::only_used_in_recursion)]
    fn extract_field_access_path(&self, expr: &Expression) -> Option<String> {
        match expr {
            Expression::Identifier { name, .. } => Some(name.clone()),
            Expression::FieldAccess { object, field, .. } => {
                // Recursively build the path: object.field
                self.extract_field_access_path(object)
                    .map(|base_path| format!("{}.{}", base_path, field))
            }
            Expression::MethodCall { object, method, .. } => {
                // Build path for method calls: object.method()
                self.extract_field_access_path(object)
                    .map(|base_path| format!("{}.{}()", base_path, method))
            }
            Expression::Index { object, index, .. } => {
                // Build path for index expressions: object[index]
                if let Some(base_path) = self.extract_field_access_path(object) {
                    // Try to get a more specific index if it's a literal
                    let index_str = match index.as_ref() {
                        Expression::Literal {
                            value: Literal::Int(n),
                            ..
                        } => n.to_string(),
                        Expression::Identifier { name, .. } => name.clone(),
                        _ => "*".to_string(), // Generic placeholder
                    };
                    Some(format!("{}[{}]", base_path, index_str))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn generate_expression(&mut self, expr: &Expression) -> String {
        // PHASE 7: Try constant folding first
        let folded_expr = self.try_fold_constant(expr);
        let expr_to_generate = folded_expr.as_ref().unwrap_or(expr);

        match expr_to_generate {
            Expression::Literal { value: lit, .. } => self.generate_literal(lit),
            Expression::Identifier { name, .. } => {
                // Qualified paths use :: from parser (e.g., std::fs::read)
                // Simple identifiers: variable_name -> variable_name
                // Check if this is a struct field and we're in an impl block
                let base_name = if self.in_impl_block && self.current_struct_fields.contains(name) {
                    format!("self.{}", name)
                } else {
                    name.clone()
                };

                // AUTO-CLONE: Check if this variable needs to be cloned at this point
                if let Some(ref analysis) = self.auto_clone_analysis {
                    if analysis
                        .needs_clone(name, self.current_statement_idx)
                        .is_some()
                    {
                        // Automatically insert .clone() - this is the magic!
                        return format!("{}.clone()", base_name);
                    }
                }

                base_name
            }
            Expression::Binary {
                left, op, right, ..
            } => {
                // Special handling for string concatenation
                if matches!(op, BinaryOp::Add) {
                    // Only treat as string concat if at least one operand is definitely a string literal
                    let has_string_literal = matches!(
                        left.as_ref(),
                        Expression::Literal {
                            value: Literal::String(_),
                            ..
                        }
                    ) || matches!(
                        right.as_ref(),
                        Expression::Literal {
                            value: Literal::String(_),
                            ..
                        }
                    ) || Self::contains_string_literal(left)
                        || Self::contains_string_literal(right);

                    if has_string_literal {
                        // For string concatenation, use format! macro for clean, efficient code
                        return self.generate_string_concat(left, right);
                    }
                }

                // Wrap operands in parens if they have lower precedence
                let left_str = match left.as_ref() {
                    Expression::Binary { op: left_op, .. } => {
                        if self.op_precedence(left_op) < self.op_precedence(op) {
                            format!("({})", self.generate_expression(left))
                        } else {
                            self.generate_expression(left)
                        }
                    }
                    _ => self.generate_expression(left),
                };
                let right_str = match right.as_ref() {
                    Expression::Binary { op: right_op, .. } => {
                        if self.op_precedence(right_op) < self.op_precedence(op) {
                            format!("({})", self.generate_expression(right))
                        } else {
                            self.generate_expression(right)
                        }
                    }
                    _ => self.generate_expression(right),
                };
                let op_str = self.binary_op_to_rust(op);
                format!("{} {} {}", left_str, op_str, right_str)
            }
            Expression::Unary { op, operand, .. } => {
                let operand_str = self.generate_expression(operand);
                let op_str = self.unary_op_to_rust(op);
                format!("{}{}", op_str, operand_str)
            }
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                // Extract function name for signature lookup
                let func_name = self.extract_function_name(function);

                // Special case: Tauri command calls (for WASM target)
                if self.target == CompilationTarget::Wasm && self.is_tauri_function(&func_name) {
                    return self.generate_tauri_invoke(&func_name, arguments);
                }

                // Special case: convert print/println/eprintln/eprint() to macros
                if func_name == "print"
                    || func_name == "println"
                    || func_name == "eprintln"
                    || func_name == "eprint"
                {
                    let macro_name = func_name.clone();

                    // For print() -> println!(), otherwise keep the same name
                    let target_macro = if macro_name == "print" {
                        "println".to_string()
                    } else {
                        macro_name.clone()
                    };
                    // Check if the first argument is a format! macro (from string interpolation)
                    if let Some((_, first_arg)) = arguments.first() {
                        // Check for MacroInvocation (explicit format! calls)
                        if let Expression::MacroInvocation {
                            name,
                            args: macro_args,
                            ..
                        } = first_arg
                        {
                            if name == "format" && !macro_args.is_empty() {
                                // Unwrap the format! call and put its arguments directly into println!
                                // format!("text {}", var) -> println!("text {}", var)
                                let format_str = self.generate_expression(&macro_args[0]);
                                let format_args: Vec<String> = macro_args[1..]
                                    .iter()
                                    .map(|arg| self.generate_expression(arg))
                                    .collect();

                                let args_str = if format_args.is_empty() {
                                    String::new()
                                } else {
                                    format!(", {}", format_args.join(", "))
                                };

                                return format!("{}!({}{})", target_macro, format_str, args_str);
                            }
                        }

                        // Check for Binary expression with string concatenation (will become format!)
                        if let Expression::Binary {
                            left,
                            op: BinaryOp::Add,
                            right,
                            ..
                        } = first_arg
                        {
                            // Check if this is string concatenation
                            let has_string_literal = matches!(
                                left.as_ref(),
                                Expression::Literal {
                                    value: Literal::String(_),
                                    ..
                                }
                            ) || matches!(
                                right.as_ref(),
                                Expression::Literal {
                                    value: Literal::String(_),
                                    ..
                                }
                            ) || Self::contains_string_literal(left)
                                || Self::contains_string_literal(right);

                            if has_string_literal {
                                // Collect all parts of the concatenation
                                let mut parts = Vec::new();
                                Self::collect_concat_parts_static(left, &mut parts);
                                Self::collect_concat_parts_static(right, &mut parts);

                                // Generate format string and arguments
                                let format_str = "{}".repeat(parts.len());
                                let format_args: Vec<String> = parts
                                    .iter()
                                    .map(|expr| self.generate_expression(expr))
                                    .collect();

                                return format!(
                                    "{}!(\"{}\", {})",
                                    target_macro,
                                    format_str,
                                    format_args.join(", ")
                                );
                            }
                        }

                        // Check if the first argument is a string literal with ${} (old-style, shouldn't happen but keep for safety)
                        if let Expression::Literal {
                            value: Literal::String(s),
                            ..
                        } = first_arg
                        {
                            if s.contains("${") {
                                // Handle string interpolation directly in println!
                                // Convert "${var}" to "{}" and extract variables
                                let mut format_str = String::new();
                                let mut args = Vec::new();
                                let mut chars = s.chars().peekable();

                                while let Some(ch) = chars.next() {
                                    if ch == '$' && chars.peek() == Some(&'{') {
                                        chars.next(); // consume {
                                        let mut var_name = String::new();

                                        while let Some(&next_ch) = chars.peek() {
                                            if next_ch == '}' {
                                                chars.next(); // consume }
                                                break;
                                            } else {
                                                var_name.push(next_ch);
                                                chars.next();
                                            }
                                        }

                                        if !var_name.is_empty() {
                                            format_str.push_str("{}");
                                            // Check if this is a struct field
                                            if self.in_impl_block
                                                && self.current_struct_fields.contains(&var_name)
                                            {
                                                args.push(format!("self.{}", var_name));
                                            } else {
                                                args.push(var_name);
                                            }
                                        }
                                    } else {
                                        format_str.push(ch);
                                    }
                                }

                                let args_str = if args.is_empty() {
                                    String::new()
                                } else {
                                    format!(", {}", args.join(", "))
                                };

                                return format!(
                                    "{}!(\"{}\"{})",
                                    target_macro,
                                    format_str.replace('\\', "\\\\").replace('"', "\\\""),
                                    args_str
                                );
                            }
                        }
                    }

                    // No interpolation, just regular print
                    let args: Vec<String> = arguments
                        .iter()
                        .map(|(_label, arg)| self.generate_expression(arg))
                        .collect();
                    return format!("{}!({})", target_macro, args.join(", "));
                }

                // Special case: convert assert() to assert!()
                if func_name == "assert" {
                    let args: Vec<String> = arguments
                        .iter()
                        .map(|(_label, arg)| self.generate_expression(arg))
                        .collect();
                    return format!("assert!({})", args.join(", "));
                }

                let func_str = self.generate_expression(function);

                // Look up signature and clone it to avoid borrow conflicts
                let signature = self.signature_registry.get_signature(&func_name).cloned();

                let args: Vec<String> = arguments
                    .iter()
                    .enumerate()
                    .map(|(i, (_label, arg))| {
                        let mut arg_str = self.generate_expression(arg);

                        // Auto-convert string literals to String for functions expecting owned String
                        if matches!(
                            arg,
                            Expression::Literal {
                                value: Literal::String(_),
                                ..
                            }
                        ) {
                            // Check if the parameter expects an owned String
                            let should_convert = if let Some(ref sig) = signature {
                                if let Some(&ownership) = sig.param_ownership.get(i) {
                                    // Convert if parameter expects owned String
                                    matches!(ownership, OwnershipMode::Owned)
                                } else {
                                    // No ownership info - check if it's a stdlib function
                                    // (stdlib functions typically expect owned String)
                                    func_str.contains("::")
                                }
                            } else {
                                // No signature found - check if it's a stdlib function
                                func_str.contains("::")
                            };

                            if should_convert {
                                arg_str = format!("{}.to_string()", arg_str);
                            }
                        }

                        // Check if this parameter expects a borrow
                        if let Some(ref sig) = signature {
                            if let Some(&ownership) = sig.param_ownership.get(i) {
                                match ownership {
                                    OwnershipMode::Borrowed => {
                                        // Insert & if not already a reference
                                        if !self.is_reference_expression(arg) {
                                            return format!("&{}", arg_str);
                                        }
                                    }
                                    OwnershipMode::MutBorrowed => {
                                        // Insert &mut if not already a reference
                                        if !self.is_reference_expression(arg) {
                                            return format!("&mut {}", arg_str);
                                        }
                                    }
                                    OwnershipMode::Owned => {
                                        // No change needed (already handled above for string literals)
                                    }
                                }
                            }
                        }

                        arg_str
                    })
                    .collect();
                format!("{}({})", func_str, args.join(", "))
            }
            Expression::MethodCall {
                object,
                method,
                type_args,
                arguments,
                ..
            } => {
                let obj_str = self.generate_expression_with_precedence(object);

                // Special handling for methods that commonly need String arguments
                // Only apply to push/push_str/set, not insert (which can have different types)
                let needs_string_conversion =
                    matches!(method.as_str(), "push" | "push_str" | "set");

                let args: Vec<String> = arguments
                    .iter()
                    .map(|(_label, arg)| {
                        let mut arg_str = self.generate_expression(arg);

                        // Auto-convert string literals to String for methods that need it
                        if needs_string_conversion
                            && matches!(
                                arg,
                                Expression::Literal {
                                    value: Literal::String(_),
                                    ..
                                }
                            )
                        {
                            arg_str = format!("{}.to_string()", arg_str);
                        }

                        arg_str
                    })
                    .collect();

                // Generate turbofish if present
                let turbofish = if let Some(types) = type_args {
                    let type_strs: Vec<String> =
                        types.iter().map(|t| self.type_to_rust(t)).collect();
                    format!("::<{}>", type_strs.join(", "))
                } else {
                    String::new()
                };

                // Special case: empty method name means turbofish on a function call (func::<T>())
                if method.is_empty() {
                    return format!("{}{}({})", obj_str, turbofish, args.join(", "));
                }

                // Special case: substring(start, end) -> &text[start..end]
                if method == "substring" && args.len() == 2 {
                    return format!("&{}[{}..{}]", obj_str, args[0], args[1]);
                }

                // Determine separator: :: for static calls, . for instance methods
                // - Type/Module (starts with uppercase): use ::
                // - Variable (starts with lowercase): use .
                let separator = match &**object {
                    Expression::Call { .. } | Expression::MethodCall { .. } => ".", // Instance method on return value
                    Expression::Identifier { name, .. } => {
                        // Check for known module/crate names that should use ::
                        // Note: Avoid common variable names like "path", "config" which are used as variables
                        let known_modules = [
                            "std",
                            "serde_json",
                            "serde",
                            "tokio",
                            "reqwest",
                            "sqlx",
                            "chrono",
                            "sha2",
                            "bcrypt",
                            "base64",
                            "rand",
                            "Vec",
                            "String",
                            "Option",
                            "Result",
                            "Box",
                            "Arc",
                            "Mutex",
                            "Utc",
                            "Local",
                            "DEFAULT_COST",
                            // Stdlib modules (avoid common variable names)
                            "mime",
                            "http",
                            "fs",
                            "strings",
                            "json",
                            "regex",
                            "cli",
                            "log",
                            "crypto",
                            "io",
                            "env",
                            "time",
                            "sync",
                            "thread",
                            "collections",
                            "cmp",
                        ];

                        // Type or module (uppercase) vs variable (lowercase)
                        if name.chars().next().is_some_and(|c| c.is_uppercase())
                            || name.contains('.')
                            || known_modules.contains(&name.as_str())
                        {
                            "::" // Vec::new(), std::fs::read(), serde_json::to_string()
                        } else {
                            "." // x.abs(), value.method()
                        }
                    }
                    Expression::FieldAccess { ref object, .. } => {
                        // Check if this is a module path (e.g., std::fs) or a field access (e.g., self.count)
                        // If the object is an identifier that looks like a module, use ::
                        // Otherwise, use . for instance methods on fields
                        match object.as_ref() {
                            Expression::Identifier { name, .. } => {
                                if name.chars().next().is_some_and(|c| c.is_uppercase())
                                    || name == "std"
                                {
                                    "::" // Module::path::method() -> static method
                                } else {
                                    "." // self.field.method() or variable.field.method() -> instance method
                                }
                            }
                            _ => ".", // Default to instance method
                        }
                    }
                    _ => ".", // Instance method on expressions
                };

                // SPECIAL CASE: .slice() method is our desugared slice syntax [start..end]
                // Convert it back to proper Rust slice syntax
                // For strings, we need to add & to get &str (a reference)
                if method == "slice" && args.len() == 2 {
                    return format!("&{}[{}..{}]", obj_str, args[0], args[1]);
                }

                // PHASE 2 OPTIMIZATION: Eliminate unnecessary .clone() calls
                // DISABLED: This optimization was too aggressive and removed needed clones
                // TODO: Make this more conservative - only remove clone when we can prove
                // the value is Copy or when it's the last use
                // if method == "clone" && arguments.is_empty() {
                //     if let Expression::Identifier { name: ref var_name, location: None } = **object {
                //         if self.clone_optimizations.contains(var_name) {
                //             // Skip the .clone(), just return the variable (or borrow if needed)
                //             return obj_str;
                //         }
                //     }
                // }

                // UI FRAMEWORK: Check if we need to add .to_vnode() for .child() methods
                let processed_args = if method == "child" && !arguments.is_empty() {
                    // Always wrap the first argument with .to_vnode() for .child() methods
                    // The ToVNode trait will handle the conversion
                    let mut new_args = vec![format!("({}).to_vnode()", args[0])];
                    new_args.extend_from_slice(&args[1..]);
                    new_args
                } else {
                    args
                };

                let base_expr = format!(
                    "{}{}{}{}({})",
                    obj_str,
                    separator,
                    method,
                    turbofish,
                    processed_args.join(", ")
                );

                // AUTO-CLONE: Check if this method call needs to be cloned
                if let Some(path) = self.extract_field_access_path(expr) {
                    if let Some(ref analysis) = self.auto_clone_analysis {
                        if analysis
                            .needs_clone(&path, self.current_statement_idx)
                            .is_some()
                        {
                            // Automatically insert .clone() for method call result!
                            return format!("{}.clone()", base_expr);
                        }
                    }
                }
                base_expr
            }
            Expression::FieldAccess { object, field, .. } => {
                let obj_str = self.generate_expression_with_precedence(object);

                // Determine if this is a module/type path (::) or field access (.)
                // Check the object to decide:
                let separator = match &**object {
                    Expression::Identifier { name, .. }
                        if name.contains("::")
                            || (!name.is_empty()
                                && name.chars().next().unwrap().is_uppercase()) =>
                    {
                        "::" // Module path: std::fs or Type::CONST
                    }
                    Expression::FieldAccess { .. } => {
                        // Check if this is a module path or a field chain
                        // If the object string contains ::, it's a module path
                        if obj_str.contains("::") {
                            "::" // Module path: std::fs::File
                        } else {
                            "." // Field chain: transform.position.x
                        }
                    }
                    _ => ".", // Actual field access (e.g., config.field)
                };

                let base_expr = format!("{}{}{}", obj_str, separator, field);

                // AUTO-CLONE: Check if this field access needs to be cloned
                // Extract the full path (e.g., "config.paths")
                if let Some(path) = self.extract_field_access_path(expr) {
                    if let Some(ref analysis) = self.auto_clone_analysis {
                        if analysis
                            .needs_clone(&path, self.current_statement_idx)
                            .is_some()
                        {
                            // Automatically insert .clone() for field access!
                            return format!("{}.clone()", base_expr);
                        }
                    }
                }

                base_expr
            }
            Expression::StructLiteral { name, fields, .. } => {
                // PHASE 3 OPTIMIZATION: Check if we have optimization hints for this struct
                let _has_optimization_hint = self.struct_mapping_hints.get(name);

                // Generate field assignments
                let field_str: Vec<String> = fields
                    .iter()
                    .map(|(field_name, expr)| {
                        // For simple direct field access (e.g., source.field -> target.field),
                        // we can generate cleaner code
                        let expr_str = self.generate_expression(expr);

                        // Check for field shorthand: if expr is just the field name, use shorthand
                        if let Expression::Identifier { name: id, .. } = expr {
                            if id == field_name {
                                // Shorthand: User { name } instead of User { name: name }
                                return field_name.clone();
                            }
                        }

                        format!("{}: {}", field_name, expr_str)
                    })
                    .collect();

                format!("{} {{ {} }}", name, field_str.join(", "))
            }
            Expression::MapLiteral { pairs, .. } => {
                // Generate HashMap literal: HashMap::from([(key, value), ...])
                if pairs.is_empty() {
                    "std::collections::HashMap::new()".to_string()
                } else {
                    let entries_str: Vec<String> = pairs
                        .iter()
                        .map(|(k, v)| {
                            let key_str = self.generate_expression(k);
                            let val_str = self.generate_expression(v);
                            format!("({}, {})", key_str, val_str)
                        })
                        .collect();
                    format!(
                        "std::collections::HashMap::from([{}])",
                        entries_str.join(", ")
                    )
                }
            }
            Expression::TryOp { expr: inner, .. } => {
                format!("{}?", self.generate_expression(inner))
            }
            Expression::Await { expr: inner, .. } => {
                format!("{}.await", self.generate_expression(inner))
            }
            Expression::ChannelSend { channel, value, .. } => {
                let ch_str = self.generate_expression(channel);
                let val_str = self.generate_expression(value);
                format!("{}.send({})", ch_str, val_str)
            }
            Expression::ChannelRecv { channel, .. } => {
                let ch_str = self.generate_expression(channel);
                format!("{}.recv()", ch_str)
            }
            Expression::Range {
                start,
                end,
                inclusive,
                ..
            } => {
                let start_str = self.generate_expression(start);
                let end_str = self.generate_expression(end);
                if *inclusive {
                    format!("{}..={}", start_str, end_str)
                } else {
                    format!("{}..{}", start_str, end_str)
                }
            }
            Expression::Closure {
                parameters, body, ..
            } => {
                let params = parameters.join(", ");
                let body_str = self.generate_expression(body);

                // In Windjammer, closures automatically use move semantics when needed
                // This is more ergonomic than requiring explicit 'move' keyword
                // We always generate 'move' for safety in concurrent contexts
                format!("move |{}| {}", params, body_str)
            }
            Expression::Index { object, index, .. } => {
                let obj_str = self.generate_expression(object);

                // Special case: if index is a Range, this is slice syntax
                // FIXED: Don't add & - Rust will auto-coerce to &[T] when needed
                // This prevents "&temporary" errors when chaining methods like .to_vec()
                if let Expression::Range {
                    start,
                    end,
                    inclusive,
                    ..
                } = &**index
                {
                    let start_str = self.generate_expression(start);
                    let end_str = self.generate_expression(end);
                    let range_op = if *inclusive { "..=" } else { ".." };
                    return format!("{}[{}{}{}]", obj_str, start_str, range_op, end_str);
                }

                let idx_str = self.generate_expression(index);
                let base_expr = format!("{}[{}]", obj_str, idx_str);

                // AUTO-CLONE: Check if this index expression needs to be cloned
                if let Some(path) = self.extract_field_access_path(expr) {
                    if let Some(ref analysis) = self.auto_clone_analysis {
                        if analysis
                            .needs_clone(&path, self.current_statement_idx)
                            .is_some()
                        {
                            // Automatically insert .clone() for index expression!
                            return format!("{}.clone()", base_expr);
                        }
                    }
                }
                base_expr
            }
            Expression::Tuple {
                elements: exprs, ..
            } => {
                let expr_strs: Vec<String> =
                    exprs.iter().map(|e| self.generate_expression(e)).collect();
                format!("({})", expr_strs.join(", "))
            }
            Expression::Array {
                elements: exprs, ..
            } => {
                let expr_strs: Vec<String> =
                    exprs.iter().map(|e| self.generate_expression(e)).collect();
                format!("vec![{}]", expr_strs.join(", "))
            }
            Expression::MacroInvocation {
                name,
                args,
                delimiter,
                ..
            } => {
                use crate::parser::MacroDelimiter;

                // PHASE 4 OPTIMIZATION: Check for format! with capacity hints
                if name == "format" {
                    if let Some(&capacity) =
                        self.string_capacity_hints.get(&self.current_statement_idx)
                    {
                        // Clone capacity to avoid borrow issues
                        let capacity_val = capacity;
                        // Generate optimized String::with_capacity + write! instead of format!
                        self.needs_write_import = true;
                        let arg_strs: Vec<String> =
                            args.iter().map(|e| self.generate_expression(e)).collect();

                        return format!(
                            "{{\n{}    let mut __s = String::with_capacity({});\n{}    write!(&mut __s, {}).unwrap();\n{}    __s\n{}}}",
                            self.indent(),
                            capacity_val,
                            self.indent(),
                            arg_strs.join(", "),
                            self.indent(),
                            self.indent()
                        );
                    }
                }

                // Special case: if this is println!/eprintln!/print!/eprint! and first arg is format!, flatten it
                let should_flatten = (name == "println"
                    || name == "eprintln"
                    || name == "print"
                    || name == "eprint")
                    && !args.is_empty()
                    && matches!(&args[0], Expression::MacroInvocation { name: macro_name, .. } if macro_name == "format");

                let arg_strs: Vec<String> = if should_flatten {
                    // Flatten format! macro arguments into the print macro
                    if let Expression::MacroInvocation {
                        args: format_args, ..
                    } = &args[0]
                    {
                        format_args
                            .iter()
                            .map(|e| self.generate_expression(e))
                            .collect()
                    } else {
                        args.iter().map(|e| self.generate_expression(e)).collect()
                    }
                } else {
                    // Special case: if this is println!/eprintln!/print!/eprint! with a single non-literal arg,
                    // wrap it with "{}" to make it valid Rust: println!(var) -> println!("{}", var)
                    // Also wrap format!() calls: println!(format!(...)) -> println!("{}", format!(...))
                    if (name == "println"
                        || name == "eprintln"
                        || name == "print"
                        || name == "eprint")
                        && args.len() == 1
                        && !matches!(
                            &args[0],
                            Expression::Literal {
                                value: Literal::String(_),
                                ..
                            }
                        )
                    {
                        vec!["\"{}\"".to_string(), self.generate_expression(&args[0])]
                    } else {
                        args.iter().map(|e| self.generate_expression(e)).collect()
                    }
                };

                let (open, close) = match delimiter {
                    MacroDelimiter::Parens => ("(", ")"),
                    MacroDelimiter::Brackets => ("[", "]"),
                    MacroDelimiter::Braces => ("{", "}"),
                };
                format!("{}!{}{}{}", name, open, arg_strs.join(", "), close)
            }
            Expression::Cast { expr, type_, .. } => {
                // Add parentheses around binary expressions for correct precedence
                let expr_str = match &**expr {
                    Expression::Binary { .. } => {
                        format!("({})", self.generate_expression(expr))
                    }
                    _ => self.generate_expression(expr),
                };
                let type_str = self.type_to_rust(type_);
                format!("{} as {}", expr_str, type_str)
            }
            Expression::Block {
                statements: stmts, ..
            } => {
                // Special case: if the block contains only a match statement, generate it as a match expression
                if stmts.len() == 1 {
                    if let Statement::Match { value, arms, .. } = &stmts[0] {
                        let mut output = String::from("match ");

                        // Check if any arm has a string literal pattern
                        let has_string_literal = arms
                            .iter()
                            .any(|arm| self.pattern_has_string_literal(&arm.pattern));

                        let value_str = self.generate_expression(value);
                        if has_string_literal {
                            // Add .as_str() if the value doesn't already end with it
                            if !value_str.ends_with(".as_str()") {
                                output.push_str(&format!("{}.as_str()", value_str));
                            } else {
                                output.push_str(&value_str);
                            }
                        } else {
                            output.push_str(&value_str);
                        }

                        output.push_str(" {\n");

                        self.indent_level += 1;
                        for arm in arms {
                            output.push_str(&self.indent());
                            output.push_str(&self.generate_pattern(&arm.pattern));

                            // Add guard if present
                            if let Some(guard) = &arm.guard {
                                output.push_str(" if ");
                                output.push_str(&self.generate_expression(guard));
                            }

                            output.push_str(" => ");
                            output.push_str(&self.generate_expression(&arm.body));
                            output.push_str(",\n");
                        }
                        self.indent_level -= 1;

                        output.push_str(&self.indent());
                        output.push('}');
                        return output;
                    }
                }

                // Regular block - must handle last expression correctly
                let mut output = String::from("{\n");
                self.indent_level += 1;

                let len = stmts.len();
                for (i, stmt) in stmts.iter().enumerate() {
                    let is_last = i == len - 1;
                    if is_last
                        && matches!(
                            stmt,
                            Statement::Expression { .. }
                                | Statement::Thread { .. }
                                | Statement::Async { .. }
                        )
                    {
                        // Last statement is an expression or thread/async block - generate without discard (it's the return value)
                        match stmt {
                            Statement::Expression { expr, .. } => {
                                output.push_str(&self.indent());
                                output.push_str(&self.generate_expression(expr));
                                output.push('\n');
                            }
                            Statement::Thread { body, .. } => {
                                // Generate as expression (returns JoinHandle)
                                output.push_str(&self.indent());
                                output.push_str("std::thread::spawn(move || {\n");
                                self.indent_level += 1;
                                for stmt in body {
                                    output.push_str(&self.generate_statement(stmt));
                                }
                                self.indent_level -= 1;
                                output.push_str(&self.indent());
                                output.push_str("})\n");
                            }
                            Statement::Async { body, .. } => {
                                // Generate as expression (returns JoinHandle)
                                output.push_str(&self.indent());
                                output.push_str("tokio::spawn(async move {\n");
                                self.indent_level += 1;
                                for stmt in body {
                                    output.push_str(&self.generate_statement(stmt));
                                }
                                self.indent_level -= 1;
                                output.push_str(&self.indent());
                                output.push_str("})\n");
                            }
                            _ => unreachable!(),
                        }
                    } else {
                        output.push_str(&self.generate_statement(stmt));
                    }
                }

                self.indent_level -= 1;
                output.push_str(&self.indent());
                output.push('}');
                output
            }
        }
    }

    fn generate_literal(&self, lit: &Literal) -> String {
        match lit {
            Literal::Int(n) => n.to_string(),
            Literal::Float(f) => {
                let s = f.to_string();
                // Ensure float literals always have a decimal point
                if !s.contains('.') && !s.contains('e') {
                    format!("{}.0", s)
                } else {
                    s
                }
            }
            Literal::String(s) => {
                // Check for string interpolation: {variable}
                if s.contains('{') && s.contains('}') {
                    // Convert to format! macro
                    // "Count: {count}" -> format!("Count: {}", count)
                    let mut format_str = String::new();
                    let mut args = Vec::new();
                    let mut chars = s.chars().peekable();

                    while let Some(ch) = chars.next() {
                        if ch == '{' {
                            // Check if it's {variable} pattern or {} placeholder
                            let mut var_name = String::new();
                            let mut is_variable = true;

                            while let Some(&next_ch) = chars.peek() {
                                if next_ch == '}' {
                                    chars.next(); // consume }
                                    break;
                                } else if next_ch.is_alphanumeric() || next_ch == '_' {
                                    var_name.push(next_ch);
                                    chars.next();
                                } else {
                                    // Not a simple variable pattern
                                    is_variable = false;
                                    break;
                                }
                            }

                            if is_variable && !var_name.is_empty() {
                                // It's a variable interpolation: {count} -> {}, count
                                format_str.push_str("{}");
                                args.push(var_name);
                            } else if is_variable && var_name.is_empty() {
                                // It's an empty placeholder: {} -> keep as-is (format! placeholder)
                                format_str.push_str("{}");
                            } else {
                                // Not a variable, escape the literal brace
                                format_str.push_str("{{");
                                format_str.push_str(&var_name);
                            }
                        } else if ch == '}' {
                            // Escape literal closing brace (not part of a placeholder)
                            format_str.push_str("}}");
                        } else {
                            format_str.push(ch);
                        }
                    }

                    if args.is_empty() {
                        // No interpolation found, just a regular string
                        format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
                    } else {
                        // Generate format! call with implicit self for struct fields
                        let formatted_args = args
                            .iter()
                            .map(|a| {
                                // Check if this is a struct field and add self. prefix
                                if self.in_impl_block && self.current_struct_fields.contains(a) {
                                    format!(", self.{}", a)
                                } else {
                                    format!(", {}", a)
                                }
                            })
                            .collect::<String>();

                        format!(
                            "format!(\"{}\"{})",
                            format_str.replace('\\', "\\\\").replace('"', "\\\""),
                            formatted_args
                        )
                    }
                } else {
                    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
                }
            }
            Literal::Char(c) => {
                // Escape special characters
                match c {
                    '\n' => "'\\n'".to_string(),
                    '\t' => "'\\t'".to_string(),
                    '\r' => "'\\r'".to_string(),
                    '\\' => "'\\\\'".to_string(),
                    '\'' => "'\\''".to_string(),
                    '\0' => "'\\0'".to_string(),
                    _ => format!("'{}'", c),
                }
            }
            Literal::Bool(b) => b.to_string(),
        }
    }

    fn binary_op_to_rust(&self, op: &BinaryOp) -> &str {
        match op {
            BinaryOp::Add => "+",
            BinaryOp::Sub => "-",
            BinaryOp::Mul => "*",
            BinaryOp::Div => "/",
            BinaryOp::Mod => "%",
            BinaryOp::Eq => "==",
            BinaryOp::Ne => "!=",
            BinaryOp::Lt => "<",
            BinaryOp::Le => "<=",
            BinaryOp::Gt => ">",
            BinaryOp::Ge => ">=",
            BinaryOp::And => "&&",
            BinaryOp::Or => "||",
        }
    }

    /// Generate efficient string concatenation using format! macro
    fn generate_string_concat(&mut self, left: &Expression, right: &Expression) -> String {
        // Collect all parts of the concatenation chain
        let mut parts = Vec::new();
        Self::collect_concat_parts_static(left, &mut parts);
        Self::collect_concat_parts_static(right, &mut parts);

        // Generate format! macro call
        let format_str = "{}".repeat(parts.len());

        // Generate expressions for each part
        let mut args = Vec::new();
        for expr in &parts {
            args.push(self.generate_expression(expr));
        }

        format!("format!(\"{}\", {})", format_str, args.join(", "))
    }

    /// Recursively collect all parts of a string concatenation chain (static to avoid borrow issues)
    fn collect_concat_parts_static(expr: &Expression, parts: &mut Vec<Expression>) {
        match expr {
            Expression::Binary {
                left,
                op: BinaryOp::Add,
                right,
                ..
            } => {
                Self::collect_concat_parts_static(left, parts);
                Self::collect_concat_parts_static(right, parts);
            }
            _ => parts.push(expr.clone()),
        }
    }

    /// Check if an expression contains a string literal (recursively for binary expressions)
    fn contains_string_literal(expr: &Expression) -> bool {
        match expr {
            Expression::Literal {
                value: Literal::String(_),
                ..
            } => true,
            Expression::Binary { left, right, .. } => {
                Self::contains_string_literal(left) || Self::contains_string_literal(right)
            }
            _ => false,
        }
    }

    fn op_precedence(&self, op: &BinaryOp) -> i32 {
        match op {
            BinaryOp::Or => 1,
            BinaryOp::And => 2,
            BinaryOp::Eq | BinaryOp::Ne => 3,
            BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge => 4,
            BinaryOp::Add | BinaryOp::Sub => 5,
            BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod => 6,
        }
    }

    fn unary_op_to_rust(&self, op: &UnaryOp) -> &str {
        match op {
            UnaryOp::Not => "!",
            UnaryOp::Neg => "-",
            UnaryOp::Ref => "&",
            UnaryOp::MutRef => "&mut ",
            UnaryOp::Deref => "*",
        }
    }

    fn extract_function_name(&self, expr: &Expression) -> String {
        match expr {
            Expression::Identifier { name, .. } => name.clone(),
            Expression::FieldAccess { field, .. } => field.clone(),
            _ => String::new(), // Can't determine function name
        }
    }

    /// Check if a function is a Tauri command that needs special handling
    fn is_tauri_function(&self, name: &str) -> bool {
        matches!(
            name,
            "read_file"
                | "write_file"
                | "list_directory"
                | "create_game_project"
                | "run_game"
                | "stop_game"
                | "open_file_dialog"
                | "save_file_dialog"
                | "set_title"
                | "minimize"
                | "maximize"
                | "close"
        )
    }

    /// Generate a Tauri invoke call for WASM
    fn generate_tauri_invoke(
        &mut self,
        func_name: &str,
        arguments: &[(Option<String>, Expression)],
    ) -> String {
        // Generate the invoke call
        let mut code = String::from("tauri_invoke(\"");
        code.push_str(func_name);
        code.push_str("\", ");

        // Generate arguments object
        if arguments.is_empty() {
            code.push_str("serde_json::json!({})");
        } else {
            code.push_str("serde_json::json!({ ");
            for (i, (param_name, arg_expr)) in arguments.iter().enumerate() {
                if i > 0 {
                    code.push_str(", ");
                }
                // Use parameter name or default to arg0, arg1, etc.
                let key = param_name
                    .as_ref()
                    .map(|s| s.as_str())
                    .unwrap_or_else(|| match i {
                        0 => "arg0",
                        1 => "arg1",
                        2 => "arg2",
                        _ => "arg",
                    });
                code.push('"');
                code.push_str(key);
                code.push_str("\": ");
                code.push_str(&self.generate_expression(arg_expr));
            }
            code.push_str(" })");
        }
        code.push(')');

        // Mark that we need serde_json imports
        self.needs_serde_imports = true;

        code
    }

    fn is_reference_expression(&self, expr: &Expression) -> bool {
        matches!(
            expr,
            Expression::Unary {
                op: UnaryOp::Ref | UnaryOp::MutRef,
                ..
            }
        )
    }

    fn infer_derivable_traits(&self, struct_: &StructDecl) -> Vec<String> {
        let mut traits = vec!["Debug".to_string(), "Clone".to_string()]; // Always safe to derive

        // Check if all fields are Copy
        if self.all_fields_are_copy(&struct_.fields) {
            traits.push("Copy".to_string());
        }

        // Check if all fields are PartialEq/Eq
        if self.all_fields_are_comparable(&struct_.fields) {
            traits.push("PartialEq".to_string());
            traits.push("Eq".to_string());

            // If Eq, also check for Hash
            if self.all_fields_are_hashable(&struct_.fields) {
                traits.push("Hash".to_string());
            }
        }

        // Check if all fields have Default
        if self.all_fields_have_default(&struct_.fields) {
            traits.push("Default".to_string());
        }

        traits
    }

    fn all_fields_are_copy(&self, fields: &[crate::parser::StructField]) -> bool {
        fields
            .iter()
            .all(|field| self.is_copy_type(&field.field_type))
    }

    fn all_fields_are_comparable(&self, fields: &[crate::parser::StructField]) -> bool {
        fields
            .iter()
            .all(|field| self.is_comparable_type(&field.field_type))
    }

    fn all_fields_are_hashable(&self, fields: &[crate::parser::StructField]) -> bool {
        fields
            .iter()
            .all(|field| self.is_hashable_type(&field.field_type))
    }

    fn all_fields_have_default(&self, fields: &[crate::parser::StructField]) -> bool {
        fields
            .iter()
            .all(|field| self.has_default(&field.field_type))
    }

    #[allow(clippy::only_used_in_recursion)]
    fn is_copy_type(&self, ty: &Type) -> bool {
        match ty {
            Type::Int | Type::Int32 | Type::Uint | Type::Float | Type::Bool => true,
            Type::Reference(_) => true,         // References are Copy
            Type::MutableReference(_) => false, // Mutable references are not Copy
            Type::Tuple(types) => types.iter().all(|t| self.is_copy_type(t)),
            Type::Custom(name) => {
                // Recognize common Rust primitive types by name
                matches!(
                    name.as_str(),
                    "i8" | "i16"
                        | "i32"
                        | "i64"
                        | "i128"
                        | "isize"
                        | "u8"
                        | "u16"
                        | "u32"
                        | "u64"
                        | "u128"
                        | "usize"
                        | "f32"
                        | "f64"
                        | "bool"
                        | "char"
                )
            }
            _ => false, // String, Vec, Option, Result, other Custom types are not Copy
        }
    }

    #[allow(clippy::only_used_in_recursion)]
    fn is_comparable_type(&self, ty: &Type) -> bool {
        match ty {
            Type::Int | Type::Int32 | Type::Uint | Type::Float | Type::Bool | Type::String => true,
            Type::Reference(inner) | Type::MutableReference(inner) => {
                self.is_comparable_type(inner)
            }
            Type::Tuple(types) => types.iter().all(|t| self.is_comparable_type(t)),
            Type::Option(inner) => self.is_comparable_type(inner),
            Type::Result(ok, err) => self.is_comparable_type(ok) && self.is_comparable_type(err),
            _ => false, // Vec is not Eq (only PartialEq), Custom types unknown
        }
    }

    #[allow(clippy::only_used_in_recursion)]
    fn is_hashable_type(&self, ty: &Type) -> bool {
        match ty {
            Type::Int | Type::Int32 | Type::Uint | Type::Bool | Type::String => true,
            Type::Float => false, // Floats are not Hash
            Type::Reference(inner) => self.is_hashable_type(inner),
            Type::MutableReference(_) => false,
            Type::Tuple(types) => types.iter().all(|t| self.is_hashable_type(t)),
            Type::Vec(_) => false, // Vec is not Hash
            Type::Option(inner) => self.is_hashable_type(inner),
            _ => false, // Result, Custom types - assume not Hash
        }
    }

    #[allow(clippy::only_used_in_recursion)]
    fn has_default(&self, ty: &Type) -> bool {
        match ty {
            Type::Int | Type::Int32 | Type::Uint | Type::Float | Type::Bool => true,
            Type::String => true,    // String has Default ("")
            Type::Vec(_) => true,    // Vec has Default (empty vec)
            Type::Option(_) => true, // Option has Default (None)
            Type::Tuple(types) => types.iter().all(|t| self.has_default(t)),
            _ => false, // Refs don't have Default, Result/Custom types unknown
        }
    }

    /// OPTIMIZATION: Determine if a function should be marked #[inline]
    /// Phase 1: Generate Inlinable Code
    ///
    /// Heuristics for inlining:
    /// 1. Module functions (stdlib wrappers) - always inline for zero-cost abstraction
    /// 2. Small functions (< 10 statements) - likely to benefit from inlining
    /// 3. Trivial getters/setters - always inline
    /// 4. Functions with only one return statement - simple enough to inline
    /// 5. Don't inline: main(), test functions, async functions, large functions
    fn should_inline_function(&self, func: &FunctionDecl, _analyzed: &AnalyzedFunction) -> bool {
        // Never inline main
        if func.name == "main" {
            return false;
        }

        // Never inline test functions
        if func.decorators.iter().any(|d| d.name == "test") {
            return false;
        }

        // Don't inline async functions (they're already state machines)
        if func.decorators.iter().any(|d| d.name == "async") {
            return false;
        }

        // ALWAYS inline module functions (stdlib wrappers)
        // These are thin wrappers around Rust stdlib and should have zero overhead
        if self.is_module {
            return true;
        }

        // Count statements in function body
        let statement_count = self.count_statements(&func.body);

        // Inline small functions (< 10 statements)
        if statement_count < 10 {
            return true;
        }

        // Inline trivial single-expression functions
        if statement_count == 1 {
            if let Statement::Return { value: Some(_), .. } = &func.body[0] {
                return true;
            }
            if let Statement::Expression { .. } = &func.body[0] {
                return true;
            }
        }

        // Default: don't inline large functions
        false
    }

    /// Check if we should add & for borrowed iteration in a for loop
    /// Returns true if iterating over a field of a borrowed parameter
    fn should_borrow_for_iteration(&self, iterable: &Expression) -> bool {
        match iterable {
            // Field access on a variable (e.g., game.walls)
            Expression::FieldAccess { object, .. } => {
                // Check if the object is a simple identifier
                if let Expression::Identifier { .. } = &**object {
                    // Check if this is a parameter in the current function
                    // For game decorator functions, the first parameter is always borrowed
                    // For impl methods, self is borrowed
                    // For now, we'll use a simple heuristic: if it's a field access, assume borrowed
                    return true;
                }
                false
            }
            _ => false,
        }
    }

    /// Extract the identifier from a pattern (for for-loop variable names)
    fn extract_pattern_identifier(&self, pattern: &Pattern) -> Option<String> {
        match pattern {
            Pattern::Identifier(name) => Some(name.clone()),
            _ => None,
        }
    }

    /// Check if a loop body modifies a variable
    fn loop_body_modifies_variable(&self, body: &[Statement], var_name: &str) -> bool {
        for stmt in body {
            if self.statement_modifies_variable(stmt, var_name) {
                return true;
            }
        }
        false
    }

    /// Check if a statement modifies a variable
    fn statement_modifies_variable(&self, stmt: &Statement, var_name: &str) -> bool {
        match stmt {
            Statement::Assignment { target, .. } => {
                // Check if we're assigning to var_name or var_name.field
                self.expression_references_variable_or_field(target, var_name)
            }
            Statement::If {
                then_block,
                else_block,
                ..
            } => {
                then_block
                    .iter()
                    .any(|s| self.statement_modifies_variable(s, var_name))
                    || else_block.as_ref().is_some_and(|block| {
                        block
                            .iter()
                            .any(|s| self.statement_modifies_variable(s, var_name))
                    })
            }
            Statement::While { body, .. } | Statement::For { body, .. } => body
                .iter()
                .any(|s| self.statement_modifies_variable(s, var_name)),
            _ => false,
        }
    }

    /// Check if an expression references a variable or its fields
    #[allow(clippy::only_used_in_recursion)]
    fn expression_references_variable_or_field(&self, expr: &Expression, var_name: &str) -> bool {
        match expr {
            Expression::Identifier { name, .. } => name == var_name,
            Expression::FieldAccess { object, .. } => {
                // Check if object is the variable
                if let Expression::Identifier { name, .. } = &**object {
                    name == var_name
                } else {
                    self.expression_references_variable_or_field(object, var_name)
                }
            }
            _ => false,
        }
    }

    /// Check if we should add &mut for index access on borrowed fields
    /// e.g., self.enemies[i] should become &mut self.enemies[i] in let bindings
    fn should_mut_borrow_index_access(&self, expr: &Expression) -> bool {
        match expr {
            Expression::Index { object, .. } => {
                // Check if we're indexing into a field access (e.g., self.enemies[i])
                if let Expression::FieldAccess {
                    object: field_obj, ..
                } = &**object
                {
                    // Check if the field is accessed on self or a borrowed parameter
                    if let Expression::Identifier { .. } = &**field_obj {
                        // For now, assume self and first parameter are borrowed
                        return true; // Conservative: always borrow index access (including self)
                    }
                }
                false
            }
            _ => false,
        }
    }

    /// Count statements in a function body (for inline heuristics)
    fn count_statements(&self, body: &[Statement]) -> usize {
        let mut count = 0;
        for stmt in body {
            count += match stmt {
                Statement::Let { .. } => 1,
                Statement::Const { .. } => 1,
                Statement::Static { .. } => 1,
                Statement::Return { .. } => 1,
                Statement::Expression { .. } => 1,
                Statement::If { .. } => 3, // Weighted more heavily
                Statement::While { .. } => 3,
                Statement::Loop { .. } => 3,
                Statement::For { .. } => 3,
                Statement::Match { .. } => 5, // Match statements are complex
                Statement::Assignment { .. } => 1,
                Statement::Thread { .. } => 2, // Thread spawn
                Statement::Async { .. } => 2,  // Async spawn
                Statement::Defer { .. } => 1,
                Statement::Break { .. } => 1,
                Statement::Continue { .. } => 1,
                Statement::Use { .. } => 0, // Use statements don't affect complexity
            };
        }
        count
    }

    // Format type parameters with trait bounds for Rust output
    // Example: [TypeParam { name: "T", bounds: ["Display", "Clone"] }] -> "T: Display + Clone"
    fn format_type_params(&self, type_params: &[crate::parser::TypeParam]) -> String {
        type_params
            .iter()
            .map(|param| {
                if param.bounds.is_empty() {
                    param.name.clone()
                } else {
                    // Expand bound aliases
                    let expanded_bounds: Vec<String> = param
                        .bounds
                        .iter()
                        .flat_map(|bound| {
                            // Check if this bound is an alias
                            if let Some(traits) = self.bound_aliases.get(bound) {
                                traits.clone()
                            } else {
                                vec![bound.clone()]
                            }
                        })
                        .collect();
                    format!("{}: {}", param.name, expanded_bounds.join(" + "))
                }
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    // Format where clause for Rust output
    // Example: [("T", ["Display"]), ("U", ["Debug", "Clone"])] -> "\nwhere\n    T: Display,\n    U: Debug + Clone"
    fn format_where_clause(&self, where_clause: &[(String, Vec<String>)]) -> String {
        if where_clause.is_empty() {
            return String::new();
        }

        let clauses: Vec<String> = where_clause
            .iter()
            .map(|(type_param, bounds)| format!("    {}: {}", type_param, bounds.join(" + ")))
            .collect();

        format!("\nwhere\n{}", clauses.join(",\n"))
    }

    /// PHASE 6 OPTIMIZATION: Wrap function body with defer drop logic
    /// This defers heavy deallocations to a background thread, making functions return 10,000x faster.
    /// Reference: https://abrams.cc/rust-dropping-things-in-another-thread
    ///
    /// Transform:
    ///   let result = compute();
    ///   result
    /// Into:
    ///   let result = compute();
    ///   std::thread::spawn(move || drop(variable));
    ///   result
    fn wrap_with_defer_drop(
        &self,
        body: String,
        optimizations: &[crate::analyzer::DeferDropOptimization],
    ) -> String {
        if optimizations.is_empty() {
            return body;
        }

        let lines: Vec<&str> = body.lines().collect();
        if lines.is_empty() {
            return body;
        }

        let mut new_body = String::new();

        // Find the last non-empty, non-comment line (likely the return expression or last statement)
        let mut last_line_idx = lines.len() - 1;
        while last_line_idx > 0 {
            let trimmed = lines[last_line_idx].trim();
            if !trimmed.is_empty() && !trimmed.starts_with("//") {
                break;
            }
            last_line_idx -= 1;
        }

        // Copy all lines except the last one
        for (i, line) in lines.iter().enumerate() {
            if i < last_line_idx {
                new_body.push_str(line);
                new_body.push('\n');
            }
        }

        // Insert defer drop statements before the final return/expression
        for opt in optimizations {
            // Generate the defer drop code
            new_body.push_str(&self.indent());
            new_body.push_str(&format!(
                "// DEFER DROP: Deallocate {} ({:?}) in background thread for faster return\n",
                opt.variable, opt.estimated_size
            ));
            new_body.push_str(&self.indent());
            new_body.push_str(&format!(
                "std::thread::spawn(move || drop({}));\n",
                opt.variable
            ));
        }

        // Add the final line (return expression or last statement)
        new_body.push_str(lines[last_line_idx]);

        // Add any trailing lines (closing braces, etc.)
        for line in &lines[last_line_idx + 1..] {
            new_body.push('\n');
            new_body.push_str(line);
        }

        new_body
    }

    /// PHASE 7: Check if an expression can be evaluated at compile time
    /// If true, we can use `const` instead of `static`
    #[allow(clippy::only_used_in_recursion)]
    fn is_const_evaluable(&self, expr: &Expression) -> bool {
        match expr {
            // Literals are always const
            Expression::Literal { .. } => true,

            // Binary operations on const values are const
            Expression::Binary { left, right, .. } => {
                self.is_const_evaluable(left) && self.is_const_evaluable(right)
            }

            // Unary operations on const values are const
            Expression::Unary { operand, .. } => self.is_const_evaluable(operand),

            // Struct literals with const fields might be const
            Expression::StructLiteral { fields, .. } => {
                fields.iter().all(|(_, expr)| self.is_const_evaluable(expr))
            }

            // Map literals with const entries might be const
            Expression::MapLiteral { pairs, .. } => pairs
                .iter()
                .all(|(k, v)| self.is_const_evaluable(k) && self.is_const_evaluable(v)),

            // Most other expressions are not const-evaluable
            _ => false,
        }
    }

    /// Generate automatic trait implementation for @component decorator
    fn generate_component_impl(&mut self, s: &StructDecl) -> String {
        let mut output = String::new();

        // For now, generate a marker comment
        // In future iterations, we'll generate actual trait implementations
        output.push_str(&format!(
            "// Component trait implementation for {}\n// TODO: Implement Component trait",
            s.name
        ));

        output
    }

    /// Generate automatic trait implementation for @game decorator
    fn generate_game_impl(&mut self, s: &StructDecl) -> String {
        let mut output = String::new();

        // Generate Default implementation for game state
        // All fields are initialized to their default values (0, 0.0, false, etc.)
        output.push_str(&format!("impl Default for {} {{\n", s.name));
        output.push_str("    fn default() -> Self {\n");
        output.push_str(&format!("        {} {{\n", s.name));

        for field in &s.fields {
            let default_value = match &field.field_type {
                Type::Int | Type::Int32 | Type::Uint => "0",
                Type::Float => "0.0",
                Type::Bool => "false",
                Type::String => "String::new()",
                Type::Vec(_) => "Vec::new()",
                Type::Custom(name) if name == "String" => "String::new()",
                Type::Custom(name) if name == "Vec3" => "Vec3::new(0.0, 0.0, 0.0)",
                Type::Custom(name) if name.starts_with("Vec") => "Vec::new()",
                _ => "Default::default()",
            };
            output.push_str(&format!("            {}: {},\n", field.name, default_value));
        }

        output.push_str("        }\n");
        output.push_str("    }\n");
        output.push('}');

        output
    }
}

/// Helper function to check if a pattern contains a string literal
/// Extracted to avoid clippy::only_used_in_recursion warning
fn pattern_has_string_literal_impl(pattern: &Pattern) -> bool {
    match pattern {
        Pattern::Literal(Literal::String(_)) => true,
        Pattern::Tuple(patterns) => patterns.iter().any(pattern_has_string_literal_impl),
        Pattern::Or(patterns) => patterns.iter().any(pattern_has_string_literal_impl),
        _ => false,
    }
}
