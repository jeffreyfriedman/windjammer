"""Audio components."""

from .math import Vec3


class AudioSource:
    """Audio source component."""
    
    def __init__(self, audio_file: str = "", volume: float = 1.0, looping: bool = False):
        self.audio_file = audio_file
        self.volume = volume
        self.looping = looping
        self.position = Vec3.zero()
    
    def play(self) -> None:
        """Play the audio."""
        print(f"[Audio] Playing: {self.audio_file}")
    
    def stop(self) -> None:
        """Stop the audio."""
        print(f"[Audio] Stopping: {self.audio_file}")
    
    def __repr__(self) -> str:
        return f"AudioSource(file='{self.audio_file}', volume={self.volume})"


class AudioListener:
    """Audio listener component."""
    
    def __init__(self, position: Vec3 = None):
        self.position = position or Vec3.zero()
    
    def __repr__(self) -> str:
        return f"AudioListener(pos={self.position})"

