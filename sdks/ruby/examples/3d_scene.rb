#!/usr/bin/env ruby
# frozen_string_literal: true

# 3D Scene Demo
#
# Demonstrates 3D rendering with the Windjammer Ruby SDK.
#
# Run with: ruby examples/3d_scene.rb

require 'windjammer'

puts '=== Windjammer 3D Scene Demo (Ruby) ==='

# Create 3D application
app = Windjammer::App.new

# Setup system
app.add_startup_system do
  puts "\n[Setup] Creating 3D scene..."
  
  # Create 3D camera
  camera = Windjammer::Camera3D.new(
    position: Windjammer::Vec3.new(0, 5, 10),
    look_at: Windjammer::Vec3.new(0, 0, 0),
    fov: 60.0
  )
  puts '  - Camera3D at (0, 5, 10) looking at (0, 0, 0)'
  
  # Create meshes
  cube = Windjammer::Mesh.cube(size: 1.0)
  puts '  - Cube mesh (size=1.0)'
  
  sphere = Windjammer::Mesh.sphere(radius: 1.0, subdivisions: 32)
  puts '  - Sphere mesh (radius=1.0, subdivisions=32)'
  
  plane = Windjammer::Mesh.plane(size: 10.0)
  puts '  - Plane mesh (size=10.0)'
  
  # Create materials
  material = Windjammer::Material.new(
    albedo: Windjammer::Color.new(0.8, 0.2, 0.2, 1.0),
    metallic: 0.5,
    roughness: 0.5
  )
  puts '  - PBR Material (red, metallic=0.5, roughness=0.5)'
  
  # Create light
  light = Windjammer::PointLight.new(
    position: Windjammer::Vec3.new(5, 5, 5),
    color: Windjammer::Color.new(1, 1, 1, 1),
    intensity: 1000.0
  )
  puts '  - Point Light at (5, 5, 5) intensity=1000'
  
  puts '[Setup] Scene ready!'
end

# Update system
app.add_system do |time|
  # This would rotate meshes each frame
end

puts '3D application configured!'
puts '- Camera: Perspective (60° FOV)'
puts '- Rendering: Deferred PBR'
puts '- Lighting: Point Light'
puts

# Run the application
app.run

