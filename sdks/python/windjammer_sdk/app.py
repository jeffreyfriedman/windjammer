"""
Main application class for Windjammer games.
"""

from typing import Callable, List, Optional, Any
import sys


class App:
    """
    Main application class for Windjammer games.
    
    Example:
        >>> app = App()
        >>> @app.system
        >>> def update():
        >>>     print("Update!")
        >>> app.run()
    """
    
    def __init__(self):
        """Initialize a new Windjammer application."""
        self._systems: List[Callable] = []
        self._startup_systems: List[Callable] = []
        self._shutdown_systems: List[Callable] = []
        self._running = False
        
        # TODO: Initialize FFI connection to Rust backend
        print("[Windjammer] Initializing application...")
    
    def system(self, func: Callable) -> Callable:
        """
        Decorator to register a system that runs every frame.
        
        Args:
            func: The system function to register
            
        Returns:
            The original function (for chaining)
            
        Example:
            >>> @app.system
            >>> def update():
            >>>     print("Update!")
        """
        self._systems.append(func)
        return func
    
    def startup(self, func: Callable) -> Callable:
        """
        Decorator to register a startup system that runs once at the beginning.
        
        Args:
            func: The startup system function to register
            
        Returns:
            The original function (for chaining)
            
        Example:
            >>> @app.startup
            >>> def setup():
            >>>     print("Setup!")
        """
        self._startup_systems.append(func)
        return func
    
    def shutdown(self, func: Callable) -> Callable:
        """
        Decorator to register a shutdown system that runs once at the end.
        
        Args:
            func: The shutdown system function to register
            
        Returns:
            The original function (for chaining)
            
        Example:
            >>> @app.shutdown
            >>> def cleanup():
            >>>     print("Cleanup!")
        """
        self._shutdown_systems.append(func)
        return func
    
    def add_system(self, system: Callable) -> 'App':
        """
        Add a system programmatically.
        
        Args:
            system: The system function to add
            
        Returns:
            Self for chaining
        """
        self._systems.append(system)
        return self
    
    def add_startup_system(self, system: Callable) -> 'App':
        """
        Add a startup system programmatically.
        
        Args:
            system: The startup system function to add
            
        Returns:
            Self for chaining
        """
        self._startup_systems.append(system)
        return self
    
    def run(self) -> None:
        """
        Run the application.
        
        This starts the game loop and runs until the application is closed.
        """
        print(f"[Windjammer] Starting application with {len(self._systems)} systems")
        
        # Run startup systems
        for system in self._startup_systems:
            try:
                system()
            except Exception as e:
                print(f"[Windjammer] Error in startup system: {e}", file=sys.stderr)
        
        self._running = True
        
        # TODO: Start actual game loop with FFI
        # For now, just run systems once as a demonstration
        print("[Windjammer] Running systems...")
        for system in self._systems:
            try:
                system()
            except Exception as e:
                print(f"[Windjammer] Error in system: {e}", file=sys.stderr)
        
        # Run shutdown systems
        for system in self._shutdown_systems:
            try:
                system()
            except Exception as e:
                print(f"[Windjammer] Error in shutdown system: {e}", file=sys.stderr)
        
        print("[Windjammer] Application finished")
        self._running = False
    
    def is_running(self) -> bool:
        """Check if the application is currently running."""
        return self._running
    
    def quit(self) -> None:
        """Request the application to quit."""
        self._running = False
        print("[Windjammer] Quit requested")

