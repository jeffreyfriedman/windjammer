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
    app = App()
    
    # Add a simple system using decorator
    @app.system
    def hello_system():
        print("Hello from the game loop!")
    
    print("Application created successfully!")
    print("Systems registered: 1")
    print()
    print("Note: Full app.run() would start the game loop")
    print("For this example, we're just demonstrating SDK setup")
    print()
    
    # Run the application
    app.run()

if __name__ == "__main__":
    main()

