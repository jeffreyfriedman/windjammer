#!/usr/bin/env swift
/**
 * 3D Scene Demo
 * 
 * Demonstrates 3D rendering with the Windjammer Swift SDK.
 * 
 * Run with: swift examples/3DScene.swift
 */

import Foundation

print("=== Windjammer 3D Scene Demo (Swift) ===")

// Create 3D application
let app = App()

// Setup system
app.addStartupSystem {
    print("\n[Setup] Creating 3D scene...")
    
    // Create 3D camera
    let camera = Camera3D(
        position: Vec3(x: 0, y: 5, z: 10),
        lookAt: Vec3(x: 0, y: 0, z: 0),
        fov: 60.0
    )
    print("  - Camera3D at (0, 5, 10) looking at (0, 0, 0)")
    
    // Create meshes
    let cube = Mesh.cube(size: 1.0)
    print("  - Cube mesh (size=1.0)")
    
    let sphere = Mesh.sphere(radius: 1.0, subdivisions: 32)
    print("  - Sphere mesh (radius=1.0, subdivisions=32)")
    
    let plane = Mesh.plane(size: 10.0)
    print("  - Plane mesh (size=10.0)")
    
    // Create materials
    let material = Material(
        albedo: Color(r: 0.8, g: 0.2, b: 0.2, a: 1.0),
        metallic: 0.5,
        roughness: 0.5
    )
    print("  - PBR Material (red, metallic=0.5, roughness=0.5)")
    
    // Create light
    let light = PointLight(
        position: Vec3(x: 5, y: 5, z: 5),
        color: Color(r: 1, g: 1, b: 1, a: 1),
        intensity: 1000.0
    )
    print("  - Point Light at (5, 5, 5) intensity=1000")
    
    print("[Setup] Scene ready!")
}

// Update system
app.addSystem { (time: Time) in
    // This would rotate meshes each frame
}

print("3D application configured!")
print("- Camera: Perspective (60° FOV)")
print("- Rendering: Deferred PBR")
print("- Lighting: Point Light")
print()

// Run the application
app.run()

