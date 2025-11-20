"""
FFI bindings for Windjammer C API.

This module provides low-level ctypes bindings to the Windjammer C FFI layer.
"""

from ctypes import *
import os
import platform
from typing import Optional

# ============================================================================
# Library Loading
# ============================================================================

def _get_library_path() -> str:
    """Get the path to the Windjammer C FFI library."""
    system = platform.system()
    
    if system == "Linux":
        lib_name = "libwindjammer_c_ffi.so"
    elif system == "Darwin":  # macOS
        lib_name = "libwindjammer_c_ffi.dylib"
    elif system == "Windows":
        lib_name = "windjammer_c_ffi.dll"
    else:
        raise RuntimeError(f"Unsupported platform: {system}")
    
    # Try to find the library
    # 1. Same directory as this file
    lib_path = os.path.join(os.path.dirname(__file__), lib_name)
    if os.path.exists(lib_path):
        return lib_path
    
    # 2. System library path (will be searched by CDLL)
    return lib_name

# Load the library
try:
    _lib = CDLL(_get_library_path())
except OSError as e:
    # For now, create a mock library for development
    # TODO: Remove this once the library is built
    print(f"Warning: Could not load Windjammer C FFI library: {e}")
    print("Running in mock mode for development")
    _lib = None

# ============================================================================
# C Types
# ============================================================================

class WjVec2(Structure):
    """2D vector."""
    _fields_ = [
        ("x", c_float),
        ("y", c_float),
    ]

class WjVec3(Structure):
    """3D vector."""
    _fields_ = [
        ("x", c_float),
        ("y", c_float),
        ("z", c_float),
    ]

class WjVec4(Structure):
    """4D vector."""
    _fields_ = [
        ("x", c_float),
        ("y", c_float),
        ("z", c_float),
        ("w", c_float),
    ]

class WjColor(Structure):
    """RGBA color."""
    _fields_ = [
        ("r", c_float),
        ("g", c_float),
        ("b", c_float),
        ("a", c_float),
    ]

# Opaque pointer types
WjEngine = c_void_p
WjWindow = c_void_p
WjEntity = c_void_p
WjWorld = c_void_p
WjCamera2D = c_void_p
WjCamera3D = c_void_p
WjSprite = c_void_p
WjMesh = c_void_p
WjMaterial = c_void_p
WjTexture = c_void_p
WjPointLight = c_void_p
WjDirectionalLight = c_void_p
WjRigidBody = c_void_p
WjCollider = c_void_p
WjSound = c_void_p
WjAudioSource = c_void_p
WjBehaviorTree = c_void_p
WjStateMachine = c_void_p
WjNetworkConnection = c_void_p
WjAnimationClip = c_void_p
WjWidget = c_void_p

# Error codes
class WjErrorCode(c_int):
    """Error codes returned by FFI functions."""
    Ok = 0
    NullPointer = 1
    InvalidArgument = 2
    OutOfMemory = 3
    Panic = 4
    Unknown = 5

# ============================================================================
# Error Handling
# ============================================================================

class WindjammerError(Exception):
    """Base exception for Windjammer errors."""
    pass

def _check_error():
    """Check if an error occurred in the last FFI call."""
    if _lib is None:
        return  # Mock mode
    
    error = _lib.wj_get_last_error()
    if error:
        msg = string_at(error).decode('utf-8')
        _lib.wj_clear_last_error()
        raise WindjammerError(msg)

# ============================================================================
# Function Signatures
# ============================================================================

if _lib is not None:
    # Error handling
    _lib.wj_get_last_error.argtypes = []
    _lib.wj_get_last_error.restype = c_char_p
    
    _lib.wj_clear_last_error.argtypes = []
    _lib.wj_clear_last_error.restype = None
    
    # Math types
    _lib.wj_vec2_new.argtypes = [c_float, c_float]
    _lib.wj_vec2_new.restype = WjVec2
    
    _lib.wj_vec3_new.argtypes = [c_float, c_float, c_float]
    _lib.wj_vec3_new.restype = WjVec3
    
    _lib.wj_vec4_new.argtypes = [c_float, c_float, c_float, c_float]
    _lib.wj_vec4_new.restype = WjVec4
    
    _lib.wj_color_new.argtypes = [c_float, c_float, c_float, c_float]
    _lib.wj_color_new.restype = WjColor
    
    # Version
    _lib.wj_version_major.argtypes = []
    _lib.wj_version_major.restype = c_uint
    
    _lib.wj_version_minor.argtypes = []
    _lib.wj_version_minor.restype = c_uint
    
    _lib.wj_version_patch.argtypes = []
    _lib.wj_version_patch.restype = c_uint
    
    # TODO: Add more function signatures as needed

# ============================================================================
# Helper Functions
# ============================================================================

def get_version() -> tuple[int, int, int]:
    """Get the Windjammer version."""
    if _lib is None:
        return (0, 1, 0)  # Mock version
    
    major = _lib.wj_version_major()
    minor = _lib.wj_version_minor()
    patch = _lib.wj_version_patch()
    return (major, minor, patch)

