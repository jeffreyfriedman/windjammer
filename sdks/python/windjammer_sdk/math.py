"""
Math types and utilities for Windjammer.
"""

from typing import Tuple, Union
import math


class Vec2:
    """2D vector."""
    
    def __init__(self, x: float = 0.0, y: float = 0.0):
        self.x = float(x)
        self.y = float(y)
    
    def __repr__(self) -> str:
        return f"Vec2({self.x}, {self.y})"
    
    def __add__(self, other: 'Vec2') -> 'Vec2':
        return Vec2(self.x + other.x, self.y + other.y)
    
    def __sub__(self, other: 'Vec2') -> 'Vec2':
        return Vec2(self.x - other.x, self.y - other.y)
    
    def __mul__(self, scalar: float) -> 'Vec2':
        return Vec2(self.x * scalar, self.y * scalar)
    
    def length(self) -> float:
        """Calculate the length of the vector."""
        return math.sqrt(self.x * self.x + self.y * self.y)
    
    def normalize(self) -> 'Vec2':
        """Return a normalized copy of the vector."""
        length = self.length()
        if length > 0:
            return Vec2(self.x / length, self.y / length)
        return Vec2(0, 0)
    
    def dot(self, other: 'Vec2') -> float:
        """Calculate the dot product with another vector."""
        return self.x * other.x + self.y * other.y
    
    @staticmethod
    def zero() -> 'Vec2':
        """Return a zero vector."""
        return Vec2(0, 0)
    
    @staticmethod
    def one() -> 'Vec2':
        """Return a vector with all components set to 1."""
        return Vec2(1, 1)


class Vec3:
    """3D vector."""
    
    def __init__(self, x: float = 0.0, y: float = 0.0, z: float = 0.0):
        self.x = float(x)
        self.y = float(y)
        self.z = float(z)
    
    def __repr__(self) -> str:
        return f"Vec3({self.x}, {self.y}, {self.z})"
    
    def __add__(self, other: 'Vec3') -> 'Vec3':
        return Vec3(self.x + other.x, self.y + other.y, self.z + other.z)
    
    def __sub__(self, other: 'Vec3') -> 'Vec3':
        return Vec3(self.x - other.x, self.y - other.y, self.z - other.z)
    
    def __mul__(self, scalar: float) -> 'Vec3':
        return Vec3(self.x * scalar, self.y * scalar, self.z * scalar)
    
    def length(self) -> float:
        """Calculate the length of the vector."""
        return math.sqrt(self.x * self.x + self.y * self.y + self.z * self.z)
    
    def normalize(self) -> 'Vec3':
        """Return a normalized copy of the vector."""
        length = self.length()
        if length > 0:
            return Vec3(self.x / length, self.y / length, self.z / length)
        return Vec3(0, 0, 0)
    
    def dot(self, other: 'Vec3') -> float:
        """Calculate the dot product with another vector."""
        return self.x * other.x + self.y * other.y + self.z * other.z
    
    def cross(self, other: 'Vec3') -> 'Vec3':
        """Calculate the cross product with another vector."""
        return Vec3(
            self.y * other.z - self.z * other.y,
            self.z * other.x - self.x * other.z,
            self.x * other.y - self.y * other.x
        )
    
    @staticmethod
    def zero() -> 'Vec3':
        """Return a zero vector."""
        return Vec3(0, 0, 0)
    
    @staticmethod
    def one() -> 'Vec3':
        """Return a vector with all components set to 1."""
        return Vec3(1, 1, 1)
    
    @staticmethod
    def up() -> 'Vec3':
        """Return the up vector (0, 1, 0)."""
        return Vec3(0, 1, 0)
    
    @staticmethod
    def forward() -> 'Vec3':
        """Return the forward vector (0, 0, -1)."""
        return Vec3(0, 0, -1)
    
    @staticmethod
    def right() -> 'Vec3':
        """Return the right vector (1, 0, 0)."""
        return Vec3(1, 0, 0)


class Vec4:
    """4D vector."""
    
    def __init__(self, x: float = 0.0, y: float = 0.0, z: float = 0.0, w: float = 0.0):
        self.x = float(x)
        self.y = float(y)
        self.z = float(z)
        self.w = float(w)
    
    def __repr__(self) -> str:
        return f"Vec4({self.x}, {self.y}, {self.z}, {self.w})"
    
    @staticmethod
    def zero() -> 'Vec4':
        """Return a zero vector."""
        return Vec4(0, 0, 0, 0)


class Mat4:
    """4x4 matrix."""
    
    def __init__(self):
        # TODO: Implement matrix
        pass
    
    @staticmethod
    def identity() -> 'Mat4':
        """Return an identity matrix."""
        return Mat4()


class Quat:
    """Quaternion for rotations."""
    
    def __init__(self, x: float = 0.0, y: float = 0.0, z: float = 0.0, w: float = 1.0):
        self.x = float(x)
        self.y = float(y)
        self.z = float(z)
        self.w = float(w)
    
    def __repr__(self) -> str:
        return f"Quat({self.x}, {self.y}, {self.z}, {self.w})"
    
    @staticmethod
    def identity() -> 'Quat':
        """Return an identity quaternion."""
        return Quat(0, 0, 0, 1)

