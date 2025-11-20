"""
Application management for Windjammer.

This module provides the main App class for creating and running games.
"""

from typing import Callable, Optional
from .ffi import _lib, WjEngine, WjWorld, _check_error

class App:
    """Main application class for Windjammer games."""
    
    def __init__(self, title: str = "Windjammer Game", width: int = 1280, height: int = 720):
        """
        Create a new Windjammer application.
        
        Args:
            title: Window title
            width: Window width in pixels
            height: Window height in pixels
        """
        self.title = title
        self.width = width
        self.height = height
        self._engine_handle: Optional[WjEngine] = None
        self._world_handle: Optional[WjWorld] = None
        self._startup_systems = []
        self._update_systems = []
        self._shutdown_systems = []
    
    def add_startup_system(self, system: Callable[[], None]):
        """
        Add a system that runs once at startup.
        
        Args:
            system: Function to run at startup
        """
        self._startup_systems.append(system)
        return self
    
    def add_system(self, system: Callable[[], None]):
        """
        Add a system that runs every frame.
        
        Args:
            system: Function to run each frame
        """
        self._update_systems.append(system)
        return self
    
    def add_shutdown_system(self, system: Callable[[], None]):
        """
        Add a system that runs once at shutdown.
        
        Args:
            system: Function to run at shutdown
        """
        self._shutdown_systems.append(system)
        return self
    
    def run(self):
        """Run the application."""
        try:
            # Initialize engine (mock mode for now)
            print(f"Starting {self.title} ({self.width}x{self.height})")
            
            # Run startup systems
            for system in self._startup_systems:
                system()
            
            # TODO: Implement actual game loop with FFI
            # For now, just run update systems once
            print("Running game loop...")
            for system in self._update_systems:
                system()
            
            # Run shutdown systems
            for system in self._shutdown_systems:
                system()
            
            print("Game finished")
            
        except Exception as e:
            print(f"Error running game: {e}")
            raise
    
    def __repr__(self) -> str:
        return f"App(title='{self.title}', size=({self.width}, {self.height}))"


class World:
    """ECS World for managing entities and components."""
    
    def __init__(self):
        """Create a new ECS world."""
        self._world_handle: Optional[WjWorld] = None
        # TODO: Initialize actual world with FFI
    
    def spawn_entity(self) -> 'Entity':
        """
        Spawn a new entity.
        
        Returns:
            New entity
        """
        # TODO: Use FFI to spawn entity
        return Entity()
    
    def __repr__(self) -> str:
        return "World()"


class Entity:
    """Represents a game entity in the ECS."""
    
    def __init__(self):
        """Create a new entity."""
        self._entity_id = 0  # TODO: Get from FFI
    
    def add_component(self, component):
        """
        Add a component to this entity.
        
        Args:
            component: Component to add
        """
        # TODO: Use FFI to add component
        pass
    
    def get_component(self, component_type):
        """
        Get a component from this entity.
        
        Args:
            component_type: Type of component to get
        
        Returns:
            Component instance or None
        """
        # TODO: Use FFI to get component
        return None
    
    def remove_component(self, component_type):
        """
        Remove a component from this entity.
        
        Args:
            component_type: Type of component to remove
        """
        # TODO: Use FFI to remove component
        pass
    
    def __repr__(self) -> str:
        return f"Entity(id={self._entity_id})"
