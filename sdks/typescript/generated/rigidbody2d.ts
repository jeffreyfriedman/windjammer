/** 2D physics rigid body */
export class RigidBody2D {
  body_type: RigidBodyType;
  velocity: Vec2;
  mass: number;
  /** Applies a force to the body */
  apply_force(force: Vec2) {
    throw new Error('Not implemented');
  }

  /** Applies an impulse to the body */
  apply_impulse(impulse: Vec2) {
    throw new Error('Not implemented');
  }

}
