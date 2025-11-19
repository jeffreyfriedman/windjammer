package windjammer

import "fmt"

// Camera2D represents a 2D orthographic camera
type Camera2D struct {
	Position Vec2
	Zoom     float32
}

// NewCamera2D creates a new 2D camera
func NewCamera2D(position Vec2, zoom float32) *Camera2D {
	return &Camera2D{
		Position: position,
		Zoom:     zoom,
	}
}

// String returns a string representation of Camera2D
func (c *Camera2D) String() string {
	return fmt.Sprintf("Camera2D(pos=%v, zoom=%.2f)", c.Position, c.Zoom)
}

// Sprite represents a 2D sprite component
type Sprite struct {
	Texture  string
	Position Vec2
	Size     Vec2
}

// NewSprite creates a new sprite
func NewSprite(texture string, position Vec2, size Vec2) *Sprite {
	return &Sprite{
		Texture:  texture,
		Position: position,
		Size:     size,
	}
}

// String returns a string representation of Sprite
func (s *Sprite) String() string {
	return fmt.Sprintf("Sprite(texture='%s', pos=%v)", s.Texture, s.Position)
}

