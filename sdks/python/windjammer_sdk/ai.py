"""AI components."""

from typing import List, Callable
from .math import Vec3


class BehaviorTree:
    """Behavior tree for AI decision making."""
    
    def __init__(self):
        self.root = None
    
    def tick(self) -> str:
        """Execute one tick of the behavior tree."""
        return "success"
    
    def __repr__(self) -> str:
        return "BehaviorTree()"


class Pathfinder:
    """Pathfinding component."""
    
    def __init__(self):
        self.path: List[Vec3] = []
    
    def find_path(self, start: Vec3, goal: Vec3) -> List[Vec3]:
        """Find a path from start to goal."""
        print(f"[Pathfinding] Finding path from {start} to {goal}")
        return [start, goal]
    
    def __repr__(self) -> str:
        return f"Pathfinder(path_length={len(self.path)})"

