/**
 * Hello World Example
 * 
 * The simplest possible Windjammer application in TypeScript.
 * 
 * Run with: npm run build && node dist/examples/hello-world.js
 */

import { App, VERSION } from '../src/index';

console.log('=== Windjammer Hello World (TypeScript) ===');
console.log(`SDK Version: ${VERSION}`);
console.log();

// Create a new application
const app = new App();

// Add a simple system
app.addSystem(() => {
  console.log('Hello from the game loop!');
});

console.log('Application created successfully!');
console.log('Systems registered: 1');
console.log();
console.log('Note: Full app.run() would start the game loop');
console.log('For this example, we\'re just demonstrating SDK setup');
console.log();

// Run the application
app.run();

