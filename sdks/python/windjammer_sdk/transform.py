"""Transform component for position, rotation, and scale."""

from .math import Vec3, Quat


class Transform:
    """Transform component for position, rotation, and scale."""
    
    def __init__(
        self,
        position: Vec3 = None,
        rotation: Vec3 = None,
        scale: Vec3 = None
    ):
        self.position = position or Vec3.zero()
        self.rotation = rotation or Vec3.zero()
        self.scale = scale or Vec3.one()
    
    def __repr__(self) -> str:
        return f"Transform(pos={self.position}, rot={self.rotation}, scale={self.scale})"

