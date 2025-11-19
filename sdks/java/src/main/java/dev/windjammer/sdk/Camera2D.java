package dev.windjammer.sdk;

/**
 * 2D orthographic camera.
 */
public class Camera2D {
    private Vec2 position;
    private float zoom;

    /**
     * Creates a new 2D camera.
     *
     * @param position Camera position
     * @param zoom Zoom level
     */
    public Camera2D(Vec2 position, float zoom) {
        this.position = position;
        this.zoom = zoom;
    }

    public Vec2 getPosition() {
        return position;
    }

    public void setPosition(Vec2 position) {
        this.position = position;
    }

    public float getZoom() {
        return zoom;
    }

    public void setZoom(float zoom) {
        this.zoom = zoom;
    }

    @Override
    public String toString() {
        return String.format("Camera2D(pos=%s, zoom=%.2f)", position, zoom);
    }
}

