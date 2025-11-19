/**
 * 3D Scene Example
 * 
 * Demonstrates 3D rendering with PBR materials and lighting.
 * 
 * Run with: npm run build && node dist/examples/3d-scene.js
 */

import { App, Vec3, Camera3D, PointLight, Mesh, Material } from '../src/index';

console.log('=== Windjammer 3D Scene Demo (TypeScript) ===');

// Create 3D application
const app = new App();

// Setup system
app.addStartupSystem(() => {
  console.log('\n[Setup] Creating 3D scene...');
  
  // Create camera
  const camera = new Camera3D({
    position: new Vec3(0, 5, 10),
    lookAt: new Vec3(0, 0, 0),
    fov: 60
  });
  console.log(`  - ${camera}`);
  
  // Create lights
  const light = new PointLight({
    position: new Vec3(4, 8, 4),
    intensity: 1500
  });
  console.log(`  - ${light}`);
  
  // Create 3D objects
  const cube = Mesh.cube(1.0);
  const material = Material.standard();
  console.log(`  - ${cube}`);
  console.log(`  - ${material}`);
  
  const sphere = Mesh.sphere(0.5, 32);
  console.log(`  - ${sphere}`);
  
  const plane = Mesh.plane(10.0);
  console.log(`  - ${plane}`);
  
  console.log('[Setup] 3D scene ready!');
});

// Update system
app.addSystem(() => {
  // This would rotate 3D objects each frame
});

console.log('3D application configured!');
console.log('- Camera: Perspective');
console.log('- Rendering: Deferred + PBR');
console.log('- Physics: 3D (Rapier3D)');
console.log('- Lighting: Point, Directional, Spot');
console.log();

// Run the application
app.run();

