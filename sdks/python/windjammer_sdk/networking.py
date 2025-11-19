"""Networking components."""


class NetworkClient:
    """Network client."""
    
    def __init__(self, server_address: str = "127.0.0.1:7777"):
        self.server_address = server_address
        self.connected = False
    
    def connect(self) -> bool:
        """Connect to the server."""
        print(f"[Network] Connecting to {self.server_address}...")
        self.connected = True
        return True
    
    def disconnect(self) -> None:
        """Disconnect from the server."""
        print("[Network] Disconnecting...")
        self.connected = False
    
    def send(self, message: bytes) -> None:
        """Send a message to the server."""
        if self.connected:
            print(f"[Network] Sending {len(message)} bytes")
    
    def __repr__(self) -> str:
        return f"NetworkClient(server='{self.server_address}', connected={self.connected})"


class NetworkServer:
    """Network server."""
    
    def __init__(self, port: int = 7777):
        self.port = port
        self.running = False
    
    def start(self) -> bool:
        """Start the server."""
        print(f"[Network] Starting server on port {self.port}...")
        self.running = True
        return True
    
    def stop(self) -> None:
        """Stop the server."""
        print("[Network] Stopping server...")
        self.running = False
    
    def __repr__(self) -> str:
        return f"NetworkServer(port={self.port}, running={self.running})"

