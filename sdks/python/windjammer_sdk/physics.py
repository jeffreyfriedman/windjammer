"""Physics components."""

from .math import Vec3


class RigidBody:
    """Rigid body physics component."""
    
    def __init__(self, mass: float = 1.0, is_kinematic: bool = False):
        self.mass = mass
        self.is_kinematic = is_kinematic
        self.velocity = Vec3.zero()
    
    def __repr__(self) -> str:
        return f"RigidBody(mass={self.mass}, kinematic={self.is_kinematic})"


class Collider:
    """Collider component."""
    
    def __init__(self, shape: str = "box", size: Vec3 = None):
        self.shape = shape
        self.size = size or Vec3.one()
    
    @staticmethod
    def box(size: Vec3 = None) -> 'Collider':
        """Create a box collider."""
        return Collider("box", size or Vec3.one())
    
    @staticmethod
    def sphere(radius: float = 0.5) -> 'Collider':
        """Create a sphere collider."""
        collider = Collider("sphere")
        collider.radius = radius
        return collider
    
    def __repr__(self) -> str:
        return f"Collider(shape='{self.shape}')"

