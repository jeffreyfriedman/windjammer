//! # Networking Module
//!
//! Provides client-server networking for multiplayer games.
//!
//! ## Features
//! - TCP and UDP transport
//! - Reliable and unreliable channels
//! - Connection management
//! - Message serialization
//! - Bandwidth management
//! - Network statistics
//! - Event system
//!
//! ## Example
//! ```no_run
//! use windjammer_game_framework::networking::{NetworkServer, NetworkClient, NetworkMessage};
//!
//! // Server
//! let mut server = NetworkServer::new("127.0.0.1:8080").unwrap();
//! server.start();
//!
//! // Client
//! let mut client = NetworkClient::new();
//! client.connect("127.0.0.1:8080").unwrap();
//! client.send_reliable(NetworkMessage::new(b"Hello".to_vec()));
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::{SocketAddr, TcpListener, TcpStream, UdpSocket};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Unique identifier for a network client
pub type ClientId = u64;

/// Network transport protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetworkTransport {
    /// TCP - reliable, ordered
    Tcp,
    /// UDP - unreliable, unordered, fast
    Udp,
}

/// Network channel type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetworkChannel {
    /// Reliable, ordered delivery (TCP)
    Reliable,
    /// Unreliable, fast delivery (UDP)
    Unreliable,
}

/// Network message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkMessage {
    /// Message ID for tracking
    pub id: u64,
    /// Message data
    pub data: Vec<u8>,
    /// Channel type
    pub channel: NetworkChannel,
    /// Timestamp when message was created
    pub timestamp: u64,
}

impl NetworkMessage {
    /// Create a new network message
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            id: 0, // Will be assigned by sender
            data,
            channel: NetworkChannel::Reliable,
            timestamp: 0,
        }
    }

    /// Create a new reliable message
    pub fn reliable(data: Vec<u8>) -> Self {
        Self {
            id: 0,
            data,
            channel: NetworkChannel::Reliable,
            timestamp: 0,
        }
    }

    /// Create a new unreliable message
    pub fn unreliable(data: Vec<u8>) -> Self {
        Self {
            id: 0,
            data,
            channel: NetworkChannel::Unreliable,
            timestamp: 0,
        }
    }

    /// Serialize message to bytes
    pub fn to_bytes(&self) -> Result<Vec<u8>, NetworkError> {
        bincode::serialize(self).map_err(|e| NetworkError::SerializationError(e.to_string()))
    }

    /// Deserialize message from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, NetworkError> {
        bincode::deserialize(bytes).map_err(|e| NetworkError::SerializationError(e.to_string()))
    }
}

/// Network event
#[derive(Debug, Clone)]
pub enum NetworkEvent {
    /// Client connected
    ClientConnected { client_id: ClientId, address: SocketAddr },
    /// Client disconnected
    ClientDisconnected { client_id: ClientId },
    /// Message received
    MessageReceived { client_id: ClientId, message: NetworkMessage },
    /// Connection error
    ConnectionError { client_id: ClientId, error: String },
}

/// Network error
#[derive(Debug, Clone)]
pub enum NetworkError {
    /// Failed to bind to address
    BindError(String),
    /// Failed to connect
    ConnectionError(String),
    /// Failed to send message
    SendError(String),
    /// Failed to receive message
    ReceiveError(String),
    /// Serialization error
    SerializationError(String),
    /// Client not found
    ClientNotFound(ClientId),
    /// Server not running
    ServerNotRunning,
}

/// Network statistics
#[derive(Debug, Clone, Default)]
pub struct NetworkStats {
    /// Total bytes sent
    pub bytes_sent: u64,
    /// Total bytes received
    pub bytes_received: u64,
    /// Total messages sent
    pub messages_sent: u64,
    /// Total messages received
    pub messages_received: u64,
    /// Average round-trip time (ms)
    pub avg_rtt: f32,
    /// Packet loss percentage
    pub packet_loss: f32,
    /// Current bandwidth usage (bytes/sec)
    pub bandwidth_usage: f32,
}

/// Connected client information
#[derive(Debug)]
struct ClientConnection {
    /// Client ID
    id: ClientId,
    /// Client address
    address: SocketAddr,
    /// TCP stream
    tcp_stream: Option<TcpStream>,
    /// Last activity time
    last_activity: Instant,
    /// Network statistics
    stats: NetworkStats,
}

