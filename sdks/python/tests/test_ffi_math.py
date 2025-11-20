"""
Tests for FFI-based math types.
"""

import unittest
from windjammer_sdk.math import Vec2, Vec3, Color


class TestVec2(unittest.TestCase):
    """Tests for Vec2."""
    
    def test_creation(self):
        """Test Vec2 creation."""
        v = Vec2(1.0, 2.0)
        self.assertEqual(v.x, 1.0)
        self.assertEqual(v.y, 2.0)
    
    def test_default_values(self):
        """Test Vec2 default values."""
        v = Vec2()
        self.assertEqual(v.x, 0.0)
        self.assertEqual(v.y, 0.0)
    
    def test_property_setters(self):
        """Test Vec2 property setters."""
        v = Vec2(1.0, 2.0)
        v.x = 3.0
        v.y = 4.0
        self.assertEqual(v.x, 3.0)
        self.assertEqual(v.y, 4.0)
    
    def test_addition(self):
        """Test Vec2 addition."""
        v1 = Vec2(1.0, 2.0)
        v2 = Vec2(3.0, 4.0)
        v3 = v1 + v2
        self.assertEqual(v3.x, 4.0)
        self.assertEqual(v3.y, 6.0)
    
    def test_subtraction(self):
        """Test Vec2 subtraction."""
        v1 = Vec2(5.0, 6.0)
        v2 = Vec2(1.0, 2.0)
        v3 = v1 - v2
        self.assertEqual(v3.x, 4.0)
        self.assertEqual(v3.y, 4.0)
    
    def test_scalar_multiplication(self):
        """Test Vec2 scalar multiplication."""
        v1 = Vec2(2.0, 3.0)
        v2 = v1 * 2.0
        self.assertEqual(v2.x, 4.0)
        self.assertEqual(v2.y, 6.0)
    
    def test_scalar_division(self):
        """Test Vec2 scalar division."""
        v1 = Vec2(4.0, 6.0)
        v2 = v1 / 2.0
        self.assertEqual(v2.x, 2.0)
        self.assertEqual(v2.y, 3.0)
    
    def test_equality(self):
        """Test Vec2 equality."""
        v1 = Vec2(1.0, 2.0)
        v2 = Vec2(1.0, 2.0)
        v3 = Vec2(3.0, 4.0)
        self.assertEqual(v1, v2)
        self.assertNotEqual(v1, v3)
    
    def test_repr(self):
        """Test Vec2 string representation."""
        v = Vec2(1.0, 2.0)
        self.assertEqual(repr(v), "Vec2(1.0, 2.0)")


