/**
 * Windjammer JavaScript/TypeScript SDK
 * 
 * JavaScript and TypeScript bindings for the Windjammer Game Engine.
 * 
 * @example
 * ```typescript
 * import { App, Vec3 } from 'windjammer-sdk';
 * 
 * const app = new App();
 * app.run();
 * ```
 * 
 * @packageDocumentation
 */

// Core exports
export { App } from './app';
export { Vec2, Vec3, Vec4, Mat4, Quat } from './math';
export { Transform } from './transform';
export { Time } from './time';
export { Input, KeyCode, MouseButton } from './input';

// 2D exports
export { Sprite, Camera2D } from './sprite';

// 3D exports
export { Mesh, Material, Camera3D, PointLight, DirectionalLight, SpotLight } from './mesh';

// Physics exports
export { RigidBody, Collider } from './physics';

// Audio exports
export { AudioSource, AudioListener } from './audio';

// Networking exports
export { NetworkClient, NetworkServer } from './networking';

// AI exports
export { BehaviorTree, Pathfinder } from './ai';

// Types
export type { System, StartupSystem, ShutdownSystem } from './app';
export type { SpriteOptions } from './sprite';
export type { Camera3DOptions, PointLightOptions, DirectionalLightOptions, SpotLightOptions, MaterialOptions } from './mesh';
export type { RigidBodyOptions, ColliderOptions } from './physics';
export type { AudioSourceOptions } from './audio';

// Version
export const VERSION = '0.1.0';

