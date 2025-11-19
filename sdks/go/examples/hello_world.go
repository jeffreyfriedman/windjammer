package main

import (
	"fmt"
	wj "github.com/windjammer/sdk-go/windjammer"
)

// Hello World Example
//
// The simplest possible Windjammer application in Go.
//
// Build and run:
//   go run examples/hello_world.go

func main() {
	fmt.Println("=== Windjammer Hello World (Go) ===")
	fmt.Println("SDK Version: 0.1.0\n")

	// Create a new application
	app := wj.NewApp()

	// Add a simple system
	app.AddSystem(func() {
		fmt.Println("Hello from the game loop!")
	})

	fmt.Println("Application created successfully!")
	fmt.Println("Systems registered: 1\n")
	fmt.Println("Note: Full app.Run() would start the game loop")
	fmt.Println("For this example, we're just demonstrating SDK setup\n")

	// Run the application
	app.Run()
}

