/** 2D transformation (position, rotation, scale) */
export class Transform2D {
  position: Vec2;
  rotation: number;
  scale: Vec2;
  /** Translates the transform */
  translate(offset: Vec2) {
    throw new Error('Not implemented');
  }

  /** Rotates the transform */
  rotate(angle: number) {
    throw new Error('Not implemented');
  }

}
