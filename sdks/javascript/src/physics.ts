/**
 * Physics components.
 */

import { Vec3 } from './math';

/**
 * Options for creating a rigid body.
 */
export interface RigidBodyOptions {
  /** Mass of the body */
  mass?: number;
  /** Whether the body is kinematic */
  isKinematic?: boolean;
}

/**
 * Rigid body physics component.
 */
export class RigidBody {
  /** Mass of the body */
  mass: number;
  
  /** Whether the body is kinematic */
  isKinematic: boolean;
  
  /** Velocity of the body */
  velocity: Vec3;

  /**
   * Create a new rigid body.
   * 
   * @param options - Rigid body options
   */
  constructor(options: RigidBodyOptions = {}) {
    this.mass = options.mass ?? 1.0;
    this.isKinematic = options.isKinematic ?? false;
    this.velocity = Vec3.zero();
  }

  toString(): string {
    return `RigidBody(mass=${this.mass}, kinematic=${this.isKinematic})`;
  }
}

/**
 * Options for creating a collider.
 */
export interface ColliderOptions {
  /** Shape of the collider */
  shape?: string;
  /** Size of the collider */
  size?: Vec3;
}

/**
 * Collider component.
 */
export class Collider {
  /** Shape of the collider */
  shape: string;
  
  /** Size of the collider */
  size: Vec3;

  /**
   * Create a new collider.
   * 
   * @param options - Collider options
   */
  constructor(options: ColliderOptions = {}) {
    this.shape = options.shape || 'box';
    this.size = options.size || Vec3.one();
  }

  /**
   * Create a box collider.
   * 
   * @param size - Size of the box
   * @returns A new box collider
   */
  static box(size: Vec3 = Vec3.one()): Collider {
    return new Collider({ shape: 'box', size });
  }

  /**
   * Create a sphere collider.
   * 
   * @param radius - Radius of the sphere
   * @returns A new sphere collider
   */
  static sphere(radius: number = 0.5): Collider {
    const collider = new Collider({ shape: 'sphere' });
    (collider as any).radius = radius;
    return collider;
  }

  toString(): string {
    return `Collider(shape='${this.shape}')`;
  }
}

