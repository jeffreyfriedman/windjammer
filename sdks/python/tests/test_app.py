"""Tests for app module."""

import pytest
from windjammer_sdk import App


class TestApp:
    """Tests for App class."""
    
    def test_creation(self):
        app = App()
        assert app is not None
        assert not app.is_running()
    
    def test_add_system(self):
        app = App()
        
        called = []
        
        @app.system
        def test_system():
            called.append(True)
        
        assert len(app._systems) == 1
    
    def test_add_startup_system(self):
        app = App()
        
        @app.startup
        def test_startup():
            pass
        
        assert len(app._startup_systems) == 1
    
    def test_add_shutdown_system(self):
        app = App()
        
        @app.shutdown
        def test_shutdown():
            pass
        
        assert len(app._shutdown_systems) == 1
    
    def test_run(self):
        app = App()
        
        called = []
        
        @app.startup
        def startup():
            called.append("startup")
        
        @app.system
        def update():
            called.append("update")
        
        @app.shutdown
        def shutdown():
            called.append("shutdown")
        
        app.run()
        
        assert "startup" in called
        assert "update" in called
        assert "shutdown" in called

