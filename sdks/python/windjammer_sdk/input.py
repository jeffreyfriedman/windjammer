"""Input handling."""

from enum import Enum


class KeyCode(Enum):
    """Keyboard key codes."""
    A = "A"
    B = "B"
    C = "C"
    D = "D"
    E = "E"
    F = "F"
    W = "W"
    S = "S"
    SPACE = "Space"
    ESCAPE = "Escape"


class MouseButton(Enum):
    """Mouse button codes."""
    LEFT = 0
    RIGHT = 1
    MIDDLE = 2


class Input:
    """Input state manager."""
    
    def __init__(self):
        self._keys = set()
        self._mouse_buttons = set()
    
    def is_key_pressed(self, key: KeyCode) -> bool:
        """Check if a key is currently pressed."""
        return key in self._keys
    
    def is_mouse_button_pressed(self, button: MouseButton) -> bool:
        """Check if a mouse button is currently pressed."""
        return button in self._mouse_buttons

