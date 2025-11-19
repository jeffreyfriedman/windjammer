"""Time utilities."""


class Time:
    """Time information for the current frame."""
    
    def __init__(self):
        self.delta_seconds = 0.016  # ~60 FPS
        self.total_seconds = 0.0
        self.frame_count = 0
    
    def __repr__(self) -> str:
        return f"Time(delta={self.delta_seconds}, total={self.total_seconds})"

