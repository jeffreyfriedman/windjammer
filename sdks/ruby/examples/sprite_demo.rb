#!/usr/bin/env ruby
# frozen_string_literal: true

# 2D Sprite Demo
#
# Demonstrates 2D sprite rendering with the Windjammer Ruby SDK.
#
# Run with: ruby examples/sprite_demo.rb

require 'windjammer'

puts '=== Windjammer 2D Sprite Demo (Ruby) ==='

# Create 2D application
app = Windjammer::App.new

# Setup system
app.add_startup_system do
  puts "\n[Setup] Creating 2D scene..."
  
  # Create camera
  camera = Windjammer::Camera2D.new(
    position: Windjammer::Vec2.new(0, 0),
    zoom: 1.0
  )
  puts "  - #{camera}"
  
  # Create sprites
  sprite1 = Windjammer::Sprite.new(
    texture: 'player.png',
    position: Windjammer::Vec2.new(0, 0),
    size: Windjammer::Vec2.new(64, 64)
  )
  puts "  - Sprite 'player.png' at (0, 0) size=(64, 64)"
  
  sprite2 = Windjammer::Sprite.new(
    texture: 'enemy.png',
    position: Windjammer::Vec2.new(100, 100),
    size: Windjammer::Vec2.new(48, 48)
  )
  puts "  - Sprite 'enemy.png' at (100, 100) size=(48, 48)"
  
  puts '[Setup] Scene ready!'
end

# Update system
app.add_system do
  # This would rotate sprites each frame
end

puts '2D application configured!'
puts '- Camera: Orthographic'
puts '- Sprites: Enabled'
puts '- Physics: 2D'
puts

# Run the application
app.run