class TestVec3(unittest.TestCase):
    """Tests for Vec3."""
    
    def test_creation(self):
        """Test Vec3 creation."""
        v = Vec3(1.0, 2.0, 3.0)
        self.assertEqual(v.x, 1.0)
        self.assertEqual(v.y, 2.0)
        self.assertEqual(v.z, 3.0)
    
    def test_default_values(self):
        """Test Vec3 default values."""
        v = Vec3()
        self.assertEqual(v.x, 0.0)
        self.assertEqual(v.y, 0.0)
        self.assertEqual(v.z, 0.0)
    
    def test_property_setters(self):
        """Test Vec3 property setters."""
        v = Vec3(1.0, 2.0, 3.0)
        v.x = 4.0
        v.y = 5.0
        v.z = 6.0
        self.assertEqual(v.x, 4.0)
        self.assertEqual(v.y, 5.0)
        self.assertEqual(v.z, 6.0)
    
    def test_addition(self):
        """Test Vec3 addition."""
        v1 = Vec3(1.0, 2.0, 3.0)
        v2 = Vec3(4.0, 5.0, 6.0)
        v3 = v1 + v2
        self.assertEqual(v3.x, 5.0)
        self.assertEqual(v3.y, 7.0)
        self.assertEqual(v3.z, 9.0)
    
    def test_subtraction(self):
        """Test Vec3 subtraction."""
        v1 = Vec3(5.0, 6.0, 7.0)
        v2 = Vec3(1.0, 2.0, 3.0)
        v3 = v1 - v2
        self.assertEqual(v3.x, 4.0)
        self.assertEqual(v3.y, 4.0)
        self.assertEqual(v3.z, 4.0)
    
    def test_scalar_multiplication(self):
        """Test Vec3 scalar multiplication."""
        v1 = Vec3(2.0, 3.0, 4.0)
        v2 = v1 * 2.0
        self.assertEqual(v2.x, 4.0)
        self.assertEqual(v2.y, 6.0)
        self.assertEqual(v2.z, 8.0)
    
    def test_scalar_division(self):
        """Test Vec3 scalar division."""
        v1 = Vec3(4.0, 6.0, 8.0)
        v2 = v1 / 2.0
        self.assertEqual(v2.x, 2.0)
        self.assertEqual(v2.y, 3.0)
        self.assertEqual(v2.z, 4.0)
    
    def test_equality(self):
        """Test Vec3 equality."""
        v1 = Vec3(1.0, 2.0, 3.0)
        v2 = Vec3(1.0, 2.0, 3.0)
        v3 = Vec3(4.0, 5.0, 6.0)
        self.assertEqual(v1, v2)
        self.assertNotEqual(v1, v3)
    
    def test_repr(self):
        """Test Vec3 string representation."""
        v = Vec3(1.0, 2.0, 3.0)
        self.assertEqual(repr(v), "Vec3(1.0, 2.0, 3.0)")


class TestColor(unittest.TestCase):
    """Tests for Color."""
    
    def test_creation(self):
        """Test Color creation."""
        c = Color(1.0, 0.5, 0.0, 1.0)
        self.assertEqual(c.r, 1.0)
        self.assertEqual(c.g, 0.5)
        self.assertEqual(c.b, 0.0)
        self.assertEqual(c.a, 1.0)
    
    def test_default_values(self):
        """Test Color default values."""
        c = Color()
        self.assertEqual(c.r, 1.0)
        self.assertEqual(c.g, 1.0)
        self.assertEqual(c.b, 1.0)
        self.assertEqual(c.a, 1.0)
    
    def test_property_setters(self):
        """Test Color property setters."""
        c = Color(1.0, 0.5, 0.0, 1.0)
        c.r = 0.5
        c.g = 0.25
        c.b = 0.75
        c.a = 0.5
        self.assertEqual(c.r, 0.5)
        self.assertEqual(c.g, 0.25)
        self.assertEqual(c.b, 0.75)
        self.assertEqual(c.a, 0.5)
    
    def test_predefined_colors(self):
        """Test predefined colors."""
        self.assertEqual(Color.white(), Color(1.0, 1.0, 1.0, 1.0))
        self.assertEqual(Color.black(), Color(0.0, 0.0, 0.0, 1.0))
        self.assertEqual(Color.red(), Color(1.0, 0.0, 0.0, 1.0))
        self.assertEqual(Color.green(), Color(0.0, 1.0, 0.0, 1.0))
        self.assertEqual(Color.blue(), Color(0.0, 0.0, 1.0, 1.0))
        self.assertEqual(Color.yellow(), Color(1.0, 1.0, 0.0, 1.0))
        self.assertEqual(Color.magenta(), Color(1.0, 0.0, 1.0, 1.0))
        self.assertEqual(Color.cyan(), Color(0.0, 1.0, 1.0, 1.0))
    
    def test_equality(self):
        """Test Color equality."""
        c1 = Color(1.0, 0.5, 0.0, 1.0)
        c2 = Color(1.0, 0.5, 0.0, 1.0)
        c3 = Color(0.0, 0.5, 1.0, 1.0)
        self.assertEqual(c1, c2)
        self.assertNotEqual(c1, c3)
    
    def test_repr(self):
        """Test Color string representation."""
        c = Color(1.0, 0.5, 0.0, 1.0)
        self.assertEqual(repr(c), "Color(1.0, 0.5, 0.0, 1.0)")


if __name__ == '__main__':
    unittest.main()

