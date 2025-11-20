#!/usr/bin/env python3
"""
3D Scene Example

Demonstrates 3D rendering with PBR materials, lighting, and post-processing.

Run with: python examples/3d_scene.py
"""

from windjammer_sdk import (
    App, Vec3, Camera3D, PointLight, Mesh, Material, Color,
    PostProcessing, BloomSettings, SSAOSettings, ToneMappingMode
)

def main():
    app = App()
    
    @app.startup
    def setup():
        # Camera
        Camera3D(
            position=Vec3(0, 5, 10),
            look_at=Vec3(0, 0, 0),
            fov=60.0
        )
        
        # Lights
        PointLight(Vec3(5, 5, 5), Color(1.0, 0.8, 0.6, 1.0), 2000.0)
        PointLight(Vec3(-5, 5, 5), Color(0.6, 0.8, 1.0, 1.0), 1500.0)
        PointLight(Vec3(0, 10, -5), Color(1, 1, 1, 1), 1000.0)
        
        # Meshes with PBR materials
        Mesh.cube(1.0).with_material(Material(
            albedo=Color(0.8, 0.2, 0.2, 1.0),
            metallic=0.8,
            roughness=0.2,
            emissive=Color(0.5, 0.1, 0.1, 1.0)
        ))
        
        Mesh.sphere(1.0, 32).with_material(Material(
            albedo=Color(0.2, 0.2, 0.8, 1.0),
            metallic=0.5,
            roughness=0.5,
            emissive=Color(0.1, 0.1, 0.5, 1.0)
        ))
        
        Mesh.plane(10.0).with_material(Material(
            albedo=Color(0.3, 0.3, 0.3, 1.0),
            metallic=0.0,
            roughness=0.9
        ))
        
        # Post-processing
        post = PostProcessing()
        post.enable_hdr(True)
        post.set_bloom(BloomSettings(threshold=1.0, intensity=0.8, radius=4.0, soft_knee=0.5))
        post.set_ssao(SSAOSettings(radius=0.5, intensity=1.5, bias=0.025, samples=16))
        post.set_tone_mapping(ToneMappingMode.ACES, 1.2)
        post.set_color_grading(temperature=0.1, saturation=1.2, contrast=1.1)
    
    @app.system
    def update(time):
        # Rotate objects for dynamic lighting
        pass
    
    app.run()

if __name__ == "__main__":
    main()

