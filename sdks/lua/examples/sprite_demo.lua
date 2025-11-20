#!/usr/bin/env lua
--[[
2D Sprite Demo

Demonstrates 2D sprite rendering with the Windjammer Lua SDK.

Run with: lua examples/sprite_demo.lua
--]]

local windjammer = require("windjammer")

print("=== Windjammer 2D Sprite Demo (Lua) ===")

-- Create 2D application
local app = windjammer.App.new()

-- Setup system
app:add_startup_system(function()
    print("\n[Setup] Creating 2D scene...")
    
    -- Create camera
    local camera = windjammer.Camera2D.new(
        windjammer.Vec2.new(0, 0),
        1.0
    )
    print("  - Camera2D at (0, 0) zoom=1.0")
    
    -- Create sprites
    local sprite1 = windjammer.Sprite.new({
        texture = "player.png",
        position = windjammer.Vec2.new(0, 0),
        size = windjammer.Vec2.new(64, 64)
    })
    print("  - Sprite 'player.png' at (0, 0) size=(64, 64)")
    
    local sprite2 = windjammer.Sprite.new({
        texture = "enemy.png",
        position = windjammer.Vec2.new(100, 100),
        size = windjammer.Vec2.new(48, 48)
    })
    print("  - Sprite 'enemy.png' at (100, 100) size=(48, 48)")
    
    print("[Setup] Scene ready!")
end)

-- Update system
app:add_system(function()
    -- This would rotate sprites each frame
end)

print("2D application configured!")
print("- Camera: Orthographic")
print("- Sprites: Enabled")
print("- Physics: 2D")
print()

-- Run the application
app:run()

