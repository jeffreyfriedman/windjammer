// Windjammer Game Editor - Tauri Backend
// Frontend is built with windjammer-ui (WASM)

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Read a file from the filesystem
#[tauri::command]
fn read_file(path: String) -> Result<String, String> {
    fs::read_to_string(&path).map_err(|e| format!("Failed to read file: {}", e))
}

/// Write a file to the filesystem
#[tauri::command]
fn write_file(path: String, content: String) -> Result<(), String> {
    fs::write(&path, content).map_err(|e| format!("Failed to write file: {}", e))
}

/// List files in a directory
#[tauri::command]
fn list_directory(path: String) -> Result<Vec<FileEntry>, String> {
    let entries = fs::read_dir(&path).map_err(|e| format!("Failed to read directory: {}", e))?;

    let mut files = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let path = entry.path();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        files.push(FileEntry {
            name,
            path: path.to_string_lossy().to_string(),
            is_directory: path.is_dir(),
        });
    }

    Ok(files)
}

/// Create a new Windjammer game project
#[tauri::command]
fn create_game_project(path: String, name: String, template: String) -> Result<(), String> {
    let project_path = PathBuf::from(&path).join(&name);
    fs::create_dir_all(&project_path)
        .map_err(|e| format!("Failed to create project directory: {}", e))?;

    // Get template content
    let main_wj = match template.as_str() {
        "platformer" => get_platformer_template(&name),
        "puzzle" => get_puzzle_template(&name),
        "shooter" => get_shooter_template(&name),
        _ => get_platformer_template(&name), // Default to platformer
    };

    fs::write(project_path.join("main.wj"), main_wj)
        .map_err(|e| format!("Failed to create main.wj: {}", e))?;

    Ok(())
}

fn get_platformer_template(name: &str) -> String {
    format!(
        r#"// {} - A Platformer Game
use std::game::*

// Game state
struct {} {{
    player_x: f32,
    player_y: f32,
    velocity_y: f32,
    on_ground: bool,
}}

// Initialize the game
fn init() -> {} {{
    {} {{
        player_x: 400.0,
        player_y: 300.0,
        velocity_y: 0.0,
        on_ground: true,
    }}
}}

// Update game logic
fn update(game: {}, input: Input, dt: f32) -> {} {{
    let mut new_game = game
    
    // Horizontal movement
    if input.is_key_down(Key::Left) {{
        new_game.player_x = new_game.player_x - 200.0 * dt
    }}
    if input.is_key_down(Key::Right) {{
        new_game.player_x = new_game.player_x + 200.0 * dt
    }}
    
    // Jumping
    if input.is_key_pressed(Key::Space) && new_game.on_ground {{
        new_game.velocity_y = -500.0
        new_game.on_ground = false
    }}
    
    // Apply gravity
    if !new_game.on_ground {{
        new_game.velocity_y = new_game.velocity_y + 980.0 * dt
        new_game.player_y = new_game.player_y + new_game.velocity_y * dt
        
        // Ground collision
        if new_game.player_y >= 500.0 {{
            new_game.player_y = 500.0
            new_game.velocity_y = 0.0
            new_game.on_ground = true
        }}
    }}
    
    new_game
}}

// Render the game
fn render(game: {}, renderer: Renderer) {{
    // Clear screen
    renderer.clear(Color::rgb(0.53, 0.81, 0.92))
    
    // Draw ground
    renderer.draw_rect(0.0, 500.0, 800.0, 100.0, Color::rgb(0.4, 0.3, 0.2))
    
    // Draw player
    renderer.draw_rect(
        game.player_x - 25.0,
        game.player_y - 50.0,
        50.0,
        50.0,
        Color::rgb(1.0, 0.3, 0.3)
    )
}}

// Main game loop
fn main() {{
    let mut game = init()
    let input = Input::new()
    let renderer = Renderer::new()
    
    game = update(game, input, 0.016)
    render(game, renderer)
    
    println!("Platformer game initialized!")
}}
"#,
        name, name, name, name, name, name, name
    )
}

fn get_puzzle_template(name: &str) -> String {
    format!(
        r#"// {} - A Puzzle Game
use std::game::*

// Game state
struct {} {{
    grid: Vec<Vec<i32>>,
    selected_x: i32,
    selected_y: i32,
}}

// Initialize the game
fn init() -> {} {{
    let grid = vec![
        vec![1, 2, 3],
        vec![4, 0, 5],
        vec![6, 7, 8],
    ]
    
    {} {{
        grid: grid,
        selected_x: 1,
        selected_y: 1,
    }}
}}

// Update game logic
fn update(game: {}, input: Input, dt: f32) -> {} {{
    let mut new_game = game
    
    // Move selection
    if input.is_key_pressed(Key::Left) && new_game.selected_x > 0 {{
        new_game.selected_x = new_game.selected_x - 1
    }}
    if input.is_key_pressed(Key::Right) && new_game.selected_x < 2 {{
        new_game.selected_x = new_game.selected_x + 1
    }}
    if input.is_key_pressed(Key::Up) && new_game.selected_y > 0 {{
        new_game.selected_y = new_game.selected_y - 1
    }}
    if input.is_key_pressed(Key::Down) && new_game.selected_y < 2 {{
        new_game.selected_y = new_game.selected_y + 1
    }}
    
    new_game
}}

// Render the game
fn render(game: {}, renderer: Renderer) {{
    // Clear screen
    renderer.clear(Color::rgb(0.15, 0.15, 0.2))
    
    // Draw grid
    let cell_size = 100.0
    let start_x = 250.0
    let start_y = 150.0
    
    // Draw each cell (simplified - would need actual grid iteration)
    renderer.draw_rect(
        start_x + (game.selected_x as f32) * cell_size,
        start_y + (game.selected_y as f32) * cell_size,
        cell_size,
        cell_size,
        Color::rgb(0.3, 0.6, 0.9)
    )
}}

// Main game loop
fn main() {{
    let mut game = init()
    let input = Input::new()
    let renderer = Renderer::new()
    
    game = update(game, input, 0.016)
    render(game, renderer)
    
    println!("Puzzle game initialized!")
}}
"#,
        name, name, name, name, name, name, name
    )
}

