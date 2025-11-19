package windjammer

import "math"

// Vec2 represents a 2D vector
type Vec2 struct {
	X, Y float32
}

// Add adds two vectors
func (v Vec2) Add(other Vec2) Vec2 {
	return Vec2{X: v.X + other.X, Y: v.Y + other.Y}
}

// Sub subtracts two vectors
func (v Vec2) Sub(other Vec2) Vec2 {
	return Vec2{X: v.X - other.X, Y: v.Y - other.Y}
}

// Mul multiplies by a scalar
func (v Vec2) Mul(scalar float32) Vec2 {
	return Vec2{X: v.X * scalar, Y: v.Y * scalar}
}

// Length calculates the length of the vector
func (v Vec2) Length() float32 {
	return float32(math.Sqrt(float64(v.X*v.X + v.Y*v.Y)))
}

// Normalized returns a normalized copy of the vector
func (v Vec2) Normalized() Vec2 {
	length := v.Length()
	if length > 0 {
		return Vec2{X: v.X / length, Y: v.Y / length}
	}
	return Vec2Zero()
}

// Dot calculates the dot product
func (v Vec2) Dot(other Vec2) float32 {
	return v.X*other.X + v.Y*other.Y
}

// Vec2Zero returns a zero vector (0, 0)
func Vec2Zero() Vec2 {
	return Vec2{X: 0, Y: 0}
}

// Vec2One returns a one vector (1, 1)
func Vec2One() Vec2 {
	return Vec2{X: 1, Y: 1}
}

// Vec3 represents a 3D vector
type Vec3 struct {
	X, Y, Z float32
}

// Add adds two vectors
func (v Vec3) Add(other Vec3) Vec3 {
	return Vec3{X: v.X + other.X, Y: v.Y + other.Y, Z: v.Z + other.Z}
}

// Sub subtracts two vectors
func (v Vec3) Sub(other Vec3) Vec3 {
	return Vec3{X: v.X - other.X, Y: v.Y - other.Y, Z: v.Z - other.Z}
}

// Mul multiplies by a scalar
func (v Vec3) Mul(scalar float32) Vec3 {
	return Vec3{X: v.X * scalar, Y: v.Y * scalar, Z: v.Z * scalar}
}

// Length calculates the length of the vector
func (v Vec3) Length() float32 {
	return float32(math.Sqrt(float64(v.X*v.X + v.Y*v.Y + v.Z*v.Z)))
}

// Normalized returns a normalized copy of the vector
func (v Vec3) Normalized() Vec3 {
	length := v.Length()
	if length > 0 {
		return Vec3{X: v.X / length, Y: v.Y / length, Z: v.Z / length}
	}
	return Vec3Zero()
}

// Dot calculates the dot product
func (v Vec3) Dot(other Vec3) float32 {
	return v.X*other.X + v.Y*other.Y + v.Z*other.Z
}

// Cross calculates the cross product
func (v Vec3) Cross(other Vec3) Vec3 {
	return Vec3{
		X: v.Y*other.Z - v.Z*other.Y,
		Y: v.Z*other.X - v.X*other.Z,
		Z: v.X*other.Y - v.Y*other.X,
	}
}

// Vec3Zero returns a zero vector (0, 0, 0)
func Vec3Zero() Vec3 {
	return Vec3{X: 0, Y: 0, Z: 0}
}

// Vec3One returns a one vector (1, 1, 1)
func Vec3One() Vec3 {
	return Vec3{X: 1, Y: 1, Z: 1}
}

// Vec3Up returns the up vector (0, 1, 0)
func Vec3Up() Vec3 {
	return Vec3{X: 0, Y: 1, Z: 0}
}

// Vec3Forward returns the forward vector (0, 0, -1)
func Vec3Forward() Vec3 {
	return Vec3{X: 0, Y: 0, Z: -1}
}

// Vec3Right returns the right vector (1, 0, 0)
func Vec3Right() Vec3 {
	return Vec3{X: 1, Y: 0, Z: 0}
}

