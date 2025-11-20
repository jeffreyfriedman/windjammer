#!/usr/bin/env lua
--[[
3D Scene Demo

Demonstrates 3D rendering with PBR materials, lighting, and post-processing.

Run with: lua examples/3d_scene.lua
--]]

local wj = require("windjammer")

local app = wj.App.new()

app:add_startup_system(function()
    -- Camera
    wj.Camera3D.new({
        position = wj.Vec3.new(0, 5, 10),
        look_at = wj.Vec3.new(0, 0, 0),
        fov = 60.0
    })
    
    -- Lights
    wj.PointLight.new({ position = wj.Vec3.new(5, 5, 5), color = wj.Color.new(1.0, 0.8, 0.6, 1.0), intensity = 2000.0 })
    wj.PointLight.new({ position = wj.Vec3.new(-5, 5, 5), color = wj.Color.new(0.6, 0.8, 1.0, 1.0), intensity = 1500.0 })
    wj.PointLight.new({ position = wj.Vec3.new(0, 10, -5), color = wj.Color.new(1, 1, 1, 1), intensity = 1000.0 })
    
    -- Meshes with PBR materials
    wj.Mesh.cube(1.0):with_material(wj.Material.new({
        albedo = wj.Color.new(0.8, 0.2, 0.2, 1.0),
        metallic = 0.8,
        roughness = 0.2,
        emissive = wj.Color.new(0.5, 0.1, 0.1, 1.0)
    }))
    
    wj.Mesh.sphere(1.0, 32):with_material(wj.Material.new({
        albedo = wj.Color.new(0.2, 0.2, 0.8, 1.0),
        metallic = 0.5,
        roughness = 0.5,
        emissive = wj.Color.new(0.1, 0.1, 0.5, 1.0)
    }))
    
    wj.Mesh.plane(10.0):with_material(wj.Material.new({
        albedo = wj.Color.new(0.3, 0.3, 0.3, 1.0),
        metallic = 0.0,
        roughness = 0.9
    }))
    
    -- Post-processing
    local post = wj.PostProcessing.new()
    post:enable_hdr(true)
    post:set_bloom({ threshold = 1.0, intensity = 0.8, radius = 4.0, soft_knee = 0.5 })
    post:set_ssao({ radius = 0.5, intensity = 1.5, bias = 0.025, samples = 16 })
    post:set_tone_mapping(wj.ToneMappingMode.ACES, 1.2)
    post:set_color_grading({ temperature = 0.1, tint = 0.0, saturation = 1.2, contrast = 1.1 })
end)

app:add_system(function(time)
    -- Rotate objects for dynamic lighting
end)

app:run()
