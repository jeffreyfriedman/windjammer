/** 2D transformation (position, rotation, scale) */
export class Transform2D {
  constructor(position, rotation, scale) {
    this.position = null;
    this.rotation = null;
    this.scale = null;
  }

  /** Translates the transform */
  translate(offset) {
    throw new Error('Not implemented');
  }

  /** Rotates the transform */
  rotate(angle) {
    throw new Error('Not implemented');
  }

}
