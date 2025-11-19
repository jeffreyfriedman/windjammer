namespace Windjammer.SDK;

/// <summary>
/// Transform component for position, rotation, and scale.
/// </summary>
public class Transform
{
    /// <summary>Position of the transform.</summary>
    public Vector3 Position { get; set; } = Vector3.Zero;
    
    /// <summary>Rotation of the transform.</summary>
    public Vector3 Rotation { get; set; } = Vector3.Zero;
    
    /// <summary>Scale of the transform.</summary>
    public Vector3 Scale { get; set; } = Vector3.One;

    /// <inheritdoc/>
    public override string ToString() => $"Transform(pos={Position}, rot={Rotation}, scale={Scale})";
}

