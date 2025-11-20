#!/usr/bin/env lua
--[[
Hello World Example

The simplest possible Windjammer application in Lua.

Run with: lua examples/hello_world.lua
--]]

local windjammer = require("windjammer")

print("=== Windjammer Hello World (Lua) ===")
print("SDK Version: " .. windjammer.VERSION)
print()

-- Create a new application
local app = windjammer.App.new()

-- Add a simple system
app:add_system(function()
    print("Hello from the game loop!")
end)

print("Application created successfully!")
print("Systems registered: 1")
print()
print("Note: Full app:run() would start the game loop")
print("For this example, we're just demonstrating SDK setup")
print()

-- Run the application
app:run()

