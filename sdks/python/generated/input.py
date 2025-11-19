class Input:
    """Input manager for keyboard and mouse"""
    def __init__(self):
        pass

    def is_key_pressed(self, key):
        """Returns true if key was just pressed this frame"""
        pass

    def is_key_held(self, key):
        """Returns true if key is currently held down"""
        pass

    def is_mouse_button_pressed(self, button):
        """Returns true if mouse button was just pressed"""
        pass

    def mouse_position(self):
        """Returns current mouse position"""
        pass

