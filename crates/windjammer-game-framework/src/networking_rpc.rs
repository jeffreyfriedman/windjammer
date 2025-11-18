//! # RPC (Remote Procedure Call) Module
//!
//! Provides RPC functionality for multiplayer games.
//!
//! ## Features
//! - Client-to-Server RPCs
//! - Server-to-Client RPCs
//! - Reliable and unreliable RPCs
//! - RPC with return values
//! - RPC batching
//! - RPC validation
//! - RPC rate limiting
//!
//! ## Example
//! ```no_run
//! use windjammer_game_framework::networking_rpc::{RpcManager, RpcCall, RpcTarget};
//!
//! let mut rpc_manager = RpcManager::new();
//! rpc_manager.register_handler("spawn_player", |args| {
//!     // Handle spawn player RPC
//!     Ok(vec![])
//! });
//!
//! // Call RPC
//! rpc_manager.call_rpc("spawn_player", vec![1, 2, 3], RpcTarget::Server);
//! ```

use crate::networking::{ClientId, NetworkChannel, NetworkMessage};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// RPC ID for tracking calls
pub type RpcId = u64;

/// RPC target
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RpcTarget {
    /// Send to server
    Server,
    /// Send to specific client
    Client(ClientId),
    /// Send to all clients
    AllClients,
    /// Send to all clients except one
    AllClientsExcept(ClientId),
}

/// RPC reliability
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RpcReliability {
    /// Reliable delivery (TCP)
    Reliable,
    /// Unreliable delivery (UDP)
    Unreliable,
}

impl Default for RpcReliability {
    fn default() -> Self {
        Self::Reliable
    }
}

/// RPC call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcCall {
    /// RPC ID
    pub id: RpcId,
    /// RPC name
    pub name: String,
    /// RPC arguments (serialized)
    pub args: Vec<u8>,
    /// RPC target
    pub target: RpcTarget,
    /// RPC reliability
    pub reliability: RpcReliability,
    /// Timestamp
    pub timestamp: u64,
    /// Sender client ID (None for server)
    pub sender: Option<ClientId>,
}

impl RpcCall {
    /// Create a new RPC call
    pub fn new(name: String, args: Vec<u8>, target: RpcTarget) -> Self {
        Self {
            id: 0, // Will be assigned by manager
            name,
            args,
            target,
            reliability: RpcReliability::default(),
            timestamp: current_timestamp(),
            sender: None,
        }
    }

    /// Create a reliable RPC call
    pub fn reliable(name: String, args: Vec<u8>, target: RpcTarget) -> Self {
        Self {
            id: 0,
            name,
            args,
            target,
            reliability: RpcReliability::Reliable,
            timestamp: current_timestamp(),
            sender: None,
        }
    }

    /// Create an unreliable RPC call
    pub fn unreliable(name: String, args: Vec<u8>, target: RpcTarget) -> Self {
        Self {
            id: 0,
            name,
            args,
            target,
            reliability: RpcReliability::Unreliable,
            timestamp: current_timestamp(),
            sender: None,
        }
    }

    /// Serialize RPC call to bytes
    pub fn to_bytes(&self) -> Result<Vec<u8>, RpcError> {
        bincode::serialize(self).map_err(|e| RpcError::SerializationError(e.to_string()))
    }

    /// Deserialize RPC call from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, RpcError> {
        bincode::deserialize(bytes).map_err(|e| RpcError::SerializationError(e.to_string()))
    }

    /// Convert to network message
    pub fn to_network_message(&self) -> Result<NetworkMessage, RpcError> {
        let bytes = self.to_bytes()?;
        let channel = match self.reliability {
            RpcReliability::Reliable => NetworkChannel::Reliable,
            RpcReliability::Unreliable => NetworkChannel::Unreliable,
        };
        Ok(NetworkMessage {
            id: self.id,
            data: bytes,
            channel,
            timestamp: self.timestamp,
        })
    }
}

/// RPC response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcResponse {
    /// RPC ID this response is for
    pub rpc_id: RpcId,
    /// Response data (serialized)
    pub data: Vec<u8>,
    /// Was the RPC successful
    pub success: bool,
    /// Error message (if any)
    pub error: Option<String>,
}

impl RpcResponse {
    /// Create a success response
    pub fn success(rpc_id: RpcId, data: Vec<u8>) -> Self {
        Self {
            rpc_id,
            data,
            success: true,
            error: None,
        }
    }

    /// Create an error response
    pub fn error(rpc_id: RpcId, error: String) -> Self {
        Self {
            rpc_id,
            data: Vec::new(),
            success: false,
            error: Some(error),
        }
    }

    /// Serialize response to bytes
    pub fn to_bytes(&self) -> Result<Vec<u8>, RpcError> {
        bincode::serialize(self).map_err(|e| RpcError::SerializationError(e.to_string()))
    }

