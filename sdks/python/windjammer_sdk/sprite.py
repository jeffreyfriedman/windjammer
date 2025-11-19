"""2D sprite components."""

from .math import Vec2
from .transform import Transform


class Sprite:
    """2D sprite component."""
    
    def __init__(
        self,
        texture: str = "",
        position: Vec2 = None,
        size: Vec2 = None
    ):
        self.texture = texture
        self.position = position or Vec2.zero()
        self.size = size or Vec2(100, 100)
        self.transform = Transform()
    
    def __repr__(self) -> str:
        return f"Sprite(texture='{self.texture}', pos={self.position})"


class Camera2D:
    """2D orthographic camera."""
    
    def __init__(self, position: Vec2 = None, zoom: float = 1.0):
        self.position = position or Vec2.zero()
        self.zoom = zoom
    
    def __repr__(self) -> str:
        return f"Camera2D(pos={self.position}, zoom={self.zoom})"