fn get_shooter_template(name: &str) -> String {
    format!(
        r#"// {} - A Shooter Game
use std::game::*

// Game state
struct {} {{
    ship_x: f32,
    ship_y: f32,
    bullets: Vec<Bullet>,
    score: i32,
}}

struct Bullet {{
    x: f32,
    y: f32,
}}

// Initialize the game
fn init() -> {} {{
    {} {{
        ship_x: 400.0,
        ship_y: 500.0,
        bullets: vec![],
        score: 0,
    }}
}}

// Update game logic
fn update(game: {}, input: Input, dt: f32) -> {} {{
    let mut new_game = game
    
    // Ship movement
    if input.is_key_down(Key::Left) {{
        new_game.ship_x = new_game.ship_x - 300.0 * dt
    }}
    if input.is_key_down(Key::Right) {{
        new_game.ship_x = new_game.ship_x + 300.0 * dt
    }}
    
    // Keep ship in bounds
    if new_game.ship_x < 25.0 {{
        new_game.ship_x = 25.0
    }}
    if new_game.ship_x > 775.0 {{
        new_game.ship_x = 775.0
    }}
    
    // Shoot bullets
    if input.is_key_pressed(Key::Space) {{
        let bullet = Bullet {{
            x: new_game.ship_x,
            y: new_game.ship_y - 30.0,
        }}
        // Would push to bullets vec here
    }}
    
    new_game
}}

// Render the game
fn render(game: {}, renderer: Renderer) {{
    // Clear screen
    renderer.clear(Color::rgb(0.0, 0.0, 0.1))
    
    // Draw ship
    renderer.draw_rect(
        game.ship_x - 25.0,
        game.ship_y - 15.0,
        50.0,
        30.0,
        Color::rgb(0.2, 0.8, 0.3)
    )
    
    // Draw score (would need text rendering)
    println!("Score: {{}}", game.score)
}}

// Main game loop
fn main() {{
    let mut game = init()
    let input = Input::new()
    let renderer = Renderer::new()
    
    game = update(game, input, 0.016)
    render(game, renderer)
    
    println!("Shooter game initialized!")
}}
"#,
        name, name, name, name, name, name, name
    )
}

/// Compile and run a Windjammer game
#[tauri::command]
fn run_game(project_path: String) -> Result<String, String> {
    // Find the windjammer compiler
    let compiler_path = find_windjammer_compiler()?;

    // Compile the game
    let output = Command::new(&compiler_path)
        .arg("build")
        .arg(&project_path)
        .output()
        .map_err(|e| format!("Failed to run compiler: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "Compilation failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(format!(
        "Compilation successful!\n{}",
        String::from_utf8_lossy(&output.stdout)
    ))
}

/// Stop a running game
#[tauri::command]
fn stop_game() -> Result<(), String> {
    // TODO: Implement game process management
    Ok(())
}

fn find_windjammer_compiler() -> Result<String, String> {
    // Get the directory where the editor binary is located
    let exe_path =
        std::env::current_exe().map_err(|e| format!("Failed to get executable path: {}", e))?;
    let exe_dir = exe_path
        .parent()
        .ok_or("Failed to get executable directory")?;

    // Try to find the compiler in various locations
    let mut possible_paths = vec![
        // Same directory as editor (release build)
        exe_dir.join("windjammer"),
        exe_dir.join("wj"),
    ];

    // Add parent directory paths
    if let Some(parent) = exe_dir.parent() {
        if let Some(grandparent) = parent.parent() {
            possible_paths.push(grandparent.join("windjammer"));
            possible_paths.push(grandparent.join("wj"));
        }
    }

    // Add workspace paths
    possible_paths.extend(vec![
        PathBuf::from("../../target/debug/windjammer"),
        PathBuf::from("../../target/release/windjammer"),
        PathBuf::from("../../target/debug/wj"),
        PathBuf::from("../../target/release/wj"),
        PathBuf::from("./target/debug/windjammer"),
        PathBuf::from("./target/release/windjammer"),
    ]);

    // Clone for error message before consuming
    let paths_for_error = possible_paths.clone();

    for path in possible_paths {
        if path.exists() {
            return Ok(path.to_string_lossy().to_string());
        }
    }

    // Try PATH
    if let Ok(output) = Command::new("which").arg("windjammer").output() {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Ok(path);
            }
        }
    }

    if let Ok(output) = Command::new("which").arg("wj").output() {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Ok(path);
            }
        }
    }

    Err(format!(
        "Windjammer compiler not found. Searched in:\n{}\n\nPlease ensure the compiler is built and in your PATH.",
        paths_for_error.iter().map(|p| format!("  - {}", p.display())).collect::<Vec<_>>().join("\n")
    ))
}

#[derive(serde::Serialize)]
struct FileEntry {
    name: String,
    path: String,
    is_directory: bool,
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            read_file,
            write_file,
            list_directory,
            create_game_project,
            run_game,
            stop_game,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
