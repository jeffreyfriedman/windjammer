"""
Sprite and 2D rendering for Windjammer.

This module provides 2D rendering components and camera.
"""

from typing import Optional
from .math import Vec2, Color
from .ffi import _lib, WjSprite, WjCamera2D, _check_error


class Sprite:
    """2D sprite component."""
    
    def __init__(
        self,
        texture: Optional[str] = None,
        position: Optional[Vec2] = None,
        size: Optional[Vec2] = None,
        color: Optional[Color] = None
    ):
        """
        Create a new sprite.
        
        Args:
            texture: Path to texture file
            position: Position in world space
            size: Size in pixels
            color: Tint color
        """
        self.texture = texture
        self.position = position or Vec2(0, 0)
        self.size = size or Vec2(64, 64)
        self.color = color or Color.white()
        self._handle: Optional[WjSprite] = None
        # TODO: Create sprite with FFI
    
    def __repr__(self) -> str:
        return f"Sprite(texture='{self.texture}', pos={self.position}, size={self.size})"


class Camera2D:
    """2D camera for rendering."""
    
    def __init__(
        self,
        position: Optional[Vec2] = None,
        zoom: float = 1.0,
        rotation: float = 0.0
    ):
        """
        Create a new 2D camera.
        
        Args:
            position: Camera position
            zoom: Zoom level (1.0 = normal)
            rotation: Rotation in radians
        """
        self.position = position or Vec2(0, 0)
        self.zoom = zoom
        self.rotation = rotation
        self._handle: Optional[WjCamera2D] = None
        # TODO: Create camera with FFI
    
    def __repr__(self) -> str:
        return f"Camera2D(pos={self.position}, zoom={self.zoom})"