    /// Deserialize response from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, RpcError> {
        bincode::deserialize(bytes).map_err(|e| RpcError::SerializationError(e.to_string()))
    }
}

/// RPC error
#[derive(Debug, Clone)]
pub enum RpcError {
    /// Serialization error
    SerializationError(String),
    /// Handler not found
    HandlerNotFound(String),
    /// RPC execution error
    ExecutionError(String),
    /// Rate limit exceeded
    RateLimitExceeded(String),
    /// Invalid arguments
    InvalidArguments(String),
}

/// RPC handler function type
pub type RpcHandler = Box<dyn Fn(&[u8]) -> Result<Vec<u8>, RpcError> + Send + Sync>;

/// RPC rate limiter
#[derive(Debug)]
struct RpcRateLimiter {
    /// Maximum calls per second
    max_calls_per_second: u32,
    /// Call timestamps
    call_times: Vec<Instant>,
}

impl RpcRateLimiter {
    /// Create a new rate limiter
    fn new(max_calls_per_second: u32) -> Self {
        Self {
            max_calls_per_second,
            call_times: Vec::new(),
        }
    }

    /// Check if a call is allowed
    fn allow_call(&mut self) -> bool {
        let now = Instant::now();
        let one_second_ago = now - Duration::from_secs(1);

        // Remove old timestamps
        self.call_times.retain(|&time| time > one_second_ago);

        // Check if we're under the limit
        if self.call_times.len() < self.max_calls_per_second as usize {
            self.call_times.push(now);
            true
        } else {
            false
        }
    }
}

/// RPC statistics
#[derive(Debug, Clone, Default)]
pub struct RpcStats {
    /// Total RPCs sent
    pub rpcs_sent: u64,
    /// Total RPCs received
    pub rpcs_received: u64,
    /// Total RPC errors
    pub rpc_errors: u64,
    /// Total rate limit hits
    pub rate_limit_hits: u64,
}

/// RPC manager
pub struct RpcManager {
    /// RPC handlers
    handlers: HashMap<String, RpcHandler>,
    /// Pending RPC calls
    pending_calls: Vec<RpcCall>,
    /// Pending RPC responses
    pending_responses: Vec<RpcResponse>,
    /// Next RPC ID
    next_rpc_id: RpcId,
    /// Rate limiters per RPC
    rate_limiters: HashMap<String, RpcRateLimiter>,
    /// RPC statistics
    stats: RpcStats,
}

impl RpcManager {
    /// Create a new RPC manager
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
            pending_calls: Vec::new(),
            pending_responses: Vec::new(),
            next_rpc_id: 1,
            rate_limiters: HashMap::new(),
            stats: RpcStats::default(),
        }
    }

    /// Register an RPC handler
    pub fn register_handler<F>(&mut self, name: &str, handler: F)
    where
        F: Fn(&[u8]) -> Result<Vec<u8>, RpcError> + Send + Sync + 'static,
    {
        self.handlers.insert(name.to_string(), Box::new(handler));
    }

    /// Set rate limit for an RPC
    pub fn set_rate_limit(&mut self, name: &str, max_calls_per_second: u32) {
        self.rate_limiters
            .insert(name.to_string(), RpcRateLimiter::new(max_calls_per_second));
    }

    /// Call an RPC
    pub fn call_rpc(
        &mut self,
        name: &str,
        args: Vec<u8>,
        target: RpcTarget,
    ) -> Result<RpcId, RpcError> {
        // Check rate limit
        if let Some(limiter) = self.rate_limiters.get_mut(name) {
            if !limiter.allow_call() {
                self.stats.rate_limit_hits += 1;
                return Err(RpcError::RateLimitExceeded(format!(
                    "Rate limit exceeded for RPC: {}",
                    name
                )));
            }
        }

        let rpc_id = self.next_rpc_id;
        self.next_rpc_id += 1;

        let mut call = RpcCall::new(name.to_string(), args, target);
        call.id = rpc_id;

        self.pending_calls.push(call);
        self.stats.rpcs_sent += 1;

        Ok(rpc_id)
    }

    /// Call a reliable RPC
    pub fn call_rpc_reliable(
        &mut self,
        name: &str,
        args: Vec<u8>,
        target: RpcTarget,
    ) -> Result<RpcId, RpcError> {
        let rpc_id = self.call_rpc(name, args, target)?;
        if let Some(call) = self.pending_calls.iter_mut().find(|c| c.id == rpc_id) {
            call.reliability = RpcReliability::Reliable;
        }
        Ok(rpc_id)
    }

    /// Call an unreliable RPC
    pub fn call_rpc_unreliable(
        &mut self,
        name: &str,
        args: Vec<u8>,
        target: RpcTarget,
    ) -> Result<RpcId, RpcError> {
        let rpc_id = self.call_rpc(name, args, target)?;
        if let Some(call) = self.pending_calls.iter_mut().find(|c| c.id == rpc_id) {
            call.reliability = RpcReliability::Unreliable;
        }
        Ok(rpc_id)
    }

    /// Handle an incoming RPC call
    pub fn handle_rpc(&mut self, call: RpcCall) -> RpcResponse {
        self.stats.rpcs_received += 1;

        // Find handler
        let handler = match self.handlers.get(&call.name) {
            Some(h) => h,
            None => {
                self.stats.rpc_errors += 1;
                return RpcResponse::error(
                    call.id,
                    format!("Handler not found: {}", call.name),
                );
            }
        };

        // Execute handler
        match handler(&call.args) {
            Ok(data) => RpcResponse::success(call.id, data),
            Err(e) => {
                self.stats.rpc_errors += 1;
                RpcResponse::error(call.id, format!("{:?}", e))
            }
        }
    }

    /// Get pending RPC calls
    pub fn get_pending_calls(&mut self) -> Vec<RpcCall> {
        let calls = self.pending_calls.clone();
        self.pending_calls.clear();
        calls
    }

    /// Get pending RPC responses
    pub fn get_pending_responses(&mut self) -> Vec<RpcResponse> {
        let responses = self.pending_responses.clone();
        self.pending_responses.clear();
        responses
    }

    /// Add a response
    pub fn add_response(&mut self, response: RpcResponse) {
        self.pending_responses.push(response);
    }

    /// Get RPC statistics
    pub fn get_stats(&self) -> RpcStats {
        self.stats.clone()
    }

    /// Get number of registered handlers
    pub fn handler_count(&self) -> usize {
        self.handlers.len()
    }
}

