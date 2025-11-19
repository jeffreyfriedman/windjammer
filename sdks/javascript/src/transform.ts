/**
 * Transform component for position, rotation, and scale.
 */

import { Vec3 } from './math';

/**
 * Transform component for position, rotation, and scale.
 */
export class Transform {
  /** Position of the transform */
  position: Vec3;
  
  /** Rotation of the transform */
  rotation: Vec3;
  
  /** Scale of the transform */
  scale: Vec3;

  /**
   * Create a new transform.
   * 
   * @param position - Initial position
   * @param rotation - Initial rotation
   * @param scale - Initial scale
   */
  constructor(
    position: Vec3 = Vec3.zero(),
    rotation: Vec3 = Vec3.zero(),
    scale: Vec3 = Vec3.one()
  ) {
    this.position = position;
    this.rotation = rotation;
    this.scale = scale;
  }

  toString(): string {
    return `Transform(pos=${this.position}, rot=${this.rotation}, scale=${this.scale})`;
  }
}

