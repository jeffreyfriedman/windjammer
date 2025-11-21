use js_sys::{Array, Function, Reflect};
/// WASM implementation of std::process
/// Automatically uses Tauri when available (desktop), falls back to mock (browser)
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use web_sys::{console, window};

#[derive(Debug, Clone)]
pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

/// Check if Tauri is available
fn is_tauri() -> bool {
    window()
        .and_then(|w| Reflect::get(&w, &JsValue::from_str("__TAURI__")).ok())
        .map(|v| !v.is_undefined())
        .unwrap_or(false)
}

/// Execute a command (Tauri or mocked)
pub fn execute(command: String, args: Vec<String>) -> Result<CommandOutput, String> {
    if is_tauri() {
        execute_tauri(command, args)
    } else {
        execute_mock(command, args)
    }
}

/// Execute command using Tauri
fn execute_tauri(command: String, args: Vec<String>) -> Result<CommandOutput, String> {
    let window = window().ok_or("No window available")?;
    let tauri_bridge = Reflect::get(&window, &JsValue::from_str("__WINDJAMMER_TAURI__"))
        .map_err(|_| "Tauri bridge not found")?;
    let exec_fn = Reflect::get(&tauri_bridge, &JsValue::from_str("executeCommandSync"))
        .map_err(|_| "executeCommandSync function not found")?;
    let exec_fn = exec_fn
        .dyn_into::<Function>()
        .map_err(|_| "executeCommandSync is not a function")?;

    // Convert args to JS array
    let js_args = Array::new();
    for arg in args {
        js_args.push(&JsValue::from_str(&arg));
    }

    exec_fn
        .call2(&JsValue::NULL, &JsValue::from_str(&command), &js_args)
        .map_err(|e| format!("Failed to call executeCommandSync: {:?}", e))?;

    // Return a placeholder result (actual result will be async)
    Ok(CommandOutput {
        stdout: format!("✓ Executing: {} {}", command, args.join(" ")),
        stderr: String::new(),
        exit_code: 0,
    })
}

/// Execute command (mocked in browser - logs to console)
fn execute_mock(command: String, args: Vec<String>) -> Result<CommandOutput, String> {
    // Log the command to console
    let full_command = format!("{} {}", command, args.join(" "));
    console::log_1(&JsValue::from_str(&format!(
        "🎮 Mock Execute: {}",
        full_command
    )));

    // Simulate different commands
    if command.contains("wj") {
        if args.iter().any(|a| a.contains("new")) {
            // Simulate project creation
            console::log_1(&JsValue::from_str("✅ Created new Windjammer project"));
            Ok(CommandOutput {
                stdout: "Project created successfully!\nCreated: src/main.wj\nCreated: wj.toml"
                    .to_string(),
                stderr: String::new(),
                exit_code: 0,
            })
        } else if args.iter().any(|a| a.contains("build")) {
            // Simulate build
            console::log_1(&JsValue::from_str("🔨 Building project..."));
            Ok(CommandOutput {
                stdout: "Compiling...\nBuild successful!".to_string(),
                stderr: String::new(),
                exit_code: 0,
            })
        } else if args.iter().any(|a| a.contains("run")) {
            // Simulate run
            console::log_1(&JsValue::from_str("🚀 Running project..."));
            Ok(CommandOutput {
                stdout: "Running game...\nGame started successfully!".to_string(),
                stderr: String::new(),
                exit_code: 0,
            })
        } else {
            Ok(CommandOutput {
                stdout: format!("Executed: {}", full_command),
                stderr: String::new(),
                exit_code: 0,
            })
        }
    } else {
        // Generic command
        console::log_1(&JsValue::from_str(&format!(
            "⚠️  Command not fully supported in browser: {}",
            command
        )));
        Ok(CommandOutput {
            stdout: format!("Mock execution of: {}", full_command),
            stderr: "Note: Running in browser with limited process support".to_string(),
            exit_code: 0,
        })
    }
}

/// Spawn a background process (not supported in browser)
pub fn spawn(command: String, args: Vec<String>) -> Result<u32, String> {
    console::log_1(&JsValue::from_str(&format!(
        "⚠️  spawn() not supported in browser: {} {}",
        command,
        args.join(" ")
    )));
    Err(
        "Process spawning not supported in browser. Use Web Workers for background tasks."
            .to_string(),
    )
}

/// Kill a process (not supported in browser)
pub fn kill(pid: u32) -> Result<(), String> {
    console::log_1(&JsValue::from_str(&format!(
        "⚠️  kill() not supported in browser: PID {}",
        pid
    )));
    Err("Process killing not supported in browser".to_string())
}

/// Get current directory (simulated in browser)
pub fn current_dir() -> Result<String, String> {
    Ok("/workspace".to_string())
}

/// Set current directory (simulated in browser)
pub fn set_current_dir(path: String) -> Result<(), String> {
    console::log_1(&JsValue::from_str(&format!(
        "📁 Current directory set to: {}",
        path
    )));
    Ok(())
}

/// Get environment variable (from browser)
pub fn env(key: String) -> Option<String> {
    // In browser, we don't have real env vars
    // Could use localStorage or hardcoded values
    match key.as_str() {
        "HOME" => Some("/home/user".to_string()),
        "USER" => Some("browser-user".to_string()),
        "PATH" => Some("/usr/local/bin:/usr/bin:/bin".to_string()),
        _ => None,
    }
}

/// Exit the process (close the browser tab/window)
pub fn exit(code: i32) -> ! {
    console::log_1(&JsValue::from_str(&format!(
        "🛑 Exit called with code: {}",
        code
    )));

    if let Some(window) = web_sys::window() {
        let _ = window.close();
    }

    // If we can't close the window, just loop forever
    loop {
        std::hint::spin_loop();
    }
}
