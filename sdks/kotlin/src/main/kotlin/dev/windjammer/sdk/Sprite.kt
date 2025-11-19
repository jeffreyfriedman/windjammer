package dev.windjammer.sdk

/**
 * 2D orthographic camera.
 */
data class Camera2D(
    var position: Vec2,
    var zoom: Float
) {
    override fun toString() = "Camera2D(pos=$position, zoom=%.2f)".format(zoom)
}

/**
 * 2D sprite component.
 */
data class Sprite(
    var texture: String,
    var position: Vec2,
    var size: Vec2
) {
    override fun toString() = "Sprite(texture='$texture', pos=$position)"
}

// DSL builders
fun camera2D(position: Vec2, zoom: Float = 1.0f) = Camera2D(position, zoom)
fun sprite(texture: String, position: Vec2, size: Vec2) = Sprite(texture, position, size)

