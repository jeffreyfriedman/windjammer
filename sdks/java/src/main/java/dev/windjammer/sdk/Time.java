package dev.windjammer.sdk;

/**
 * Time information for the current frame.
 */
public class Time {
    private float deltaSeconds = 0.016f; // ~60 FPS
    private float totalSeconds = 0.0f;
    private int frameCount = 0;

    /**
     * Gets the time since last frame in seconds.
     */
    public float getDeltaSeconds() {
        return deltaSeconds;
    }

    /**
     * Gets the total time since application start in seconds.
     */
    public float getTotalSeconds() {
        return totalSeconds;
    }

    /**
     * Gets the current frame number.
     */
    public int getFrameCount() {
        return frameCount;
    }

    @Override
    public String toString() {
        return String.format("Time(delta=%.3f, total=%.3f)", deltaSeconds, totalSeconds);
    }
}

