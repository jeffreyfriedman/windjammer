package dev.windjammer.sdk;

/**
 * 2D sprite component.
 */
public class Sprite {
    private String texture;
    private Vec2 position;
    private Vec2 size;

    /**
     * Creates a new sprite.
     *
     * @param texture Texture path
     * @param position Position
     * @param size Size
     */
    public Sprite(String texture, Vec2 position, Vec2 size) {
        this.texture = texture;
        this.position = position;
        this.size = size;
    }

    public String getTexture() {
        return texture;
    }

    public void setTexture(String texture) {
        this.texture = texture;
    }

    public Vec2 getPosition() {
        return position;
    }

    public void setPosition(Vec2 position) {
        this.position = position;
    }

    public Vec2 getSize() {
        return size;
    }

    public void setSize(Vec2 size) {
        this.size = size;
    }

    @Override
    public String toString() {
        return String.format("Sprite(texture='%s', pos=%s)", texture, position);
    }
}

