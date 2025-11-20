// 2D Sprite Demo
//
// Demonstrates 2D sprite rendering with the Windjammer Go SDK.
//
// Run with: go run examples/sprite_demo.go

package main

import (
	"fmt"
	wj "github.com/windjammer/sdk-go/windjammer"
)

func main() {
	fmt.Println("=== Windjammer 2D Sprite Demo (Go) ===")

	// Create 2D application
	app := wj.NewApp()

	// Setup system
	app.AddStartupSystem(func() {
		fmt.Println("\n[Setup] Creating 2D scene...")

		// Create camera
		camera := wj.NewCamera2D(wj.Vec2{X: 0, Y: 0}, 1.0)
		fmt.Printf("  - %v\n", camera)

		// Create sprites
		sprite1 := wj.NewSprite("player.png", wj.Vec2{X: 0, Y: 0}, wj.Vec2{X: 64, Y: 64})
		fmt.Println("  - Sprite 'player.png' at (0, 0) size=(64, 64)")

		sprite2 := wj.NewSprite("enemy.png", wj.Vec2{X: 100, Y: 100}, wj.Vec2{X: 48, Y: 48})
		fmt.Println("  - Sprite 'enemy.png' at (100, 100) size=(48, 48)")

		fmt.Println("[Setup] Scene ready!")
	})

	// Update system
	app.AddSystem(func() {
		// This would rotate sprites each frame
	})

	fmt.Println("2D application configured!")
	fmt.Println("- Camera: Orthographic")
	fmt.Println("- Sprites: Enabled")
	fmt.Println("- Physics: 2D")
	fmt.Println()

	// Run the application
	app.Run()
}

