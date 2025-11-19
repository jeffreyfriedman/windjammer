/**
 * 2D Sprite Demo
 * 
 * Demonstrates 2D sprite rendering with the Windjammer TypeScript SDK.
 * 
 * Run with: npm run build && node dist/examples/sprite-demo.js
 */

import { App, Vec2, Sprite, Camera2D } from '../src/index';

console.log('=== Windjammer 2D Sprite Demo (TypeScript) ===');

// Create 2D application
const app = new App();

// Setup system
app.addStartupSystem(() => {
  console.log('\n[Setup] Creating 2D scene...');
  
  // Create camera
  const camera = new Camera2D(new Vec2(0, 0), 1.0);
  console.log(`  - ${camera}`);
  
  // Create sprites
  const sprite1 = new Sprite({
    texture: 'player.png',
    position: new Vec2(0, 0),
    size: new Vec2(64, 64)
  });
  console.log(`  - ${sprite1}`);
  
  const sprite2 = new Sprite({
    texture: 'enemy.png',
    position: new Vec2(100, 100),
    size: new Vec2(48, 48)
  });
  console.log(`  - ${sprite2}`);
  
  console.log('[Setup] Scene ready!');
});

// Update system
app.addSystem(() => {
  // This would rotate sprites each frame
});

console.log('2D application configured!');
console.log('- Camera: Orthographic');
console.log('- Sprites: Enabled');
console.log('- Physics: 2D');
console.log();

// Run the application
app.run();

