"""
Windjammer Python SDK

Python bindings for the Windjammer Game Engine.

Example:
    >>> from windjammer_sdk import App, Vec3
    >>> app = App()
    >>> app.run()
"""

__version__ = "0.1.0"
__author__ = "Windjammer Contributors"
__license__ = "MIT OR Apache-2.0"

# Core imports
from .app import App
from .math import Vec2, Vec3, Color
from .ffi import get_version
from .transform import Transform
from .time import Time
from .input import Input, KeyCode, MouseButton

# 2D imports
from .sprite import Sprite, Camera2D

# 3D imports
from .mesh import Mesh, Material, Camera3D, PointLight, DirectionalLight, SpotLight

# Physics imports
from .physics import RigidBody, Collider

# Audio imports
from .audio import AudioSource, AudioListener

# Networking imports
from .networking import NetworkClient, NetworkServer

# AI imports
from .ai import BehaviorTree, Pathfinder

# All exports
__all__ = [
    # Core
    "App",
    "Vec2",
    "Vec3",
    "Color",
    "get_version",
    "Transform",
    "Time",
    "Input",
    "KeyCode",
    "MouseButton",
    # 2D
    "Sprite",
    "Camera2D",
    # 3D
    "Mesh",
    "Material",
    "Camera3D",
    "PointLight",
    "DirectionalLight",
    "SpotLight",
    # Physics
    "RigidBody",
    "Collider",
    # Audio
    "AudioSource",
    "AudioListener",
    # Networking
    "NetworkClient",
    "NetworkServer",
    # AI
    "BehaviorTree",
    "Pathfinder",
]

