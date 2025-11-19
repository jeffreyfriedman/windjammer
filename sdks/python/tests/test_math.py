"""Tests for math module."""

import pytest
from windjammer_sdk.math import Vec2, Vec3, Vec4, Quat


class TestVec2:
    """Tests for Vec2 class."""
    
    def test_creation(self):
        v = Vec2(1.0, 2.0)
        assert v.x == 1.0
        assert v.y == 2.0
    
    def test_addition(self):
        v1 = Vec2(1.0, 2.0)
        v2 = Vec2(3.0, 4.0)
        v3 = v1 + v2
        assert v3.x == 4.0
        assert v3.y == 6.0
    
    def test_subtraction(self):
        v1 = Vec2(5.0, 6.0)
        v2 = Vec2(2.0, 3.0)
        v3 = v1 - v2
        assert v3.x == 3.0
        assert v3.y == 3.0
    
    def test_multiplication(self):
        v1 = Vec2(2.0, 3.0)
        v2 = v1 * 2.0
        assert v2.x == 4.0
        assert v2.y == 6.0
    
    def test_length(self):
        v = Vec2(3.0, 4.0)
        assert v.length() == 5.0
    
    def test_normalize(self):
        v = Vec2(3.0, 4.0)
        n = v.normalize()
        assert abs(n.length() - 1.0) < 0.001
    
    def test_dot(self):
        v1 = Vec2(1.0, 2.0)
        v2 = Vec2(3.0, 4.0)
        assert v1.dot(v2) == 11.0
    
    def test_zero(self):
        v = Vec2.zero()
        assert v.x == 0.0
        assert v.y == 0.0
    
    def test_one(self):
        v = Vec2.one()
        assert v.x == 1.0
        assert v.y == 1.0


class TestVec3:
    """Tests for Vec3 class."""
    
    def test_creation(self):
        v = Vec3(1.0, 2.0, 3.0)
        assert v.x == 1.0
        assert v.y == 2.0
        assert v.z == 3.0
    
    def test_addition(self):
        v1 = Vec3(1.0, 2.0, 3.0)
        v2 = Vec3(4.0, 5.0, 6.0)
        v3 = v1 + v2
        assert v3.x == 5.0
        assert v3.y == 7.0
        assert v3.z == 9.0
    
    def test_cross(self):
        v1 = Vec3(1.0, 0.0, 0.0)
        v2 = Vec3(0.0, 1.0, 0.0)
        v3 = v1.cross(v2)
        assert v3.x == 0.0
        assert v3.y == 0.0
        assert v3.z == 1.0
    
    def test_up(self):
        v = Vec3.up()
        assert v.x == 0.0
        assert v.y == 1.0
        assert v.z == 0.0
    
    def test_forward(self):
        v = Vec3.forward()
        assert v.x == 0.0
        assert v.y == 0.0
        assert v.z == -1.0
    
    def test_right(self):
        v = Vec3.right()
        assert v.x == 1.0
        assert v.y == 0.0
        assert v.z == 0.0


class TestQuat:
    """Tests for Quat class."""
    
    def test_identity(self):
        q = Quat.identity()
        assert q.x == 0.0
        assert q.y == 0.0
        assert q.z == 0.0
        assert q.w == 1.0

