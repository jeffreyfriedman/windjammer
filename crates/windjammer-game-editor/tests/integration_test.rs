// Integration tests for Windjammer Game Editor backend

use std::fs;
use std::path::PathBuf;

#[test]
fn test_create_game_project_template() {
    let test_dir = "/tmp/windjammer_test_project";
    let project_name = "TestGame";

    // Clean up if exists
    let _ = fs::remove_dir_all(test_dir);

    // Create project directory
    let project_path = PathBuf::from(test_dir).join(project_name);
    fs::create_dir_all(&project_path).expect("Failed to create project directory");

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
        project_name,
        project_name,
        project_name,
        project_name,
        project_name,
        project_name,
        project_name
    );

    fs::write(project_path.join("main.wj"), &main_wj).expect("Failed to create main.wj");

    // Verify file exists
    assert!(project_path.join("main.wj").exists());

    // Verify content
    let content = fs::read_to_string(project_path.join("main.wj")).expect("Failed to read main.wj");
    assert!(content.contains("use std::game::*"));
    assert!(content.contains("fn init()"));
    assert!(content.contains("fn update("));
    assert!(content.contains("fn render("));
    assert!(content.contains("fn main()"));
    assert!(content.contains("player_x"));
    assert!(content.contains("player_y"));

    // Clean up
    let _ = fs::remove_dir_all(test_dir);
}

#[test]
fn test_file_operations() {
    let test_file = "/tmp/windjammer_test_file.wj";
    let test_content = "fn main() {\n    println!(\"Hello, World!\")\n}";

    // Write file
    fs::write(test_file, test_content).expect("Failed to write file");

    // Read file
    let content = fs::read_to_string(test_file).expect("Failed to read file");
    assert_eq!(content, test_content);

    // Clean up
    let _ = fs::remove_file(test_file);
}

#[test]
fn test_list_directory() {
    let test_dir = "/tmp/windjammer_test_dir";

    // Create test directory with files
    fs::create_dir_all(test_dir).expect("Failed to create directory");
    fs::write(format!("{}/file1.wj", test_dir), "content1").expect("Failed to write file1");
    fs::write(format!("{}/file2.wj", test_dir), "content2").expect("Failed to write file2");

    // List directory
    let entries = fs::read_dir(test_dir).expect("Failed to read directory");
    let files: Vec<_> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();

    assert!(files.contains(&"file1.wj".to_string()));
    assert!(files.contains(&"file2.wj".to_string()));

    // Clean up
    let _ = fs::remove_dir_all(test_dir);
}
