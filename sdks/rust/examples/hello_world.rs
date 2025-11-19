//! Hello World Example
//!
//! The simplest possible Windjammer application.
//!
//! Run with: cargo run --example hello_world

use windjammer_sdk::prelude::*;

fn main() {
    println!("=== Windjammer Hello World ===");
    println!("{}", version::version_string());
    println!();
    
    // Create a new application
    let mut app = App::new();
    
    // Add a simple system
    app.add_system(hello_system);
    
    println!("Application created successfully!");
    println!("Systems registered: 1");
    println!();
    println!("Note: Full app.run() would start the game loop");
    println!("For this example, we're just demonstrating SDK setup");
}

fn hello_system() {
    println!("Hello from the game loop!");
}

