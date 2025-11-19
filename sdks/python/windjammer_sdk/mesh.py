"""3D mesh and material components."""

from typing import Tuple
from .math import Vec3


class Mesh:
    """3D mesh component."""
    
    def __init__(self, mesh_type: str = "cube"):
        self.mesh_type = mesh_type
    
    @staticmethod
    def cube(size: float = 1.0) -> 'Mesh':
        """Create a cube mesh."""
        mesh = Mesh("cube")
        mesh.size = size
        return mesh
    
    @staticmethod
    def sphere(radius: float = 1.0, subdivisions: int = 32) -> 'Mesh':
        """Create a sphere mesh."""
        mesh = Mesh("sphere")
        mesh.radius = radius
        mesh.subdivisions = subdivisions
        return mesh
    
    @staticmethod
    def plane(size: float = 10.0) -> 'Mesh':
        """Create a plane mesh."""
        mesh = Mesh("plane")
        mesh.size = size
        return mesh
    
    def __repr__(self) -> str:
        return f"Mesh(type='{self.mesh_type}')"


class Material:
    """PBR material."""
    
    def __init__(
        self,
        albedo: Tuple[float, float, float] = (1.0, 1.0, 1.0),
        metallic: float = 0.5,
        roughness: float = 0.5
    ):
        self.albedo = albedo
        self.metallic = metallic
        self.roughness = roughness
    
    @staticmethod
    def standard() -> 'Material':
        """Create a standard PBR material."""
        return Material()
    
    def __repr__(self) -> str:
        return f"Material(albedo={self.albedo}, metallic={self.metallic}, roughness={self.roughness})"


class Camera3D:
    """3D perspective camera."""
    
    def __init__(
        self,
        position: Vec3 = None,
        look_at: Vec3 = None,
        fov: float = 60.0
    ):
        self.position = position or Vec3(0, 5, 10)
        self.look_at = look_at or Vec3.zero()
        self.fov = fov
    
    def __repr__(self) -> str:
        return f"Camera3D(pos={self.position}, look_at={self.look_at})"


class PointLight:
    """Point light source."""
    
    def __init__(self, position: Vec3 = None, intensity: float = 1000.0):
        self.position = position or Vec3.zero()
        self.intensity = intensity
    
    def __repr__(self) -> str:
        return f"PointLight(pos={self.position}, intensity={self.intensity})"


class DirectionalLight:
    """Directional light source."""
    
    def __init__(self, direction: Vec3 = None, intensity: float = 1.0):
        self.direction = direction or Vec3(0, -1, 0)
        self.intensity = intensity
    
    def __repr__(self) -> str:
        return f"DirectionalLight(dir={self.direction}, intensity={self.intensity})"


class SpotLight:
    """Spot light source."""
    
    def __init__(
        self,
        position: Vec3 = None,
        direction: Vec3 = None,
        intensity: float = 1000.0,
        angle: float = 45.0
    ):
        self.position = position or Vec3.zero()
        self.direction = direction or Vec3(0, -1, 0)
        self.intensity = intensity
        self.angle = angle
    
    def __repr__(self) -> str:
        return f"SpotLight(pos={self.position}, intensity={self.intensity})"

