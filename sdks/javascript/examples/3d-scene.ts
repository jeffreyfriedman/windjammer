/**
 * 3D Scene Example
 * 
 * Demonstrates 3D rendering with PBR materials, lighting, and post-processing.
 * 
 * Run with: npm run build && node dist/examples/3d-scene.js
 */

import { 
  App, Vec3, Camera3D, PointLight, Mesh, Material, Color,
  PostProcessing, BloomSettings, SSAOSettings, ToneMappingMode, ColorGrading
} from '../src/index';

const app = new App();

app.addStartupSystem(() => {
  // Camera
  new Camera3D({
    position: new Vec3(0, 5, 10),
    lookAt: new Vec3(0, 0, 0),
    fov: 60.0
  });
  
  // Lights
  new PointLight({ position: new Vec3(5, 5, 5), color: new Color(1.0, 0.8, 0.6, 1.0), intensity: 2000.0 });
  new PointLight({ position: new Vec3(-5, 5, 5), color: new Color(0.6, 0.8, 1.0, 1.0), intensity: 1500.0 });
  new PointLight({ position: new Vec3(0, 10, -5), color: new Color(1, 1, 1, 1), intensity: 1000.0 });
  
  // Meshes with PBR materials
  Mesh.cube(1.0).withMaterial(new Material({
    albedo: new Color(0.8, 0.2, 0.2, 1.0),
    metallic: 0.8,
    roughness: 0.2,
    emissive: new Color(0.5, 0.1, 0.1, 1.0)
  }));
  
  Mesh.sphere(1.0, 32).withMaterial(new Material({
    albedo: new Color(0.2, 0.2, 0.8, 1.0),
    metallic: 0.5,
    roughness: 0.5,
    emissive: new Color(0.1, 0.1, 0.5, 1.0)
  }));
  
  Mesh.plane(10.0).withMaterial(new Material({
    albedo: new Color(0.3, 0.3, 0.3, 1.0),
    metallic: 0.0,
    roughness: 0.9
  }));
  
  // Post-processing
  const post = new PostProcessing();
  post.enableHDR(true);
  post.setBloom({ threshold: 1.0, intensity: 0.8, radius: 4.0, softKnee: 0.5 });
  post.setSSAO({ radius: 0.5, intensity: 1.5, bias: 0.025, samples: 16 });
  post.setToneMapping(ToneMappingMode.ACES, 1.2);
  post.setColorGrading({ temperature: 0.1, tint: 0.0, saturation: 1.2, contrast: 1.1 });
});

app.addSystem(() => {
  // Rotate objects for dynamic lighting
});

app.run();