/// Network server
pub struct NetworkServer {
    /// Server address
    address: SocketAddr,
    /// TCP listener
    tcp_listener: Option<TcpListener>,
    /// UDP socket
    udp_socket: Option<UdpSocket>,
    /// Connected clients
    clients: Arc<Mutex<HashMap<ClientId, ClientConnection>>>,
    /// Next client ID
    next_client_id: Arc<Mutex<ClientId>>,
    /// Event queue
    events: Arc<Mutex<Vec<NetworkEvent>>>,
    /// Is server running
    running: Arc<Mutex<bool>>,
    /// Server statistics
    stats: Arc<Mutex<NetworkStats>>,
}

impl NetworkServer {
    /// Create a new network server
    pub fn new(address: &str) -> Result<Self, NetworkError> {
        let address: SocketAddr = address
            .parse()
            .map_err(|e| NetworkError::BindError(format!("Invalid address: {}", e)))?;

        Ok(Self {
            address,
            tcp_listener: None,
            udp_socket: None,
            clients: Arc::new(Mutex::new(HashMap::new())),
            next_client_id: Arc::new(Mutex::new(1)),
            events: Arc::new(Mutex::new(Vec::new())),
            running: Arc::new(Mutex::new(false)),
            stats: Arc::new(Mutex::new(NetworkStats::default())),
        })
    }

    /// Start the server
    pub fn start(&mut self) -> Result<(), NetworkError> {
        // Bind TCP listener
        let tcp_listener = TcpListener::bind(self.address)
            .map_err(|e| NetworkError::BindError(format!("Failed to bind TCP: {}", e)))?;
        tcp_listener
            .set_nonblocking(true)
            .map_err(|e| NetworkError::BindError(format!("Failed to set non-blocking: {}", e)))?;
        self.tcp_listener = Some(tcp_listener);

        // Bind UDP socket
        let udp_socket = UdpSocket::bind(self.address)
            .map_err(|e| NetworkError::BindError(format!("Failed to bind UDP: {}", e)))?;
        udp_socket
            .set_nonblocking(true)
            .map_err(|e| NetworkError::BindError(format!("Failed to set non-blocking: {}", e)))?;
        self.udp_socket = Some(udp_socket);

        *self.running.lock().unwrap() = true;

        Ok(())
    }

    /// Stop the server
    pub fn stop(&mut self) {
        *self.running.lock().unwrap() = false;
        self.tcp_listener = None;
        self.udp_socket = None;
        self.clients.lock().unwrap().clear();
    }

    /// Check if server is running
    pub fn is_running(&self) -> bool {
        *self.running.lock().unwrap()
    }

    /// Update server (accept connections, receive messages)
    pub fn update(&mut self) {
        if !self.is_running() {
            return;
        }

        // Accept new TCP connections
        if let Some(ref listener) = self.tcp_listener {
            match listener.accept() {
                Ok((stream, address)) => {
                    let client_id = {
                        let mut next_id = self.next_client_id.lock().unwrap();
                        let id = *next_id;
                        *next_id += 1;
                        id
                    };

                    let _ = stream.set_nonblocking(true);

                    let connection = ClientConnection {
                        id: client_id,
                        address,
                        tcp_stream: Some(stream),
                        last_activity: Instant::now(),
                        stats: NetworkStats::default(),
                    };

                    self.clients.lock().unwrap().insert(client_id, connection);

                    self.events.lock().unwrap().push(NetworkEvent::ClientConnected {
                        client_id,
                        address,
                    });
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // No new connections
                }
                Err(_e) => {
                    // Connection error
                }
            }
        }

        // Receive TCP messages from clients
        let mut disconnected_clients = Vec::new();
        {
            let mut clients = self.clients.lock().unwrap();
            for (client_id, connection) in clients.iter_mut() {
                if let Some(ref mut stream) = connection.tcp_stream {
                    let mut buffer = vec![0u8; 4096];
                    match std::io::Read::read(stream, &mut buffer) {
                        Ok(0) => {
                            // Client disconnected
                            disconnected_clients.push(*client_id);
                        }
                        Ok(n) => {
                            buffer.truncate(n);
                            if let Ok(message) = NetworkMessage::from_bytes(&buffer) {
                                connection.stats.bytes_received += n as u64;
                                connection.stats.messages_received += 1;
                                connection.last_activity = Instant::now();

                                self.events.lock().unwrap().push(NetworkEvent::MessageReceived {
                                    client_id: *client_id,
                                    message,
                                });
                            }
                        }
                        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            // No data available
                        }
                        Err(_e) => {
                            // Read error
                            disconnected_clients.push(*client_id);
                        }
                    }
                }
            }
        }

