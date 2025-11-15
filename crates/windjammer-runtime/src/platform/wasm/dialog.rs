/// WASM implementation of std::dialog using browser dialogs
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use web_sys::window;

pub type DialogResult<T> = Result<T, String>;

/// Show an alert dialog
pub fn alert(message: String) -> DialogResult<()> {
    window()
        .ok_or("No window available")?
        .alert_with_message(&message)
        .map_err(|e| format!("Failed to show alert: {:?}", e))
}

/// Show a confirm dialog
pub fn confirm(message: String) -> DialogResult<bool> {
    window()
        .ok_or("No window available")?
        .confirm_with_message(&message)
        .map_err(|e| format!("Failed to show confirm: {:?}", e))
}

/// Show a prompt dialog
pub fn prompt(message: String, default: String) -> DialogResult<Option<String>> {
    let result = window()
        .ok_or("No window available")?
        .prompt_with_message_and_default(&message, &default)
        .map_err(|e| format!("Failed to show prompt: {:?}", e))?;

    Ok(result)
}

/// Show an error dialog
pub fn error(message: String) -> DialogResult<()> {
    // Use alert for errors in browser
    alert(format!("Error: {}", message))
}

/// Show an info dialog
pub fn info(message: String) -> DialogResult<()> {
    alert(format!("Info: {}", message))
}

/// Show a warning dialog
pub fn warning(message: String) -> DialogResult<()> {
    alert(format!("Warning: {}", message))
}