impl Default for RpcManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Get current timestamp in milliseconds
fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rpc_call_creation() {
        let call = RpcCall::new("test".to_string(), vec![1, 2, 3], RpcTarget::Server);
        assert_eq!(call.name, "test");
        assert_eq!(call.args, vec![1, 2, 3]);
        assert_eq!(call.target, RpcTarget::Server);
    }

    #[test]
    fn test_rpc_call_reliable() {
        let call = RpcCall::reliable("test".to_string(), vec![1, 2, 3], RpcTarget::Server);
        assert_eq!(call.reliability, RpcReliability::Reliable);
    }

    #[test]
    fn test_rpc_call_unreliable() {
        let call = RpcCall::unreliable("test".to_string(), vec![1, 2, 3], RpcTarget::Server);
        assert_eq!(call.reliability, RpcReliability::Unreliable);
    }

    #[test]
    fn test_rpc_call_serialization() {
        let call = RpcCall::new("test".to_string(), vec![1, 2, 3], RpcTarget::Server);
        let bytes = call.to_bytes().unwrap();
        let deserialized = RpcCall::from_bytes(&bytes).unwrap();

        assert_eq!(call.name, deserialized.name);
        assert_eq!(call.args, deserialized.args);
    }

    #[test]
    fn test_rpc_response_success() {
        let response = RpcResponse::success(1, vec![4, 5, 6]);
        assert_eq!(response.rpc_id, 1);
        assert_eq!(response.data, vec![4, 5, 6]);
        assert!(response.success);
        assert!(response.error.is_none());
    }

    #[test]
    fn test_rpc_response_error() {
        let response = RpcResponse::error(1, "Test error".to_string());
        assert_eq!(response.rpc_id, 1);
        assert!(!response.success);
        assert_eq!(response.error, Some("Test error".to_string()));
    }

    #[test]
    fn test_rpc_response_serialization() {
        let response = RpcResponse::success(1, vec![1, 2, 3]);
        let bytes = response.to_bytes().unwrap();
        let deserialized = RpcResponse::from_bytes(&bytes).unwrap();

        assert_eq!(response.rpc_id, deserialized.rpc_id);
        assert_eq!(response.data, deserialized.data);
        assert_eq!(response.success, deserialized.success);
    }

    #[test]
    fn test_rpc_manager_creation() {
        let manager = RpcManager::new();
        assert_eq!(manager.handler_count(), 0);
    }

    #[test]
    fn test_rpc_manager_register_handler() {
        let mut manager = RpcManager::new();
        manager.register_handler("test", |_args| Ok(vec![1, 2, 3]));

        assert_eq!(manager.handler_count(), 1);
    }

    #[test]
    fn test_rpc_manager_call_rpc() {
        let mut manager = RpcManager::new();
        let rpc_id = manager
            .call_rpc("test", vec![1, 2, 3], RpcTarget::Server)
            .unwrap();

        assert_eq!(rpc_id, 1);
        assert_eq!(manager.get_stats().rpcs_sent, 1);
    }

    #[test]
    fn test_rpc_manager_handle_rpc() {
        let mut manager = RpcManager::new();
        manager.register_handler("test", |args| {
            assert_eq!(args, &[1, 2, 3]);
            Ok(vec![4, 5, 6])
        });

        let call = RpcCall::new("test".to_string(), vec![1, 2, 3], RpcTarget::Server);
        let response = manager.handle_rpc(call);

        assert!(response.success);
        assert_eq!(response.data, vec![4, 5, 6]);
    }

    #[test]
    fn test_rpc_manager_handle_rpc_not_found() {
        let mut manager = RpcManager::new();
        let call = RpcCall::new("nonexistent".to_string(), vec![1, 2, 3], RpcTarget::Server);
        let response = manager.handle_rpc(call);

        assert!(!response.success);
        assert!(response.error.is_some());
    }

    #[test]
    fn test_rpc_manager_rate_limit() {
        let mut manager = RpcManager::new();
        manager.set_rate_limit("test", 2); // 2 calls per second

        // First two calls should succeed
        assert!(manager
            .call_rpc("test", vec![1], RpcTarget::Server)
            .is_ok());
        assert!(manager
            .call_rpc("test", vec![2], RpcTarget::Server)
            .is_ok());

        // Third call should fail
        let result = manager.call_rpc("test", vec![3], RpcTarget::Server);
        assert!(result.is_err());
        assert_eq!(manager.get_stats().rate_limit_hits, 1);
    }

    #[test]
    fn test_rpc_manager_pending_calls() {
        let mut manager = RpcManager::new();
        manager
            .call_rpc("test1", vec![1], RpcTarget::Server)
            .unwrap();
        manager
            .call_rpc("test2", vec![2], RpcTarget::Server)
            .unwrap();

        let pending = manager.get_pending_calls();
        assert_eq!(pending.len(), 2);
        assert_eq!(pending[0].name, "test1");
        assert_eq!(pending[1].name, "test2");

        // Should be cleared after getting
        let pending2 = manager.get_pending_calls();
        assert_eq!(pending2.len(), 0);
    }

    #[test]
    fn test_rpc_manager_pending_responses() {
        let mut manager = RpcManager::new();
        manager.add_response(RpcResponse::success(1, vec![1, 2, 3]));
        manager.add_response(RpcResponse::success(2, vec![4, 5, 6]));

        let pending = manager.get_pending_responses();
        assert_eq!(pending.len(), 2);

        // Should be cleared after getting
        let pending2 = manager.get_pending_responses();
        assert_eq!(pending2.len(), 0);
    }

    #[test]
    fn test_rpc_target_types() {
        assert_eq!(RpcTarget::Server, RpcTarget::Server);
        assert_eq!(RpcTarget::Client(1), RpcTarget::Client(1));
        assert_eq!(RpcTarget::AllClients, RpcTarget::AllClients);
        assert_eq!(
            RpcTarget::AllClientsExcept(1),
            RpcTarget::AllClientsExcept(1)
        );
    }

    #[test]
    fn test_rpc_reliability_types() {
        assert_eq!(RpcReliability::Reliable, RpcReliability::Reliable);
        assert_eq!(RpcReliability::Unreliable, RpcReliability::Unreliable);
    }

    #[test]
    fn test_rpc_stats() {
        let mut manager = RpcManager::new();
        manager.register_handler("test", |_| Ok(vec![]));

        manager.call_rpc("test", vec![], RpcTarget::Server).unwrap();
        let call = RpcCall::new("test".to_string(), vec![], RpcTarget::Server);
        manager.handle_rpc(call);

        let stats = manager.get_stats();
        assert_eq!(stats.rpcs_sent, 1);
        assert_eq!(stats.rpcs_received, 1);
    }

    #[test]
    fn test_rpc_call_to_network_message() {
        let call = RpcCall::reliable("test".to_string(), vec![1, 2, 3], RpcTarget::Server);
        let msg = call.to_network_message().unwrap();

        assert_eq!(msg.channel, NetworkChannel::Reliable);
    }

    #[test]
    fn test_rpc_manager_call_reliable() {
        let mut manager = RpcManager::new();
        let rpc_id = manager
            .call_rpc_reliable("test", vec![1, 2, 3], RpcTarget::Server)
            .unwrap();

        let pending = manager.get_pending_calls();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].reliability, RpcReliability::Reliable);
    }

    #[test]
    fn test_rpc_manager_call_unreliable() {
        let mut manager = RpcManager::new();
        let rpc_id = manager
            .call_rpc_unreliable("test", vec![1, 2, 3], RpcTarget::Server)
            .unwrap();

        let pending = manager.get_pending_calls();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].reliability, RpcReliability::Unreliable);
    }
}