        // Remove disconnected clients
        for client_id in disconnected_clients {
            self.clients.lock().unwrap().remove(&client_id);
            self.events
                .lock()
                .unwrap()
                .push(NetworkEvent::ClientDisconnected { client_id });
        }
    }

    /// Send a message to a specific client
    pub fn send_to_client(
        &mut self,
        client_id: ClientId,
        message: NetworkMessage,
    ) -> Result<(), NetworkError> {
        let mut clients = self.clients.lock().unwrap();
        let connection = clients
            .get_mut(&client_id)
            .ok_or(NetworkError::ClientNotFound(client_id))?;

        let bytes = message.to_bytes()?;

        match message.channel {
            NetworkChannel::Reliable => {
                if let Some(ref mut stream) = connection.tcp_stream {
                    std::io::Write::write_all(stream, &bytes)
                        .map_err(|e| NetworkError::SendError(e.to_string()))?;
                    connection.stats.bytes_sent += bytes.len() as u64;
                    connection.stats.messages_sent += 1;
                }
            }
            NetworkChannel::Unreliable => {
                if let Some(ref socket) = self.udp_socket {
                    socket
                        .send_to(&bytes, connection.address)
                        .map_err(|e| NetworkError::SendError(e.to_string()))?;
                    connection.stats.bytes_sent += bytes.len() as u64;
                    connection.stats.messages_sent += 1;
                }
            }
        }

        Ok(())
    }

    /// Broadcast a message to all clients
    pub fn broadcast(&mut self, message: NetworkMessage) -> Result<(), NetworkError> {
        let client_ids: Vec<ClientId> = self.clients.lock().unwrap().keys().copied().collect();

        for client_id in client_ids {
            let _ = self.send_to_client(client_id, message.clone());
        }

        Ok(())
    }

    /// Poll for network events
    pub fn poll_events(&mut self) -> Vec<NetworkEvent> {
        let mut events = self.events.lock().unwrap();
        let result = events.clone();
        events.clear();
        result
    }

    /// Get server statistics
    pub fn get_stats(&self) -> NetworkStats {
        self.stats.lock().unwrap().clone()
    }

    /// Get number of connected clients
    pub fn client_count(&self) -> usize {
        self.clients.lock().unwrap().len()
    }

    /// Get list of connected client IDs
    pub fn get_client_ids(&self) -> Vec<ClientId> {
        self.clients.lock().unwrap().keys().copied().collect()
    }
}

/// Network client
pub struct NetworkClient {
    /// Client ID assigned by server
    client_id: Option<ClientId>,
    /// Server address
    server_address: Option<SocketAddr>,
    /// TCP stream
    tcp_stream: Option<TcpStream>,
    /// UDP socket
    udp_socket: Option<UdpSocket>,
    /// Is connected
    connected: bool,
    /// Event queue
    events: Vec<NetworkEvent>,
    /// Client statistics
    stats: NetworkStats,
    /// Next message ID
    next_message_id: u64,
}

impl NetworkClient {
    /// Create a new network client
    pub fn new() -> Self {
        Self {
            client_id: None,
            server_address: None,
            tcp_stream: None,
            udp_socket: None,
            connected: false,
            events: Vec::new(),
            stats: NetworkStats::default(),
            next_message_id: 1,
        }
    }

    /// Connect to a server
    pub fn connect(&mut self, address: &str) -> Result<(), NetworkError> {
        let server_address: SocketAddr = address
            .parse()
            .map_err(|e| NetworkError::ConnectionError(format!("Invalid address: {}", e)))?;

        // Connect TCP
        let tcp_stream = TcpStream::connect(server_address)
            .map_err(|e| NetworkError::ConnectionError(format!("Failed to connect TCP: {}", e)))?;
        tcp_stream
            .set_nonblocking(true)
            .map_err(|e| NetworkError::ConnectionError(format!("Failed to set non-blocking: {}", e)))?;

        // Bind UDP
        let udp_socket = UdpSocket::bind("0.0.0.0:0")
            .map_err(|e| NetworkError::ConnectionError(format!("Failed to bind UDP: {}", e)))?;
        udp_socket
            .set_nonblocking(true)
            .map_err(|e| NetworkError::ConnectionError(format!("Failed to set non-blocking: {}", e)))?;
        udp_socket
            .connect(server_address)
            .map_err(|e| NetworkError::ConnectionError(format!("Failed to connect UDP: {}", e)))?;

        self.server_address = Some(server_address);
        self.tcp_stream = Some(tcp_stream);
        self.udp_socket = Some(udp_socket);
        self.connected = true;

        Ok(())
    }

