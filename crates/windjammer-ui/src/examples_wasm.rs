//! WASM examples entry point
//! 
//! This module exports example functions that can be called from JavaScript

use wasm_bindgen::prelude::*;

/// Initialize and run the interactive counter example
#[wasm_bindgen]
pub fn run_interactive_counter() {
    // Set panic hook for better error messages
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();

    web_sys::console::log_1(&"🚀 Running interactive counter example...".into());
    
    // Implementation will go here
    web_sys::console::log_1(&"✅ Counter example loaded!".into());
}

