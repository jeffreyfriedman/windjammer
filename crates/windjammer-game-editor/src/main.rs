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
fn create_game_project(path: String, name: String) -> Result<(), String> {
    let project_path = PathBuf::from(&path).join(&name);
    fs::create_dir_all(&project_path)
        .map_err(|e| format!("Failed to create project directory: {}", e))?;

    // Create main.wj with game template
    let main_wj = format!(
        r#"// {} - A Windjammer Game
use std::game::*

// Game state
struct {} {{
    player_x: f32,
    player_y: f32,
}}

// Initialize the game
fn init() -> {} {{
    {} {{
        player_x: 400.0,
        player_y: 300.0,
    }}
}}

// Update game logic
fn update(game: {}, input: Input, dt: f32) -> {} {{
    let mut new_game = game
    
    // Handle input
    if input.is_key_down(Key::Left) {{
        new_game.player_x = new_game.player_x - 200.0 * dt
    }}
    if input.is_key_down(Key::Right) {{
        new_game.player_x = new_game.player_x + 200.0 * dt
    }}
    if input.is_key_down(Key::Up) {{
        new_game.player_y = new_game.player_y - 200.0 * dt
    }}
    if input.is_key_down(Key::Down) {{
        new_game.player_y = new_game.player_y + 200.0 * dt
    }}
    
    new_game
}}

// Render the game
fn render(game: {}, renderer: Renderer) {{
    // Clear screen
    renderer.clear(Color::rgb(0.1, 0.1, 0.15))
    
    // Draw player
    renderer.draw_rect(
        game.player_x - 25.0,
        game.player_y - 25.0,
        50.0,
        50.0,
        Color::rgb(0.2, 0.8, 0.3)
    )
}}

// Main game loop
fn main() {{
    let mut game = init()
    let input = Input::new()
    let renderer = Renderer::new()
    
    // Game loop would go here
    // For now, just test one frame
    game = update(game, input, 0.016)
    render(game, renderer)
    
    println!("Game initialized successfully!")
}}
"#,
        name, name, name, name, name, name, name
    );

    fs::write(project_path.join("main.wj"), main_wj)
        .map_err(|e| format!("Failed to create main.wj: {}", e))?;

    Ok(())
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
    // Try to find the compiler in the workspace
    let possible_paths = vec![
        "../../target/debug/windjammer",
        "../../target/release/windjammer",
        "./target/debug/windjammer",
        "./target/release/windjammer",
        "windjammer", // In PATH
    ];

    for path in possible_paths {
        if PathBuf::from(path).exists() || path == "windjammer" {
            return Ok(path.to_string());
        }
    }

    Err("Windjammer compiler not found. Please build it first.".to_string())
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
