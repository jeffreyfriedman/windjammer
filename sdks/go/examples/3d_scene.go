// 3D Scene Demo
//
// Demonstrates 3D rendering with PBR materials, lighting, and post-processing.
//
// Run with: go run examples/3d_scene.go

package main

import (
	wj "github.com/windjammer/sdk-go/windjammer"
)

func main() {
	app := wj.NewApp()

	app.AddStartupSystem(func() {
		// Camera
		wj.NewCamera3D(
			wj.Vec3{X: 0, Y: 5, Z: 10},
			wj.Vec3{X: 0, Y: 0, Z: 0},
			60.0,
		)

		// Lights
		wj.NewPointLight(wj.Vec3{X: 5, Y: 5, Z: 5}, wj.Color{R: 1.0, G: 0.8, B: 0.6, A: 1.0}, 2000.0)
		wj.NewPointLight(wj.Vec3{X: -5, Y: 5, Z: 5}, wj.Color{R: 0.6, G: 0.8, B: 1.0, A: 1.0}, 1500.0)
		wj.NewPointLight(wj.Vec3{X: 0, Y: 10, Z: -5}, wj.Color{R: 1, G: 1, B: 1, A: 1}, 1000.0)

		// Meshes with PBR materials
		wj.MeshCube(1.0).WithMaterial(wj.Material{
			Albedo:   wj.Color{R: 0.8, G: 0.2, B: 0.2, A: 1.0},
			Metallic: 0.8,
			Roughness: 0.2,
			Emissive: wj.Color{R: 0.5, G: 0.1, B: 0.1, A: 1.0},
		})

		wj.MeshSphere(1.0, 32).WithMaterial(wj.Material{
			Albedo:   wj.Color{R: 0.2, G: 0.2, B: 0.8, A: 1.0},
			Metallic: 0.5,
			Roughness: 0.5,
			Emissive: wj.Color{R: 0.1, G: 0.1, B: 0.5, A: 1.0},
		})

		wj.MeshPlane(10.0).WithMaterial(wj.Material{
			Albedo:   wj.Color{R: 0.3, G: 0.3, B: 0.3, A: 1.0},
			Metallic: 0.0,
			Roughness: 0.9,
		})

		// Post-processing
		post := wj.NewPostProcessing()
		post.EnableHDR(true)
		post.SetBloom(wj.BloomSettings{Threshold: 1.0, Intensity: 0.8, Radius: 4.0, SoftKnee: 0.5})
		post.SetSSAO(wj.SSAOSettings{Radius: 0.5, Intensity: 1.5, Bias: 0.025, Samples: 16})
		post.SetToneMapping(wj.ToneMappingModeACES, 1.2)
		post.SetColorGrading(wj.ColorGrading{Temperature: 0.1, Tint: 0.0, Saturation: 1.2, Contrast: 1.1})
	})

	app.AddSystemWithTime(func(time *wj.Time) {
		// Rotate objects for dynamic lighting
	})

	app.Run()
}
