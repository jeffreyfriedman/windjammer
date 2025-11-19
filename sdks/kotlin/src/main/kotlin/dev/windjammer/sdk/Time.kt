package dev.windjammer.sdk

/**
 * Time information for the current frame.
 */
data class Time(
    val deltaSeconds: Float = 0.016f, // ~60 FPS
    val totalSeconds: Float = 0.0f,
    val frameCount: Int = 0
) {
    override fun toString() = "Time(delta=%.3f, total=%.3f)".format(deltaSeconds, totalSeconds)
}

