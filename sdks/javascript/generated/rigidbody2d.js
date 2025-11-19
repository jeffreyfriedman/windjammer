/** 2D physics rigid body */
export class RigidBody2D {
  constructor(body_type) {
    this.body_type = null;
    this.velocity = null;
    this.mass = null;
  }

  /** Applies a force to the body */
  apply_force(force) {
    throw new Error('Not implemented');
  }

  /** Applies an impulse to the body */
  apply_impulse(impulse) {
    throw new Error('Not implemented');
  }

}
