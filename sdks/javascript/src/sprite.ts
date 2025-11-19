/**
 * 2D sprite components.
 */

import { Vec2 } from './math';
import { Transform } from './transform';

/**
 * Options for creating a sprite.
 */
export interface SpriteOptions {
  /** Texture path */
  texture?: string;
  /** Position */
  position?: Vec2;
  /** Size */
  size?: Vec2;
}

/**
 * 2D sprite component.
 */
export class Sprite {
  /** Texture path */
  texture: string;
  
  /** Position */
  position: Vec2;
  
  /** Size */
  size: Vec2;
  
  /** Transform */
  transform: Transform;

  /**
   * Create a new sprite.
   * 
   * @param options - Sprite options
   */
  constructor(options: SpriteOptions = {}) {
    this.texture = options.texture || '';
    this.position = options.position || Vec2.zero();
    this.size = options.size || new Vec2(100, 100);
    this.transform = new Transform();
  }

  toString(): string {
    return `Sprite(texture='${this.texture}', pos=${this.position})`;
  }
}

/**
 * 2D orthographic camera.
 */
export class Camera2D {
  /** Camera position */
  position: Vec2;
  
  /** Zoom level */
  zoom: number;

  /**
   * Create a new 2D camera.
   * 
   * @param position - Camera position
   * @param zoom - Zoom level
   */
  constructor(position: Vec2 = Vec2.zero(), zoom: number = 1.0) {
    this.position = position;
    this.zoom = zoom;
  }

  toString(): string {
    return `Camera2D(pos=${this.position}, zoom=${this.zoom})`;
  }
}

