using System;

namespace Windjammer.SDK;

/// <summary>
/// 2D vector.
/// </summary>
public struct Vector2 : IEquatable<Vector2>
{
    /// <summary>X component.</summary>
    public float X;
    
    /// <summary>Y component.</summary>
    public float Y;

    /// <summary>
    /// Creates a new 2D vector.
    /// </summary>
    /// <param name="x">X component.</param>
    /// <param name="y">Y component.</param>
    public Vector2(float x, float y)
    {
        X = x;
        Y = y;
    }

    /// <summary>Adds two vectors.</summary>
    public static Vector2 operator +(Vector2 a, Vector2 b) => new(a.X + b.X, a.Y + b.Y);
    
    /// <summary>Subtracts two vectors.</summary>
    public static Vector2 operator -(Vector2 a, Vector2 b) => new(a.X - b.X, a.Y - b.Y);
    
    /// <summary>Multiplies a vector by a scalar.</summary>
    public static Vector2 operator *(Vector2 v, float s) => new(v.X * s, v.Y * s);
    
    /// <summary>Multiplies a scalar by a vector.</summary>
    public static Vector2 operator *(float s, Vector2 v) => new(v.X * s, v.Y * s);

    /// <summary>Calculates the length of the vector.</summary>
    public float Length() => MathF.Sqrt(X * X + Y * Y);

    /// <summary>Returns a normalized copy of the vector.</summary>
    public Vector2 Normalized()
    {
        var len = Length();
        return len > 0 ? new Vector2(X / len, Y / len) : Zero;
    }

    /// <summary>Calculates the dot product with another vector.</summary>
    public float Dot(Vector2 other) => X * other.X + Y * other.Y;

    /// <summary>Zero vector (0, 0).</summary>
    public static Vector2 Zero => new(0, 0);
    
    /// <summary>One vector (1, 1).</summary>
    public static Vector2 One => new(1, 1);

    /// <inheritdoc/>
    public bool Equals(Vector2 other) => X.Equals(other.X) && Y.Equals(other.Y);
    
    /// <inheritdoc/>
    public override bool Equals(object? obj) => obj is Vector2 other && Equals(other);
    
    /// <inheritdoc/>
    public override int GetHashCode() => HashCode.Combine(X, Y);
    
    /// <inheritdoc/>
    public override string ToString() => $"Vector2({X}, {Y})";
}

/// <summary>
/// 3D vector.
/// </summary>
public struct Vector3 : IEquatable<Vector3>
{
    /// <summary>X component.</summary>
    public float X;
    
    /// <summary>Y component.</summary>
    public float Y;
    
    /// <summary>Z component.</summary>
    public float Z;

    /// <summary>
    /// Creates a new 3D vector.
    /// </summary>
    /// <param name="x">X component.</param>
    /// <param name="y">Y component.</param>
    /// <param name="z">Z component.</param>
    public Vector3(float x, float y, float z)
    {
        X = x;
        Y = y;
        Z = z;
    }

    /// <summary>Adds two vectors.</summary>
    public static Vector3 operator +(Vector3 a, Vector3 b) => new(a.X + b.X, a.Y + b.Y, a.Z + b.Z);
    
    /// <summary>Subtracts two vectors.</summary>
    public static Vector3 operator -(Vector3 a, Vector3 b) => new(a.X - b.X, a.Y - b.Y, a.Z - b.Z);
    
    /// <summary>Multiplies a vector by a scalar.</summary>
    public static Vector3 operator *(Vector3 v, float s) => new(v.X * s, v.Y * s, v.Z * s);
    
    /// <summary>Multiplies a scalar by a vector.</summary>
    public static Vector3 operator *(float s, Vector3 v) => new(v.X * s, v.Y * s, v.Z * s);

    /// <summary>Calculates the length of the vector.</summary>
    public float Length() => MathF.Sqrt(X * X + Y * Y + Z * Z);

    /// <summary>Returns a normalized copy of the vector.</summary>
    public Vector3 Normalized()
    {
        var len = Length();
        return len > 0 ? new Vector3(X / len, Y / len, Z / len) : Zero;
    }

