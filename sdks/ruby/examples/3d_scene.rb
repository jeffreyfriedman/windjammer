#!/usr/bin/env ruby
# frozen_string_literal: true

# 3D Scene Demo
#
# Demonstrates 3D rendering with PBR materials, lighting, and post-processing.
#
# Run with: ruby examples/3d_scene.rb

require 'windjammer'

app = Windjammer::App.new

app.add_startup_system do
  # Camera
  Windjammer::Camera3D.new(
    position: Windjammer::Vec3.new(0, 5, 10),
    look_at: Windjammer::Vec3.new(0, 0, 0),
    fov: 60.0
  )
  
  # Lights
  Windjammer::PointLight.new(position: Windjammer::Vec3.new(5, 5, 5), color: Windjammer::Color.new(1.0, 0.8, 0.6, 1.0), intensity: 2000.0)
  Windjammer::PointLight.new(position: Windjammer::Vec3.new(-5, 5, 5), color: Windjammer::Color.new(0.6, 0.8, 1.0, 1.0), intensity: 1500.0)
  Windjammer::PointLight.new(position: Windjammer::Vec3.new(0, 10, -5), color: Windjammer::Color.new(1, 1, 1, 1), intensity: 1000.0)
  
  # Meshes with PBR materials
  Windjammer::Mesh.cube(size: 1.0).with_material(Windjammer::Material.new(
    albedo: Windjammer::Color.new(0.8, 0.2, 0.2, 1.0),
    metallic: 0.8,
    roughness: 0.2,
    emissive: Windjammer::Color.new(0.5, 0.1, 0.1, 1.0)
  ))
  
  Windjammer::Mesh.sphere(radius: 1.0, subdivisions: 32).with_material(Windjammer::Material.new(
    albedo: Windjammer::Color.new(0.2, 0.2, 0.8, 1.0),
    metallic: 0.5,
    roughness: 0.5,
    emissive: Windjammer::Color.new(0.1, 0.1, 0.5, 1.0)
  ))
  
  Windjammer::Mesh.plane(size: 10.0).with_material(Windjammer::Material.new(
    albedo: Windjammer::Color.new(0.3, 0.3, 0.3, 1.0),
    metallic: 0.0,
    roughness: 0.9
  ))
  
  # Post-processing
  post = Windjammer::PostProcessing.new
  post.enable_hdr(true)
  post.set_bloom(Windjammer::BloomSettings.new(threshold: 1.0, intensity: 0.8, radius: 4.0, soft_knee: 0.5))
  post.set_ssao(Windjammer::SSAOSettings.new(radius: 0.5, intensity: 1.5, bias: 0.025, samples: 16))
  post.set_tone_mapping(Windjammer::ToneMappingMode::ACES, 1.2)
  post.set_color_grading(Windjammer::ColorGrading.new(temperature: 0.1, tint: 0.0, saturation: 1.2, contrast: 1.1))
end

app.add_system do |time|
  # Rotate objects for dynamic lighting
end

app.run
