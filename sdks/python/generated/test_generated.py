"""Test generated Python SDK"""
import sys
sys.path.insert(0, '.')

# Import generated modules
from app import App
from vec2 import Vec2
from vec3 import Vec3
from world import World
from entity import Entity
from key import Key
from mousebutton import MouseButton

def test_imports():
    """Test that all generated modules can be imported"""
    print("✓ All imports successful")

def test_vec2():
    """Test Vec2 struct"""
    v = Vec2(1.0, 2.0)
    assert v.x == 1.0
    assert v.y == 2.0
    print("✓ Vec2 works")

def test_vec3():
    """Test Vec3 struct"""
    v = Vec3(1.0, 2.0, 3.0)
    assert v.x == 1.0
    assert v.y == 2.0
    assert v.z == 3.0
    print("✓ Vec3 works")

def test_enums():
    """Test enum values"""
    assert hasattr(Key, 'A')
    assert hasattr(MouseButton, 'Left')
    print("✓ Enums work")

def test_classes():
    """Test class instantiation"""
    app = App()
    world = World()
    entity = Entity()
    print("✓ Classes instantiate")

if __name__ == '__main__':
    test_imports()
    test_vec2()
    test_vec3()
    test_enums()
    test_classes()
    print("\n🎉 All generated SDK tests passed!")

