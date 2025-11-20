"""
Math types for Windjammer.

This module provides high-level Python wrappers for Windjammer math types.
"""

from typing import Union
from .ffi import _lib, WjVec2, WjVec3, WjVec4, WjColor, _check_error

# ============================================================================
# Vec2
# ============================================================================

class Vec2:
    """2D vector."""
    
    def __init__(self, x: float = 0.0, y: float = 0.0):
        """Create a new 2D vector."""
        if _lib is not None:
            self._handle = _lib.wj_vec2_new(x, y)
            _check_error()
        else:
            # Mock mode
            self._handle = WjVec2(x, y)
    
    @property
    def x(self) -> float:
        """Get the x component."""
        return self._handle.x
    
    @x.setter
    def x(self, value: float):
        """Set the x component."""
        self._handle.x = value
    
    @property
    def y(self) -> float:
        """Get the y component."""
        return self._handle.y
    
    @y.setter
    def y(self, value: float):
        """Set the y component."""
        self._handle.y = value
    
    def __repr__(self) -> str:
        return f"Vec2({self.x}, {self.y})"
    
    def __eq__(self, other) -> bool:
        if not isinstance(other, Vec2):
            return False
        return self.x == other.x and self.y == other.y
    
    def __add__(self, other: 'Vec2') -> 'Vec2':
        """Add two vectors."""
        return Vec2(self.x + other.x, self.y + other.y)
    
    def __sub__(self, other: 'Vec2') -> 'Vec2':
        """Subtract two vectors."""
        return Vec2(self.x - other.x, self.y - other.y)
    
    def __mul__(self, scalar: float) -> 'Vec2':
        """Multiply vector by scalar."""
        return Vec2(self.x * scalar, self.y * scalar)
    
    def __truediv__(self, scalar: float) -> 'Vec2':
        """Divide vector by scalar."""
        return Vec2(self.x / scalar, self.y / scalar)

# ============================================================================
# Vec3
# ============================================================================

class Vec3:
    """3D vector."""
    
    def __init__(self, x: float = 0.0, y: float = 0.0, z: float = 0.0):
        """Create a new 3D vector."""
        if _lib is not None:
            self._handle = _lib.wj_vec3_new(x, y, z)
            _check_error()
        else:
            # Mock mode
            self._handle = WjVec3(x, y, z)
    
    @property
    def x(self) -> float:
        """Get the x component."""
        return self._handle.x
    
    @x.setter
    def x(self, value: float):
        """Set the x component."""
        self._handle.x = value
    
    @property
    def y(self) -> float:
        """Get the y component."""
        return self._handle.y
    
    @y.setter
    def y(self, value: float):
        """Set the y component."""
        self._handle.y = value
    
    @property
    def z(self) -> float:
        """Get the z component."""
        return self._handle.z
    
    @z.setter
    def z(self, value: float):
        """Set the z component."""
        self._handle.z = value
    
    def __repr__(self) -> str:
        return f"Vec3({self.x}, {self.y}, {self.z})"
    
    def __eq__(self, other) -> bool:
        if not isinstance(other, Vec3):
            return False
        return self.x == other.x and self.y == other.y and self.z == other.z
    
    def __add__(self, other: 'Vec3') -> 'Vec3':
        """Add two vectors."""
        return Vec3(self.x + other.x, self.y + other.y, self.z + other.z)
    
    def __sub__(self, other: 'Vec3') -> 'Vec3':
        """Subtract two vectors."""
        return Vec3(self.x - other.x, self.y - other.y, self.z - other.z)
    
    def __mul__(self, scalar: float) -> 'Vec3':
        """Multiply vector by scalar."""
        return Vec3(self.x * scalar, self.y * scalar, self.z * scalar)
    
    def __truediv__(self, scalar: float) -> 'Vec3':
        """Divide vector by scalar."""
        return Vec3(self.x / scalar, self.y / scalar, self.z / scalar)
    
    @staticmethod
    def zero() -> 'Vec3':
        """Create a zero vector."""
        return Vec3(0.0, 0.0, 0.0)
    
    @staticmethod
    def one() -> 'Vec3':
        """Create a vector with all components set to 1."""
        return Vec3(1.0, 1.0, 1.0)

# ============================================================================
# Color
# ============================================================================

class Color:
    """RGBA color."""
    
    def __init__(self, r: float = 1.0, g: float = 1.0, b: float = 1.0, a: float = 1.0):
        """Create a new color."""
        if _lib is not None:
            self._handle = _lib.wj_color_new(r, g, b, a)
            _check_error()
        else:
            # Mock mode
            self._handle = WjColor(r, g, b, a)
    
    @property
    def r(self) -> float:
        """Get the red component."""
        return self._handle.r
    
    @r.setter
    def r(self, value: float):
        """Set the red component."""
        self._handle.r = value
    
    @property
    def g(self) -> float:
        """Get the green component."""
        return self._handle.g
    
    @g.setter
    def g(self, value: float):
        """Set the green component."""
        self._handle.g = value
    
    @property
    def b(self) -> float:
        """Get the blue component."""
        return self._handle.b
    
    @b.setter
    def b(self, value: float):
        """Set the blue component."""
        self._handle.b = value
    
    @property
    def a(self) -> float:
        """Get the alpha component."""
        return self._handle.a
    
    @a.setter
    def a(self, value: float):
        """Set the alpha component."""
        self._handle.a = value
    
    def __repr__(self) -> str:
        return f"Color({self.r}, {self.g}, {self.b}, {self.a})"
    
    def __eq__(self, other) -> bool:
        if not isinstance(other, Color):
            return False
        return (self.r == other.r and self.g == other.g and 
                self.b == other.b and self.a == other.a)
    
    # Predefined colors
    @staticmethod
    def white() -> 'Color':
        """Create a white color."""
        return Color(1.0, 1.0, 1.0, 1.0)
    
    @staticmethod
    def black() -> 'Color':
        """Create a black color."""
        return Color(0.0, 0.0, 0.0, 1.0)
    
    @staticmethod
    def red() -> 'Color':
        """Create a red color."""
        return Color(1.0, 0.0, 0.0, 1.0)
    
    @staticmethod
    def green() -> 'Color':
        """Create a green color."""
        return Color(0.0, 1.0, 0.0, 1.0)
    
    @staticmethod
    def blue() -> 'Color':
        """Create a blue color."""
        return Color(0.0, 0.0, 1.0, 1.0)
    
    @staticmethod
    def yellow() -> 'Color':
        """Create a yellow color."""
        return Color(1.0, 1.0, 0.0, 1.0)
    
    @staticmethod
    def magenta() -> 'Color':
        """Create a magenta color."""
        return Color(1.0, 0.0, 1.0, 1.0)
    
    @staticmethod
    def cyan() -> 'Color':
        """Create a cyan color."""
        return Color(0.0, 1.0, 1.0, 1.0)