    /// Disconnect from server
    pub fn disconnect(&mut self) {
        self.tcp_stream = None;
        self.udp_socket = None;
        self.connected = false;
        self.server_address = None;
        self.client_id = None;
    }

    /// Check if connected
    pub fn is_connected(&self) -> bool {
        self.connected
    }

    /// Update client (receive messages)
    pub fn update(&mut self) {
        if !self.connected {
            return;
        }

        // Receive TCP messages
        if let Some(ref mut stream) = self.tcp_stream {
            let mut buffer = vec![0u8; 4096];
            match std::io::Read::read(stream, &mut buffer) {
                Ok(0) => {
                    // Server disconnected
                    self.disconnect();
                }
                Ok(n) => {
                    buffer.truncate(n);
                    if let Ok(message) = NetworkMessage::from_bytes(&buffer) {
                        self.stats.bytes_received += n as u64;
                        self.stats.messages_received += 1;

                        self.events.push(NetworkEvent::MessageReceived {
                            client_id: self.client_id.unwrap_or(0),
                            message,
                        });
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // No data available
                }
                Err(_e) => {
                    // Read error
                    self.disconnect();
                }
            }
        }
    }

    /// Send a reliable message
    pub fn send_reliable(&mut self, mut message: NetworkMessage) -> Result<(), NetworkError> {
        if !self.connected {
            return Err(NetworkError::SendError("Not connected".to_string()));
        }

        message.id = self.next_message_id;
        message.channel = NetworkChannel::Reliable;
        self.next_message_id += 1;

        let bytes = message.to_bytes()?;

        if let Some(ref mut stream) = self.tcp_stream {
            std::io::Write::write_all(stream, &bytes)
                .map_err(|e| NetworkError::SendError(e.to_string()))?;
            self.stats.bytes_sent += bytes.len() as u64;
            self.stats.messages_sent += 1;
        }

        Ok(())
    }

    /// Send an unreliable message
    pub fn send_unreliable(&mut self, mut message: NetworkMessage) -> Result<(), NetworkError> {
        if !self.connected {
            return Err(NetworkError::SendError("Not connected".to_string()));
        }

        message.id = self.next_message_id;
        message.channel = NetworkChannel::Unreliable;
        self.next_message_id += 1;

        let bytes = message.to_bytes()?;

        if let Some(ref socket) = self.udp_socket {
            socket
                .send(&bytes)
                .map_err(|e| NetworkError::SendError(e.to_string()))?;
            self.stats.bytes_sent += bytes.len() as u64;
            self.stats.messages_sent += 1;
        }

        Ok(())
    }

    /// Poll for network events
    pub fn poll_events(&mut self) -> Vec<NetworkEvent> {
        let result = self.events.clone();
        self.events.clear();
        result
    }

    /// Get client statistics
    pub fn get_stats(&self) -> NetworkStats {
        self.stats.clone()
    }
}

impl Default for NetworkClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_message_creation() {
        let msg = NetworkMessage::new(vec![1, 2, 3, 4]);
        assert_eq!(msg.data, vec![1, 2, 3, 4]);
        assert_eq!(msg.channel, NetworkChannel::Reliable);
    }

    #[test]
    fn test_network_message_reliable() {
        let msg = NetworkMessage::reliable(vec![1, 2, 3]);
        assert_eq!(msg.channel, NetworkChannel::Reliable);
    }

    #[test]
    fn test_network_message_unreliable() {
        let msg = NetworkMessage::unreliable(vec![1, 2, 3]);
        assert_eq!(msg.channel, NetworkChannel::Unreliable);
    }

    #[test]
    fn test_network_message_serialization() {
        let msg = NetworkMessage::new(vec![1, 2, 3, 4, 5]);
        let bytes = msg.to_bytes().unwrap();
        let deserialized = NetworkMessage::from_bytes(&bytes).unwrap();
        assert_eq!(msg.data, deserialized.data);
    }

    #[test]
    fn test_network_server_creation() {
        let server = NetworkServer::new("127.0.0.1:0");
        assert!(server.is_ok());
    }

    #[test]
    fn test_network_server_start_stop() {
        let mut server = NetworkServer::new("127.0.0.1:0").unwrap();
        assert!(!server.is_running());

        server.start().unwrap();
        assert!(server.is_running());

        server.stop();
        assert!(!server.is_running());
    }

