// 3D Scene Demo
//
// Demonstrates 3D rendering with the Windjammer Go SDK.
//
// Run with: go run examples/3d_scene.go

package main

import (
	"fmt"
	wj "github.com/windjammer/sdk-go/windjammer"
)

func main() {
	fmt.Println("=== Windjammer 3D Scene Demo (Go) ===")

	// Create 3D application
	app := wj.NewApp()

	// Setup system
	app.AddStartupSystem(func() {
		fmt.Println("\n[Setup] Creating 3D scene...")

		// Create 3D camera
		camera := wj.NewCamera3D(
			wj.Vec3{X: 0, Y: 5, Z: 10},
			wj.Vec3{X: 0, Y: 0, Z: 0},
			60.0,
		)
		fmt.Println("  - Camera3D at (0, 5, 10) looking at (0, 0, 0)")

		// Create meshes
		cube := wj.MeshCube(1.0)
		fmt.Println("  - Cube mesh (size=1.0)")

		sphere := wj.MeshSphere(1.0, 32)
		fmt.Println("  - Sphere mesh (radius=1.0, subdivisions=32)")

		plane := wj.MeshPlane(10.0)
		fmt.Println("  - Plane mesh (size=10.0)")

		// Create materials
		material := wj.NewMaterial(
			wj.Color{R: 0.8, G: 0.2, B: 0.2, A: 1.0},
			0.5, // metallic
			0.5, // roughness
		)
		fmt.Println("  - PBR Material (red, metallic=0.5, roughness=0.5)")

		// Create light
		light := wj.NewPointLight(
			wj.Vec3{X: 5, Y: 5, Z: 5},
			wj.Color{R: 1, G: 1, B: 1, A: 1},
			1000.0,
		)
		fmt.Println("  - Point Light at (5, 5, 5) intensity=1000")

		fmt.Println("[Setup] Scene ready!")
	})

	// Update system
	app.AddSystemWithTime(func(time *wj.Time) {
		// This would rotate meshes each frame
	})

	fmt.Println("3D application configured!")
	fmt.Println("- Camera: Perspective (60° FOV)")
	fmt.Println("- Rendering: Deferred PBR")
	fmt.Println("- Lighting: Point Light")
	fmt.Println()

	// Run the application
	app.Run()
}

