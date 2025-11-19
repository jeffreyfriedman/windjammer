/**
 * 3D mesh and material components.
 */

import { Vec3 } from './math';

/**
 * 3D mesh component.
 */
export class Mesh {
  private meshType: string;

  private constructor(meshType: string) {
    this.meshType = meshType;
  }

  /**
   * Create a cube mesh.
   * 
   * @param size - Size of the cube
   * @returns A new cube mesh
   */
  static cube(size: number = 1.0): Mesh {
    const mesh = new Mesh('cube');
    (mesh as any).size = size;
    return mesh;
  }

  /**
   * Create a sphere mesh.
   * 
   * @param radius - Radius of the sphere
   * @param subdivisions - Number of subdivisions
   * @returns A new sphere mesh
   */
  static sphere(radius: number = 1.0, subdivisions: number = 32): Mesh {
    const mesh = new Mesh('sphere');
    (mesh as any).radius = radius;
    (mesh as any).subdivisions = subdivisions;
    return mesh;
  }

  /**
   * Create a plane mesh.
   * 
   * @param size - Size of the plane
   * @returns A new plane mesh
   */
  static plane(size: number = 10.0): Mesh {
    const mesh = new Mesh('plane');
    (mesh as any).size = size;
    return mesh;
  }

  toString(): string {
    return `Mesh(type='${this.meshType}')`;
  }
}

/**
 * Options for creating a material.
 */
export interface MaterialOptions {
  /** Albedo color */
  albedo?: [number, number, number];
  /** Metallic value */
  metallic?: number;
  /** Roughness value */
  roughness?: number;
}

/**
 * PBR material.
 */
export class Material {
  /** Albedo color */
  albedo: [number, number, number];
  
  /** Metallic value */
  metallic: number;
  
  /** Roughness value */
  roughness: number;

  /**
   * Create a new material.
   * 
   * @param options - Material options
   */
  constructor(options: MaterialOptions = {}) {
    this.albedo = options.albedo || [1.0, 1.0, 1.0];
    this.metallic = options.metallic ?? 0.5;
    this.roughness = options.roughness ?? 0.5;
  }

  /**
   * Create a standard PBR material.
   * 
   * @param options - Material options
   * @returns A new standard material
   */
  static standard(options: MaterialOptions = {}): Material {
    return new Material(options);
  }

  toString(): string {
    return `Material(albedo=${this.albedo}, metallic=${this.metallic}, roughness=${this.roughness})`;
  }
}

/**
 * Options for creating a 3D camera.
 */
export interface Camera3DOptions {
  /** Camera position */
  position?: Vec3;
  /** Look at target */
  lookAt?: Vec3;
  /** Field of view */
  fov?: number;
}

/**
 * 3D perspective camera.
 */
export class Camera3D {
  /** Camera position */
  position: Vec3;
  
  /** Look at target */
  lookAt: Vec3;
  
  /** Field of view */
  fov: number;

  /**
   * Create a new 3D camera.
   * 
   * @param options - Camera options
   */
  constructor(options: Camera3DOptions = {}) {
    this.position = options.position || new Vec3(0, 5, 10);
    this.lookAt = options.lookAt || Vec3.zero();
    this.fov = options.fov ?? 60.0;
  }

  toString(): string {
    return `Camera3D(pos=${this.position}, lookAt=${this.lookAt})`;
  }
}

/**
 * Options for creating a point light.
 */
export interface PointLightOptions {
  /** Light position */
  position?: Vec3;
  /** Light intensity */
  intensity?: number;
}

/**
 * Point light source.
 */
export class PointLight {
  /** Light position */
  position: Vec3;
  
  /** Light intensity */
  intensity: number;

  /**
   * Create a new point light.
   * 
   * @param options - Light options
   */
  constructor(options: PointLightOptions = {}) {
    this.position = options.position || Vec3.zero();
    this.intensity = options.intensity ?? 1000.0;
  }

  toString(): string {
    return `PointLight(pos=${this.position}, intensity=${this.intensity})`;
  }
}

/**
 * Options for creating a directional light.
 */
export interface DirectionalLightOptions {
  /** Light direction */
  direction?: Vec3;
  /** Light intensity */
  intensity?: number;
}

/**
 * Directional light source.
 */
export class DirectionalLight {
  /** Light direction */
  direction: Vec3;
  
  /** Light intensity */
  intensity: number;

  /**
   * Create a new directional light.
   * 
   * @param options - Light options
   */
  constructor(options: DirectionalLightOptions = {}) {
    this.direction = options.direction || new Vec3(0, -1, 0);
    this.intensity = options.intensity ?? 1.0;
  }

  toString(): string {
    return `DirectionalLight(dir=${this.direction}, intensity=${this.intensity})`;
  }
}

/**
 * Options for creating a spot light.
 */
export interface SpotLightOptions {
  /** Light position */
  position?: Vec3;
  /** Light direction */
  direction?: Vec3;
  /** Light intensity */
  intensity?: number;
  /** Cone angle */
  angle?: number;
}

/**
 * Spot light source.
 */
export class SpotLight {
  /** Light position */
  position: Vec3;
  
  /** Light direction */
  direction: Vec3;
  
  /** Light intensity */
  intensity: number;
  
  /** Cone angle */
  angle: number;

  /**
   * Create a new spot light.
   * 
   * @param options - Light options
   */
  constructor(options: SpotLightOptions = {}) {
    this.position = options.position || Vec3.zero();
    this.direction = options.direction || new Vec3(0, -1, 0);
    this.intensity = options.intensity ?? 1000.0;
    this.angle = options.angle ?? 45.0;
  }

  toString(): string {
    return `SpotLight(pos=${this.position}, intensity=${this.intensity})`;
  }
}

