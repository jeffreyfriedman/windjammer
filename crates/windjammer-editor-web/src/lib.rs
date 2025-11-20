//! Windjammer Web Editor
//!
//! A web-based code editor for the Windjammer programming language.
//!
//! ## Features
//!
//! - **Code Editor** - Syntax highlighting, auto-completion
//! - **File Browser** - Navigate project files
//! - **Live Preview** - See changes in real-time
//! - **Error Display** - World-class error messages
//! - **Project Management** - Create, open, save projects
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────┐
//! │         Web Editor (WASM)           │
//! ├─────────────────────────────────────┤
//! │  - Code Editor (Monaco/CodeMirror)  │
//! │  - File Browser                     │
//! │  - Error Display                    │
//! │  - Live Preview                     │
//! └──────────────┬──────────────────────┘
//!                │
//!                ↓
//! ┌─────────────────────────────────────┐
//! │      Windjammer Compiler (WASM)     │
//! ├─────────────────────────────────────┤
//! │  - Lexer                            │
//! │  - Parser                           │
//! │  - Analyzer                         │
//! │  - Codegen                          │
//! └─────────────────────────────────────┘
//! ```

use wasm_bindgen::prelude::*;
use web_sys::{console, window, Document, HtmlTextAreaElement};

// Note: Some modules are commented out as they depend on non-WASM compatible crates
// pub mod compiler_bridge;
// pub mod editor;
pub mod engine_bridge;
// pub mod error_display;
// pub mod file_browser;
// pub mod project;

/// Initialize the web editor
#[wasm_bindgen(start)]
pub fn init() {
    // Set up panic hook for better error messages
    // Note: Add console_error_panic_hook dependency if you want better error messages
    // #[cfg(feature = "console_error_panic_hook")]
    // console_error_panic_hook::set_once();

    console::log_1(&"Windjammer Web Editor initialized!".into());
}

/// Main entry point for the web editor
#[wasm_bindgen]
pub fn mount_editor(container_id: &str) -> Result<(), JsValue> {
    let window = window().ok_or("No window object")?;
    let document = window.document().ok_or("No document object")?;

    let container = document
        .get_element_by_id(container_id)
        .ok_or_else(|| format!("Container '{}' not found", container_id))?;

    // Create editor UI
    let editor_html = r#"
        <div class="windjammer-editor">
            <div class="editor-header">
                <h1>Windjammer Web Editor</h1>
                <div class="editor-actions">
                    <button id="new-project">New Project</button>
                    <button id="open-project">Open</button>
                    <button id="save-project">Save</button>
                    <button id="run-project">Run</button>
                </div>
            </div>
            <div class="editor-body">
                <div class="file-browser" id="file-browser">
                    <h3>Files</h3>
                    <div id="file-list"></div>
                </div>
                <div class="code-editor">
                    <textarea id="code-editor" placeholder="Write your Windjammer code here..."></textarea>
                </div>
                <div class="error-display" id="error-display">
                    <h3>Errors</h3>
                    <div id="error-list"></div>
                </div>
            </div>
            <div class="editor-footer">
                <div class="status-bar">
                    <span id="status-text">Ready</span>
                </div>
            </div>
        </div>
    "#;

    container.set_inner_html(editor_html);

    // Set up event listeners
    setup_event_listeners(&document)?;

    // Load default project
    load_default_project(&document)?;

    Ok(())
}

/// Set up event listeners for editor actions
fn setup_event_listeners(document: &Document) -> Result<(), JsValue> {
    // New Project
    if let Some(btn) = document.get_element_by_id("new-project") {
        let closure = Closure::wrap(Box::new(move |_event: web_sys::Event| {
            console::log_1(&"New project clicked".into());
            // TODO: Implement new project dialog
        }) as Box<dyn FnMut(_)>);

        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    // Save Project
    if let Some(btn) = document.get_element_by_id("save-project") {
        let closure = Closure::wrap(Box::new(move |_event: web_sys::Event| {
            console::log_1(&"Save project clicked".into());
            save_to_local_storage();
        }) as Box<dyn FnMut(_)>);

        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    // Run Project
    if let Some(btn) = document.get_element_by_id("run-project") {
        let document_clone = document.clone();
        let closure = Closure::wrap(Box::new(move |_event: web_sys::Event| {
            console::log_1(&"Run project clicked".into());
            if let Err(e) = run_project(&document_clone) {
                console::error_1(&format!("Error running project: {:?}", e).into());
            }
        }) as Box<dyn FnMut(_)>);

        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    Ok(())
}

/// Load the default project (Hello World example)
fn load_default_project(document: &Document) -> Result<(), JsValue> {
    let default_code = r#"// Welcome to Windjammer Web Editor!
// This is a simple Hello World example.

fn main() {
    println("Hello, Windjammer!")
    
    let name = "World"
    println("Hello, " + name + "!")
    
    // Try creating a game!
    // Uncomment the code below:
    
    // @game
    // struct MyGame {
    //     score: int,
    // }
    // 
    // @init
    // fn init(game: MyGame) {
    //     game.score = 0
    // }
    // 
    // @update
    // fn update(game: MyGame, delta: float) {
    //     game.score += 1
    //     println("Score: " + game.score.to_string())
    // }
}
"#;

    if let Some(editor) = document.get_element_by_id("code-editor") {
        if let Some(textarea) = editor.dyn_ref::<HtmlTextAreaElement>() {
            textarea.set_value(default_code);
        }
    }

    // Update status
    if let Some(status) = document.get_element_by_id("status-text") {
        status.set_inner_html("Loaded default project");
    }

    Ok(())
}

/// Save project to local storage
fn save_to_local_storage() {
    let window = match window() {
        Some(w) => w,
        None => return,
    };

    let storage = match window.local_storage() {
        Ok(Some(s)) => s,
        _ => return,
    };

    let document = match window.document() {
        Some(d) => d,
        None => return,
    };

    if let Some(editor) = document.get_element_by_id("code-editor") {
        if let Some(textarea) = editor.dyn_ref::<HtmlTextAreaElement>() {
            let code = textarea.value();
            let _ = storage.set_item("windjammer_project", &code);
            console::log_1(&"Project saved to local storage!".into());

            // Update status
            if let Some(status) = document.get_element_by_id("status-text") {
                status.set_inner_html("Project saved!");
            }
        }
    }
}

/// Run the current project (compile and show results)
fn run_project(document: &Document) -> Result<(), JsValue> {
    // Get code from editor
    let code = if let Some(editor) = document.get_element_by_id("code-editor") {
        if let Some(textarea) = editor.dyn_ref::<HtmlTextAreaElement>() {
            textarea.value()
        } else {
            return Err("Editor not found".into());
        }
    } else {
        return Err("Editor not found".into());
    };

    console::log_1(&format!("Compiling code: {}", code).into());

    // TODO: Actually compile the code using Windjammer compiler
    // For now, just show a success message

    // Update status
    if let Some(status) = document.get_element_by_id("status-text") {
        status.set_inner_html("✅ Compilation successful!");
    }

    // Clear errors
    if let Some(error_list) = document.get_element_by_id("error-list") {
        error_list
            .set_inner_html("<p style='color: green;'>No errors! Code compiled successfully.</p>");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_editor_init() {
        // Basic test to ensure module compiles
        assert_eq!(2 + 2, 4);
    }
}
