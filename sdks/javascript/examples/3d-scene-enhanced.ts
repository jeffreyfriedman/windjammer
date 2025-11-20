/**
 * Enhanced 3D Scene Demo with Post-Processing
 * 
 * Demonstrates 3D rendering with advanced post-processing effects:
 * - HDR (High Dynamic Range)
 * - Bloom (glowing lights)
 * - SSAO (Screen-Space Ambient Occlusion)
 * - Tone Mapping
 * - Color Grading
 * 
 * This creates a much more visually impressive and marketable demo.
 * 
 * Run with: npm run build && node dist/examples/3d-scene-enhanced.js
 */

import { 
  App, 
  Vec3, 
  Camera3D, 
  Mesh, 
  Material, 
  Color, 
  PointLight,
  PostProcessing,
  BloomSettings,
  SSAOSettings,
  ToneMappingMode,
  ColorGrading,
  VERSION 
} from '../src/index';

console.log('=== Windjammer Enhanced 3D Scene Demo (TypeScript) ===');
console.log(`SDK Version: ${VERSION}`);
console.log('Features: HDR + Bloom + SSAO + Tone Mapping');
console.log();

// Create 3D application
const app = new App();

// Setup system
app.addStartupSystem(() => {
  console.log('\n[Setup] Creating enhanced 3D scene...');
  
  // Create 3D camera
  const camera = new Camera3D({
    position: new Vec3(0, 5, 10),
    lookAt: new Vec3(0, 0, 0),
    fov: 60.0
  });
  console.log('  - Camera3D at (0, 5, 10) looking at (0, 0, 0)');
  
  // Create meshes
  const cube = Mesh.cube(1.0);
  console.log('  - Cube mesh (size=1.0)');
  
  const sphere = Mesh.sphere(1.0, 32);
  console.log('  - Sphere mesh (radius=1.0, subdivisions=32)');
  
  const plane = Mesh.plane(10.0);
  console.log('  - Plane mesh (size=10.0)');
  
  // Create PBR materials with emissive properties for bloom
  const materialRed = new Material({
    albedo: new Color(0.8, 0.2, 0.2, 1.0),
    metallic: 0.8,
    roughness: 0.2,
    emissive: new Color(0.5, 0.1, 0.1, 1.0) // Red glow
  });
  console.log('  - PBR Material (red, metallic=0.8, roughness=0.2, emissive glow)');
  
  const materialBlue = new Material({
    albedo: new Color(0.2, 0.2, 0.8, 1.0),
    metallic: 0.5,
    roughness: 0.5,
    emissive: new Color(0.1, 0.1, 0.5, 1.0) // Blue glow
  });
  console.log('  - PBR Material (blue, metallic=0.5, roughness=0.5, emissive glow)');
  
  const materialGround = new Material({
    albedo: new Color(0.3, 0.3, 0.3, 1.0),
    metallic: 0.0,
    roughness: 0.9
  });
  console.log('  - PBR Material (ground, non-metallic)');
  
  // Create multiple lights for dramatic effect
  const light1 = new PointLight({
    position: new Vec3(5, 5, 5),
    color: new Color(1.0, 0.8, 0.6, 1.0), // Warm light
    intensity: 2000.0 // High intensity for HDR
  });
  console.log('  - Point Light 1 at (5, 5, 5) intensity=2000 (warm)');
  
  const light2 = new PointLight({
    position: new Vec3(-5, 5, 5),
    color: new Color(0.6, 0.8, 1.0, 1.0), // Cool light
    intensity: 1500.0
  });
  console.log('  - Point Light 2 at (-5, 5, 5) intensity=1500 (cool)');
  
  const light3 = new PointLight({
    position: new Vec3(0, 10, -5),
    color: new Color(1, 1, 1, 1), // White rim light
    intensity: 1000.0
  });
  console.log('  - Point Light 3 at (0, 10, -5) intensity=1000 (rim)');
  
  // Configure post-processing effects
  const postProcessing = new PostProcessing();
  
  // Enable HDR
  postProcessing.enableHDR(true);
  console.log('  - HDR enabled');
  
  // Configure Bloom (glowing lights and emissive materials)
  const bloom: BloomSettings = {
    threshold: 1.0,    // Brightness threshold
    intensity: 0.8,    // Bloom strength
    radius: 4.0,       // Bloom spread
    softKnee: 0.5      // Smooth transition
  };
  postProcessing.setBloom(bloom);
  console.log('  - Bloom configured (threshold=1.0, intensity=0.8)');
  
  // Configure SSAO (ambient occlusion for depth)
  const ssao: SSAOSettings = {
    radius: 0.5,       // Sample radius
    intensity: 1.5,    // Effect strength
    bias: 0.025,       // Depth bias
    samples: 16        // Quality (more = better but slower)
  };
  postProcessing.setSSAO(ssao);
  console.log('  - SSAO configured (radius=0.5, intensity=1.5)');
  
  // Configure Tone Mapping (HDR to LDR conversion)
  postProcessing.setToneMapping(ToneMappingMode.ACES, 1.2);
  console.log('  - Tone Mapping: ACES (exposure=1.2)');
  
  // Optional: Color Grading for cinematic look
  const colorGrading: ColorGrading = {
    temperature: 0.1,   // Slightly warm
    tint: 0.0,         // No tint
    saturation: 1.2,   // Slightly more saturated
    contrast: 1.1      // Slightly more contrast
  };
  postProcessing.setColorGrading(colorGrading);
  console.log('  - Color Grading: warm, saturated, high contrast');
  
  console.log('[Setup] Enhanced scene ready!');
});

// Update system with rotation for dynamic lighting
app.addSystem((time) => {
  // This would rotate objects to show off the lighting
  // const rotationSpeed = 0.5;
  // const angle = time.elapsed * rotationSpeed;
});

console.log('\n3D application configured with post-processing!');
console.log('- Camera: Perspective (60° FOV)');
console.log('- Rendering: Deferred PBR');
console.log('- Lighting: 3 Point Lights (warm, cool, rim)');
console.log('- Post-Processing:');
console.log('  ✨ HDR (High Dynamic Range)');
console.log('  ✨ Bloom (glowing lights)');
console.log('  ✨ SSAO (ambient occlusion)');
console.log('  ✨ ACES Tone Mapping');
console.log('  ✨ Color Grading');
console.log();
console.log('This creates a cinematic, AAA-quality visual presentation!');
console.log();

// Run the application
app.run();