    /// <summary>Calculates the dot product with another vector.</summary>
    public float Dot(Vector3 other) => X * other.X + Y * other.Y + Z * other.Z;

    /// <summary>Calculates the cross product with another vector.</summary>
    public Vector3 Cross(Vector3 other) => new(
        Y * other.Z - Z * other.Y,
        Z * other.X - X * other.Z,
        X * other.Y - Y * other.X
    );

    /// <summary>Zero vector (0, 0, 0).</summary>
    public static Vector3 Zero => new(0, 0, 0);
    
    /// <summary>One vector (1, 1, 1).</summary>
    public static Vector3 One => new(1, 1, 1);
    
    /// <summary>Up vector (0, 1, 0).</summary>
    public static Vector3 Up => new(0, 1, 0);
    
    /// <summary>Forward vector (0, 0, -1).</summary>
    public static Vector3 Forward => new(0, 0, -1);
    
    /// <summary>Right vector (1, 0, 0).</summary>
    public static Vector3 Right => new(1, 0, 0);

    /// <inheritdoc/>
    public bool Equals(Vector3 other) => X.Equals(other.X) && Y.Equals(other.Y) && Z.Equals(other.Z);
    
    /// <inheritdoc/>
    public override bool Equals(object? obj) => obj is Vector3 other && Equals(other);
    
    /// <inheritdoc/>
    public override int GetHashCode() => HashCode.Combine(X, Y, Z);
    
    /// <inheritdoc/>
    public override string ToString() => $"Vector3({X}, {Y}, {Z})";
}

/// <summary>
/// 4D vector.
/// </summary>
public struct Vector4 : IEquatable<Vector4>
{
    /// <summary>X component.</summary>
    public float X;
    
    /// <summary>Y component.</summary>
    public float Y;
    
    /// <summary>Z component.</summary>
    public float Z;
    
    /// <summary>W component.</summary>
    public float W;

    /// <summary>
    /// Creates a new 4D vector.
    /// </summary>
    public Vector4(float x, float y, float z, float w)
    {
        X = x;
        Y = y;
        Z = z;
        W = w;
    }

    /// <summary>Zero vector (0, 0, 0, 0).</summary>
    public static Vector4 Zero => new(0, 0, 0, 0);

    /// <inheritdoc/>
    public bool Equals(Vector4 other) => X.Equals(other.X) && Y.Equals(other.Y) && Z.Equals(other.Z) && W.Equals(other.W);
    
    /// <inheritdoc/>
    public override bool Equals(object? obj) => obj is Vector4 other && Equals(other);
    
    /// <inheritdoc/>
    public override int GetHashCode() => HashCode.Combine(X, Y, Z, W);
    
    /// <inheritdoc/>
    public override string ToString() => $"Vector4({X}, {Y}, {Z}, {W})";
}

/// <summary>
/// Quaternion for rotations.
/// </summary>
public struct Quaternion : IEquatable<Quaternion>
{
    /// <summary>X component.</summary>
    public float X;
    
    /// <summary>Y component.</summary>
    public float Y;
    
    /// <summary>Z component.</summary>
    public float Z;
    
    /// <summary>W component.</summary>
    public float W;

    /// <summary>
    /// Creates a new quaternion.
    /// </summary>
    public Quaternion(float x, float y, float z, float w)
    {
        X = x;
        Y = y;
        Z = z;
        W = w;
    }

    /// <summary>Identity quaternion (0, 0, 0, 1).</summary>
    public static Quaternion Identity => new(0, 0, 0, 1);

    /// <inheritdoc/>
    public bool Equals(Quaternion other) => X.Equals(other.X) && Y.Equals(other.Y) && Z.Equals(other.Z) && W.Equals(other.W);
    
    /// <inheritdoc/>
    public override bool Equals(object? obj) => obj is Quaternion other && Equals(other);
    
    /// <inheritdoc/>
    public override int GetHashCode() => HashCode.Combine(X, Y, Z, W);
    
    /// <inheritdoc/>
    public override string ToString() => $"Quaternion({X}, {Y}, {Z}, {W})";
}

