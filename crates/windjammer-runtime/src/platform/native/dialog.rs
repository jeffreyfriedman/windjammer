//! Native dialog implementation
//!
//! Uses the rfd crate for native file dialogs.

// For now, provide stub implementations
// TODO: Add rfd crate and implement properly

pub fn open_file() -> Result<String, String> {
    Err("Dialog not yet implemented for native platform".to_string())
}

pub fn open_directory() -> Result<String, String> {
    Err("Dialog not yet implemented for native platform".to_string())
}

pub fn save_file() -> Result<String, String> {
    Err("Dialog not yet implemented for native platform".to_string())
}

pub fn show_message(title: String, message: String) -> Result<(), String> {
    println!("[{}] {}", title, message);
    Ok(())
}

pub fn show_confirm(title: String, message: String) -> Result<bool, String> {
    println!("[{}] {}", title, message);
    println!("(Auto-confirmed for native platform)");
    Ok(true)
}
