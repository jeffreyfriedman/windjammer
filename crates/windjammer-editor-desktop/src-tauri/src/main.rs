// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::process::Command;
use serde::{Deserialize, Serialize};

// ============================================================================
// FILE SYSTEM COMMANDS
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
struct FileEntry {
    name: String,
    path: String,
    is_directory: bool,
}

#[tauri::command]
async fn read_file(path: String) -> Result<String, String> {
    std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read file: {}", e))
}

#[tauri::command]
async fn write_file(path: String, content: String) -> Result<(), String> {
    std::fs::write(&path, content)
        .map_err(|e| format!("Failed to write file: {}", e))
}

#[tauri::command]
async fn create_directory(path: String) -> Result<(), String> {
    std::fs::create_dir_all(&path)
        .map_err(|e| format!("Failed to create directory: {}", e))
}

#[tauri::command]
async fn list_directory(path: String) -> Result<Vec<FileEntry>, String> {
    let entries = std::fs::read_dir(&path)
        .map_err(|e| format!("Failed to read directory: {}", e))?;
    
    let mut result = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let metadata = entry.metadata().map_err(|e| format!("Failed to read metadata: {}", e))?;
        let path = entry.path();
        let name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("?")
            .to_string();
        
        result.push(FileEntry {
            name,
            path: path.to_string_lossy().to_string(),
            is_directory: metadata.is_dir(),
        });
    }
    
    Ok(result)
}

#[tauri::command]
async fn delete_file(path: String) -> Result<(), String> {
    std::fs::remove_file(&path)
        .map_err(|e| format!("Failed to delete file: {}", e))
}

#[tauri::command]
async fn file_exists(path: String) -> Result<bool, String> {
    Ok(PathBuf::from(path).exists())
}

// ============================================================================
// PROCESS COMMANDS
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
struct CommandOutput {
    stdout: String,
    stderr: String,
    exit_code: i32,
}

#[tauri::command]
async fn execute_command(command: String, args: Vec<String>) -> Result<CommandOutput, String> {
    let output = Command::new(&command)
        .args(&args)
        .output()
        .map_err(|e| format!("Failed to execute command: {}", e))?;
    
    Ok(CommandOutput {
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        exit_code: output.status.code().unwrap_or(-1),
    })
}

#[tauri::command]
async fn current_dir() -> Result<String, String> {
    std::env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .map_err(|e| format!("Failed to get current directory: {}", e))
}

#[tauri::command]
async fn set_current_dir(path: String) -> Result<(), String> {
    std::env::set_current_dir(&path)
        .map_err(|e| format!("Failed to set current directory: {}", e))
}

// ============================================================================
// DIALOG COMMANDS
// ============================================================================

// Note: Tauri 2.0 has built-in dialog APIs, but we'll keep these simple versions
// for compatibility with our Windjammer stdlib

#[tauri::command]
async fn show_message(message: String) -> Result<(), String> {
    // In Tauri 2.0, use tauri::api::dialog
    println!("Message: {}", message);
    Ok(())
}

// ============================================================================
// MAIN
// ============================================================================

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            // File system
            read_file,
            write_file,
            create_directory,
            list_directory,
            delete_file,
            file_exists,
            // Process
            execute_command,
            current_dir,
            set_current_dir,
            // Dialog
            show_message,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

