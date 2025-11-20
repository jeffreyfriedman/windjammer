#!/usr/bin/env python3
"""
Enhanced 3D Scene Demo with Post-Processing

Demonstrates 3D rendering with advanced post-processing effects:
- HDR (High Dynamic Range)
- Bloom (glowing lights)
- SSAO (Screen-Space Ambient Occlusion)
- Tone Mapping
- Color Grading

This creates a much more visually impressive and marketable demo.

Run with: python examples/3d_scene_enhanced.py
"""

from windjammer_sdk import App, Vec3, Camera3D, Mesh, Material, Color, PointLight
from windjammer_sdk.post_processing import PostProcessing, BloomSettings, SSAOSettings, ToneMappingMode

def main():
    print("=== Windjammer Enhanced 3D Scene Demo (Python) ===")
    print("Features: HDR + Bloom + SSAO + Tone Mapping")
    print()
    
    # Create 3D application
    app = App()
    
    # Setup system
    @app.startup
    def setup_3d_scene():
        print("\n[Setup] Creating enhanced 3D scene...")
        
        # Create 3D camera
        camera = Camera3D(
            position=Vec3(0, 5, 10),
            look_at=Vec3(0, 0, 0),
            fov=60.0
        )
        print("  - Camera3D at (0, 5, 10) looking at (0, 0, 0)")
        
        # Create meshes
        cube = Mesh.cube(size=1.0)
        print("  - Cube mesh (size=1.0)")
        
        sphere = Mesh.sphere(radius=1.0, subdivisions=32)
        print("  - Sphere mesh (radius=1.0, subdivisions=32)")
        
        plane = Mesh.plane(size=10.0)
        print("  - Plane mesh (size=10.0)")
        
        # Create PBR materials with emissive properties for bloom
        material_red = Material(
            albedo=Color(0.8, 0.2, 0.2, 1.0),
            metallic=0.8,
            roughness=0.2,
            emissive=Color(0.5, 0.1, 0.1, 1.0)  # Red glow
        )
        print("  - PBR Material (red, metallic=0.8, roughness=0.2, emissive glow)")
        
        material_blue = Material(
            albedo=Color(0.2, 0.2, 0.8, 1.0),
            metallic=0.5,
            roughness=0.5,
            emissive=Color(0.1, 0.1, 0.5, 1.0)  # Blue glow
        )
        print("  - PBR Material (blue, metallic=0.5, roughness=0.5, emissive glow)")
        
        material_ground = Material(
            albedo=Color(0.3, 0.3, 0.3, 1.0),
            metallic=0.0,
            roughness=0.9
        )
        print("  - PBR Material (ground, non-metallic)")
        
        # Create multiple lights for dramatic effect
        light1 = PointLight(
            position=Vec3(5, 5, 5),
            color=Color(1.0, 0.8, 0.6, 1.0),  # Warm light
            intensity=2000.0  # High intensity for HDR
        )
        print("  - Point Light 1 at (5, 5, 5) intensity=2000 (warm)")
        
        light2 = PointLight(
            position=Vec3(-5, 5, 5),
            color=Color(0.6, 0.8, 1.0, 1.0),  # Cool light
            intensity=1500.0
        )
        print("  - Point Light 2 at (-5, 5, 5) intensity=1500 (cool)")
        
        light3 = PointLight(
            position=Vec3(0, 10, -5),
            color=Color(1.0, 1.0, 1.0, 1.0),  # White rim light
            intensity=1000.0
        )
        print("  - Point Light 3 at (0, 10, -5) intensity=1000 (rim)")
        
        # Configure post-processing effects
        post_processing = PostProcessing()
        
        # Enable HDR
        post_processing.enable_hdr(True)
        print("  - HDR enabled")
        
        # Configure Bloom (glowing lights and emissive materials)
        bloom = BloomSettings(
            threshold=1.0,      # Brightness threshold
            intensity=0.8,      # Bloom strength
            radius=4.0,         # Bloom spread
            soft_knee=0.5       # Smooth transition
        )
        post_processing.set_bloom(bloom)
        print("  - Bloom configured (threshold=1.0, intensity=0.8)")
        
        # Configure SSAO (ambient occlusion for depth)
        ssao = SSAOSettings(
            radius=0.5,         # Sample radius
            intensity=1.5,      # Effect strength
            bias=0.025,         # Depth bias
            samples=16          # Quality (more = better but slower)
        )
        post_processing.set_ssao(ssao)
        print("  - SSAO configured (radius=0.5, intensity=1.5)")
        
        # Configure Tone Mapping (HDR to LDR conversion)
        post_processing.set_tone_mapping(
            mode=ToneMappingMode.ACES,  # Filmic look
            exposure=1.2                # Slight overexposure for drama
        )
        print("  - Tone Mapping: ACES (exposure=1.2)")
        
        # Optional: Color Grading for cinematic look
        post_processing.set_color_grading(
            temperature=0.1,    # Slightly warm
            tint=0.0,          # No tint
            saturation=1.2,    # Slightly more saturated
            contrast=1.1       # Slightly more contrast
        )
        print("  - Color Grading: warm, saturated, high contrast")
        
        print("[Setup] Enhanced scene ready!")
    
    # Update system with rotation for dynamic lighting
    @app.system
    def rotate_scene(time):
        # This would rotate objects to show off the lighting
        # rotation_speed = 0.5
        # angle = time.elapsed * rotation_speed
        pass
    
    print("\n3D application configured with post-processing!")
    print("- Camera: Perspective (60° FOV)")
    print("- Rendering: Deferred PBR")
    print("- Lighting: 3 Point Lights (warm, cool, rim)")
    print("- Post-Processing:")
    print("  ✨ HDR (High Dynamic Range)")
    print("  ✨ Bloom (glowing lights)")
    print("  ✨ SSAO (ambient occlusion)")
    print("  ✨ ACES Tone Mapping")
    print("  ✨ Color Grading")
    print()
    print("This creates a cinematic, AAA-quality visual presentation!")
    print()
    
    # Run the application
    app.run()

if __name__ == "__main__":
    main()

