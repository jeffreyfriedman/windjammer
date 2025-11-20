#!/usr/bin/env python3
"""
3D Scene Example

Demonstrates 3D rendering with PBR materials, lighting, and post-processing.

Run with: python examples/3d_scene.py
"""

from windjammer_sdk import (
    App, Vec3, Color, Camera3D, PointLight, DirectionalLight,
    Mesh, Material, PostProcessing
)

def main():
    print("=== Windjammer 3D Scene Demo (Python) ===")
    
    # Create 3D application
    app = App(title="3D Scene Demo")
    
    # Setup system
    def setup_3d_scene():
        print("\n[Setup] Creating 3D scene...")
        
        # Setup 3D camera
        camera = Camera3D(
            position=Vec3(0, 5, 10),
            look_at=Vec3(0, 0, 0),
            fov=60.0
        )
        print(f"  - {camera}")
        
        # Setup 3-point lighting
        key_light = PointLight(
            position=Vec3(5, 8, 5),
            color=Color(1.0, 0.8, 0.6, 1.0),  # Warm color
            intensity=2000.0
        )
        print(f"  - {key_light}")
        
        fill_light = PointLight(
            position=Vec3(-5, 3, 5),
            color=Color(0.6, 0.8, 1.0, 1.0),  # Cool color
            intensity=1500.0
        )
        print(f"  - {fill_light}")
        
        rim_light = DirectionalLight(
            direction=Vec3(0, -1, -1),
            color=Color(1.0, 1.0, 1.0, 1.0),  # White
            intensity=1000.0
        )
        print(f"  - {rim_light}")
        
        # Create meshes
        cube_mesh = Mesh.cube(1.0)
        sphere_mesh = Mesh.sphere(0.75, 32)
        plane_mesh = Mesh.plane(20.0)
        print(f"  - {cube_mesh}")
        print(f"  - {sphere_mesh}")
        print(f"  - {plane_mesh}")
        
        # Create PBR materials
        red_metallic = Material(
            albedo=Color(1.0, 0.0, 0.0, 1.0),
            metallic=0.8,
            roughness=0.2,
            emissive=Color(0.5, 0.0, 0.0, 1.0)  # Emissive red
        )
        print(f"  - {red_metallic}")
        
        blue_rough = Material(
            albedo=Color(0.0, 0.0, 1.0, 1.0),
            metallic=0.0,
            roughness=0.9,
            emissive=Color(0.0, 0.0, 0.5, 1.0)  # Emissive blue
        )
        print(f"  - {blue_rough}")
        
        ground = Material(
            albedo=Color(0.3, 0.3, 0.3, 1.0),
            metallic=0.1,
            roughness=0.7
        )
        print(f"  - {ground}")
        
        # Configure post-processing
        print("\n[PostProcessing] Configuring effects...")
        PostProcessing.enable_hdr(True)
        PostProcessing.enable_bloom(True)
        PostProcessing.enable_ssao(True)
        PostProcessing.set_tone_mapping("ACES")
        PostProcessing.set_color_grading(
            temperature=0.1,  # Slightly warmer
            saturation=0.2,   # Slightly more saturated
            contrast=0.1      # Slightly more contrast
        )
        print("  - HDR: Enabled")
        print("  - Bloom: Enabled")
        print("  - SSAO: Enabled")
        print("  - Tone Mapping: ACES")
        print("  - Color Grading: Applied")
        
        print("[Setup] Scene ready!")
    
    app.add_startup_system(setup_3d_scene)
    
    # Game logic system
    def update_scene():
        # Game logic would go here
        pass
    
    app.add_system(update_scene)
    
    print("\n3D application configured!")
    print("- Camera: Perspective")
    print("- Lights: 3-point lighting (key, fill, rim)")
    print("- Meshes: Cube, Sphere, Plane")
    print("- Materials: PBR with metallic/roughness")
    print("- Post-processing: HDR, Bloom, SSAO, ACES, Color Grading")
    print()
    
    # Run the application
    app.run()

if __name__ == "__main__":
    main()
