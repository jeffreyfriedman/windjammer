#!/usr/bin/env python3
"""
Hello World Example

The simplest possible Windjammer application in Python.

Run with: python examples/hello_world.py
"""

from windjammer_sdk import App

def main():
    print("=== Windjammer Hello World (Python) ===")
    print(f"SDK Version: {__import__('windjammer_sdk').__version__}")
    print()
    
    # Create a new application
    app = App(title="Hello World")
    
    # Add a simple system
    app.add_system(lambda: print("Hello from the game loop!"))
    
    print("Application created successfully!")
    print("Systems registered: 1")
    print()
    
    # Run the application
    app.run()

if __name__ == "__main__":
    main()

