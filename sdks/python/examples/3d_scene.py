#!/usr/bin/env python3
"""
3D Scene Example

Demonstrates 3D rendering with PBR materials and lighting.

Run with: python examples/3d_scene.py
"""

from windjammer_sdk import App, Vec3, Camera3D, PointLight, Mesh, Material

def main():
    print("=== Windjammer 3D Scene Demo (Python) ===")
    
    # Create 3D application
    app = App()
    
    # Setup system
    @app.startup
    def setup_3d_scene():
        print("\n[Setup] Creating 3D scene...")
        
        # Create camera
        camera = Camera3D(
            position=Vec3(0, 5, 10),
            look_at=Vec3(0, 0, 0),
            fov=60.0
        )
        print(f"  - {camera}")
        
        # Create lights
        light = PointLight(
            position=Vec3(4, 8, 4),
            intensity=1500.0
        )
        print(f"  - {light}")
        
        # Create 3D objects
        cube = Mesh.cube(size=1.0)
        material = Material.standard()
        print(f"  - {cube}")
        print(f"  - {material}")
        
        sphere = Mesh.sphere(radius=0.5, subdivisions=32)
        print(f"  - {sphere}")
        
        plane = Mesh.plane(size=10.0)
        print(f"  - {plane}")
        
        print("[Setup] 3D scene ready!")
    
    # Update system
    @app.system
    def rotate_objects():
        # This would rotate 3D objects each frame
        pass
    
    print("3D application configured!")
    print("- Camera: Perspective")
    print("- Rendering: Deferred + PBR")
    print("- Physics: 3D (Rapier3D)")
    print("- Lighting: Point, Directional, Spot")
    print()
    
    # Run the application
    app.run()

if __name__ == "__main__":
    main()

