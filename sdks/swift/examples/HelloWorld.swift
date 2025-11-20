#!/usr/bin/env swift
/**
 * Hello World Example
 * 
 * The simplest possible Windjammer application in Swift.
 * 
 * Run with: swift examples/HelloWorld.swift
 */

import Foundation

print("=== Windjammer Hello World (Swift) ===")
print("SDK Version: \(Windjammer.VERSION)")
print()

// Create a new application
let app = App()

// Add a simple system
app.addSystem {
    print("Hello from the game loop!")
}

print("Application created successfully!")
print("Systems registered: 1")
print()
print("Note: Full app.run() would start the game loop")
print("For this example, we're just demonstrating SDK setup")
print()

// Run the application
app.run()

