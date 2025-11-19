/**
 * Math types and utilities for Windjammer.
 */

/**
 * 2D vector.
 */
export class Vec2 {
  /**
   * Create a new 2D vector.
   * 
   * @param x - X component
   * @param y - Y component
   */
  constructor(public x: number = 0, public y: number = 0) {}

  /**
   * Add another vector to this one.
   * 
   * @param other - The vector to add
   * @returns A new vector with the result
   */
  add(other: Vec2): Vec2 {
    return new Vec2(this.x + other.x, this.y + other.y);
  }

  /**
   * Subtract another vector from this one.
   * 
   * @param other - The vector to subtract
   * @returns A new vector with the result
   */
  sub(other: Vec2): Vec2 {
    return new Vec2(this.x - other.x, this.y - other.y);
  }

  /**
   * Multiply this vector by a scalar.
   * 
   * @param scalar - The scalar to multiply by
   * @returns A new vector with the result
   */
  mul(scalar: number): Vec2 {
    return new Vec2(this.x * scalar, this.y * scalar);
  }

  /**
   * Calculate the length of this vector.
   * 
   * @returns The length
   */
  length(): number {
    return Math.sqrt(this.x * this.x + this.y * this.y);
  }

  /**
   * Return a normalized copy of this vector.
   * 
   * @returns A new normalized vector
   */
  normalize(): Vec2 {
    const len = this.length();
    if (len > 0) {
      return new Vec2(this.x / len, this.y / len);
    }
    return new Vec2(0, 0);
  }

  /**
   * Calculate the dot product with another vector.
   * 
   * @param other - The other vector
   * @returns The dot product
   */
  dot(other: Vec2): number {
    return this.x * other.x + this.y * other.y;
  }

  /**
   * Create a zero vector.
   * 
   * @returns A new zero vector
   */
  static zero(): Vec2 {
    return new Vec2(0, 0);
  }

  /**
   * Create a vector with all components set to 1.
   * 
   * @returns A new vector
   */
  static one(): Vec2 {
    return new Vec2(1, 1);
  }

  toString(): string {
    return `Vec2(${this.x}, ${this.y})`;
  }
}

/**
 * 3D vector.
 */
export class Vec3 {
  /**
   * Create a new 3D vector.
   * 
   * @param x - X component
   * @param y - Y component
   * @param z - Z component
   */
  constructor(public x: number = 0, public y: number = 0, public z: number = 0) {}

  /**
   * Add another vector to this one.
   * 
   * @param other - The vector to add
   * @returns A new vector with the result
   */
  add(other: Vec3): Vec3 {
    return new Vec3(this.x + other.x, this.y + other.y, this.z + other.z);
  }

  /**
   * Subtract another vector from this one.
   * 
   * @param other - The vector to subtract
   * @returns A new vector with the result
   */
  sub(other: Vec3): Vec3 {
    return new Vec3(this.x - other.x, this.y - other.y, this.z - other.z);
  }

  /**
   * Multiply this vector by a scalar.
   * 
   * @param scalar - The scalar to multiply by
   * @returns A new vector with the result
   */
  mul(scalar: number): Vec3 {
    return new Vec3(this.x * scalar, this.y * scalar, this.z * scalar);
  }

  /**
   * Calculate the length of this vector.
   * 
   * @returns The length
   */
  length(): number {
    return Math.sqrt(this.x * this.x + this.y * this.y + this.z * this.z);
  }

  /**
   * Return a normalized copy of this vector.
   * 
   * @returns A new normalized vector
   */
  normalize(): Vec3 {
    const len = this.length();
    if (len > 0) {
      return new Vec3(this.x / len, this.y / len, this.z / len);
    }
    return new Vec3(0, 0, 0);
  }

  /**
   * Calculate the dot product with another vector.
   * 
   * @param other - The other vector
   * @returns The dot product
   */
  dot(other: Vec3): number {
    return this.x * other.x + this.y * other.y + this.z * other.z;
  }

  /**
   * Calculate the cross product with another vector.
   * 
   * @param other - The other vector
   * @returns A new vector with the result
   */
  cross(other: Vec3): Vec3 {
    return new Vec3(
      this.y * other.z - this.z * other.y,
      this.z * other.x - this.x * other.z,
      this.x * other.y - this.y * other.x
    );
  }

  /**
   * Create a zero vector.
   * 
   * @returns A new zero vector
   */
  static zero(): Vec3 {
    return new Vec3(0, 0, 0);
  }

  /**
   * Create a vector with all components set to 1.
   * 
   * @returns A new vector
   */
  static one(): Vec3 {
    return new Vec3(1, 1, 1);
  }

  /**
   * Create the up vector (0, 1, 0).
   * 
   * @returns A new up vector
   */
  static up(): Vec3 {
    return new Vec3(0, 1, 0);
  }

  /**
   * Create the forward vector (0, 0, -1).
   * 
   * @returns A new forward vector
   */
  static forward(): Vec3 {
    return new Vec3(0, 0, -1);
  }

  /**
   * Create the right vector (1, 0, 0).
   * 
   * @returns A new right vector
   */
  static right(): Vec3 {
    return new Vec3(1, 0, 0);
  }

  toString(): string {
    return `Vec3(${this.x}, ${this.y}, ${this.z})`;
  }
}

/**
 * 4D vector.
 */
export class Vec4 {
  /**
   * Create a new 4D vector.
   * 
   * @param x - X component
   * @param y - Y component
   * @param z - Z component
   * @param w - W component
   */
  constructor(
    public x: number = 0,
    public y: number = 0,
    public z: number = 0,
    public w: number = 0
  ) {}

  /**
   * Create a zero vector.
   * 
   * @returns A new zero vector
   */
  static zero(): Vec4 {
    return new Vec4(0, 0, 0, 0);
  }

  toString(): string {
    return `Vec4(${this.x}, ${this.y}, ${this.z}, ${this.w})`;
  }
}

/**
 * 4x4 matrix.
 */
export class Mat4 {
  /**
   * Create an identity matrix.
   * 
   * @returns A new identity matrix
   */
  static identity(): Mat4 {
    return new Mat4();
  }

  toString(): string {
    return 'Mat4(identity)';
  }
}

/**
 * Quaternion for rotations.
 */
export class Quat {
  /**
   * Create a new quaternion.
   * 
   * @param x - X component
   * @param y - Y component
   * @param z - Z component
   * @param w - W component
   */
  constructor(
    public x: number = 0,
    public y: number = 0,
    public z: number = 0,
    public w: number = 1
  ) {}

  /**
   * Create an identity quaternion.
   * 
   * @returns A new identity quaternion
   */
  static identity(): Quat {
    return new Quat(0, 0, 0, 1);
  }

  toString(): string {
    return `Quat(${this.x}, ${this.y}, ${this.z}, ${this.w})`;
  }
}

