namespace Windjammer.SDK;

/// <summary>
/// Time information for the current frame.
/// </summary>
public class Time
{
    /// <summary>Time since last frame in seconds.</summary>
    public float DeltaSeconds { get; set; } = 0.016f; // ~60 FPS
    
    /// <summary>Total time since application start in seconds.</summary>
    public float TotalSeconds { get; set; } = 0.0f;
    
    /// <summary>Current frame number.</summary>
    public int FrameCount { get; set; } = 0;

    /// <inheritdoc/>
    public override string ToString() => $"Time(delta={DeltaSeconds}, total={TotalSeconds})";
}

