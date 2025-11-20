#!/usr/bin/env swift
/**
 * 3D Scene Demo
 * 
 * Demonstrates 3D rendering with PBR materials, lighting, and post-processing.
 * 
 * Run with: swift examples/3DScene.swift
 */

import Foundation

let app = App()

app.addStartupSystem {
    // Camera
    Camera3D(
        position: Vec3(x: 0, y: 5, z: 10),
        lookAt: Vec3(x: 0, y: 0, z: 0),
        fov: 60.0
    )
    
    // Lights
    PointLight(position: Vec3(x: 5, y: 5, z: 5), color: Color(r: 1.0, g: 0.8, b: 0.6, a: 1.0), intensity: 2000.0)
    PointLight(position: Vec3(x: -5, y: 5, z: 5), color: Color(r: 0.6, g: 0.8, b: 1.0, a: 1.0), intensity: 1500.0)
    PointLight(position: Vec3(x: 0, y: 10, z: -5), color: Color(r: 1, g: 1, b: 1, a: 1), intensity: 1000.0)
    
    // Meshes with PBR materials
    Mesh.cube(size: 1.0).withMaterial(Material(
        albedo: Color(r: 0.8, g: 0.2, b: 0.2, a: 1.0),
        metallic: 0.8,
        roughness: 0.2,
        emissive: Color(r: 0.5, g: 0.1, b: 0.1, a: 1.0)
    ))
    
    Mesh.sphere(radius: 1.0, subdivisions: 32).withMaterial(Material(
        albedo: Color(r: 0.2, g: 0.2, b: 0.8, a: 1.0),
        metallic: 0.5,
        roughness: 0.5,
        emissive: Color(r: 0.1, g: 0.1, b: 0.5, a: 1.0)
    ))
    
    Mesh.plane(size: 10.0).withMaterial(Material(
        albedo: Color(r: 0.3, g: 0.3, b: 0.3, a: 1.0),
        metallic: 0.0,
        roughness: 0.9
    ))
    
    // Post-processing
    let post = PostProcessing()
    post.enableHDR(true)
    post.setBloom(BloomSettings(threshold: 1.0, intensity: 0.8, radius: 4.0, softKnee: 0.5))
    post.setSSAO(SSAOSettings(radius: 0.5, intensity: 1.5, bias: 0.025, samples: 16))
    post.setToneMapping(ToneMappingMode.ACES, exposure: 1.2)
    post.setColorGrading(ColorGrading(temperature: 0.1, tint: 0.0, saturation: 1.2, contrast: 1.1))
}

app.addSystem { (time: Time) in
    // Rotate objects for dynamic lighting
}

app.run()