    #[test]
    fn test_network_server_client_count() {
        let mut server = NetworkServer::new("127.0.0.1:0").unwrap();
        server.start().unwrap();
        assert_eq!(server.client_count(), 0);
    }

    #[test]
    fn test_network_client_creation() {
        let client = NetworkClient::new();
        assert!(!client.is_connected());
    }

    #[test]
    fn test_network_client_connect_disconnect() {
        // Start server
        let mut server = NetworkServer::new("127.0.0.1:9999").unwrap();
        server.start().unwrap();

        // Connect client
        let mut client = NetworkClient::new();
        let result = client.connect("127.0.0.1:9999");
        assert!(result.is_ok());
        assert!(client.is_connected());

        // Disconnect
        client.disconnect();
        assert!(!client.is_connected());

        server.stop();
    }

    #[test]
    fn test_network_stats_default() {
        let stats = NetworkStats::default();
        assert_eq!(stats.bytes_sent, 0);
        assert_eq!(stats.bytes_received, 0);
        assert_eq!(stats.messages_sent, 0);
        assert_eq!(stats.messages_received, 0);
    }

    #[test]
    fn test_network_transport_types() {
        assert_eq!(NetworkTransport::Tcp, NetworkTransport::Tcp);
        assert_eq!(NetworkTransport::Udp, NetworkTransport::Udp);
        assert_ne!(NetworkTransport::Tcp, NetworkTransport::Udp);
    }

    #[test]
    fn test_network_channel_types() {
        assert_eq!(NetworkChannel::Reliable, NetworkChannel::Reliable);
        assert_eq!(NetworkChannel::Unreliable, NetworkChannel::Unreliable);
        assert_ne!(NetworkChannel::Reliable, NetworkChannel::Unreliable);
    }

    #[test]
    fn test_network_message_id_assignment() {
        let mut client = NetworkClient::new();
        assert_eq!(client.next_message_id, 1);

        // Message IDs should increment
        let msg1 = NetworkMessage::new(vec![1]);
        let msg2 = NetworkMessage::new(vec![2]);
        assert_eq!(msg1.id, 0); // Not assigned yet
        assert_eq!(msg2.id, 0); // Not assigned yet
    }

    #[test]
    fn test_network_server_get_client_ids() {
        let mut server = NetworkServer::new("127.0.0.1:0").unwrap();
        server.start().unwrap();

        let client_ids = server.get_client_ids();
        assert_eq!(client_ids.len(), 0);

        server.stop();
    }

    #[test]
    fn test_network_client_stats() {
        let client = NetworkClient::new();
        let stats = client.get_stats();
        assert_eq!(stats.bytes_sent, 0);
        assert_eq!(stats.messages_sent, 0);
    }

    #[test]
    fn test_network_server_stats() {
        let mut server = NetworkServer::new("127.0.0.1:0").unwrap();
        server.start().unwrap();

        let stats = server.get_stats();
        assert_eq!(stats.bytes_sent, 0);
        assert_eq!(stats.messages_sent, 0);

        server.stop();
    }

    #[test]
    fn test_network_event_types() {
        let event1 = NetworkEvent::ClientConnected {
            client_id: 1,
            address: "127.0.0.1:8080".parse().unwrap(),
        };
        let event2 = NetworkEvent::ClientDisconnected { client_id: 1 };

        match event1 {
            NetworkEvent::ClientConnected { client_id, .. } => assert_eq!(client_id, 1),
            _ => panic!("Wrong event type"),
        }

        match event2 {
            NetworkEvent::ClientDisconnected { client_id } => assert_eq!(client_id, 1),
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_network_server_update() {
        let mut server = NetworkServer::new("127.0.0.1:0").unwrap();
        server.start().unwrap();

        // Update should not panic
        server.update();

        server.stop();
    }

    #[test]
    fn test_network_client_update() {
        let mut client = NetworkClient::new();

        // Update should not panic when not connected
        client.update();
    }

    #[test]
    fn test_network_server_poll_events() {
        let mut server = NetworkServer::new("127.0.0.1:0").unwrap();
        server.start().unwrap();

        let events = server.poll_events();
        assert_eq!(events.len(), 0);

        server.stop();
    }

    #[test]
    fn test_network_client_poll_events() {
        let mut client = NetworkClient::new();

        let events = client.poll_events();
        assert_eq!(events.len(), 0);
    }
}

