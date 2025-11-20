#!/usr/bin/env lua
--[[
3D Scene Demo

Demonstrates 3D rendering with the Windjammer Lua SDK.

Run with: lua examples/3d_scene.lua
--]]

local windjammer = require("windjammer")

print("=== Windjammer 3D Scene Demo (Lua) ===")

-- Create 3D application
local app = windjammer.App.new()

-- Setup system
app:add_startup_system(function()
    print("\n[Setup] Creating 3D scene...")
    
    -- Create 3D camera
    local camera = windjammer.Camera3D.new({
        position = windjammer.Vec3.new(0, 5, 10),
        look_at = windjammer.Vec3.new(0, 0, 0),
        fov = 60.0
    })
    print("  - Camera3D at (0, 5, 10) looking at (0, 0, 0)")
    
    -- Create meshes
    local cube = windjammer.Mesh.cube(1.0)
    print("  - Cube mesh (size=1.0)")
    
    local sphere = windjammer.Mesh.sphere(1.0, 32)
    print("  - Sphere mesh (radius=1.0, subdivisions=32)")
    
    local plane = windjammer.Mesh.plane(10.0)
    print("  - Plane mesh (size=10.0)")
    
    -- Create materials
    local material = windjammer.Material.new({
        albedo = windjammer.Color.new(0.8, 0.2, 0.2, 1.0),
        metallic = 0.5,
        roughness = 0.5
    })
    print("  - PBR Material (red, metallic=0.5, roughness=0.5)")
    
    -- Create light
    local light = windjammer.PointLight.new({
        position = windjammer.Vec3.new(5, 5, 5),
        color = windjammer.Color.new(1, 1, 1, 1),
        intensity = 1000.0
    })
    print("  - Point Light at (5, 5, 5) intensity=1000")
    
    print("[Setup] Scene ready!")
end)

-- Update system
app:add_system(function(time)
    -- This would rotate meshes each frame
end)

print("3D application configured!")
print("- Camera: Perspective (60° FOV)")
print("- Rendering: Deferred PBR")
print("- Lighting: Point Light")
print()

-- Run the application
app:run()

