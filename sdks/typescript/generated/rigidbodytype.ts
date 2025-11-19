/** Rigid body type for physics */
export enum RigidBodyType {
  /** Dynamic body (affected by forces) */
  Dynamic = 0,
  /** Static body (never moves) */
  Static = 1,
  /** Kinematic body (moved by code) */
  Kinematic = 2,
}
