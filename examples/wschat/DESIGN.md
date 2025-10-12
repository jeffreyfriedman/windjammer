# wschat - Production WebSocket Chat Server

**A scalable real-time chat server built in Windjammer**

---

## 🎯 Goals

1. **Validate Windjammer WebSocket capabilities** - Real-time bidirectional communication
2. **Handle 10,000+ concurrent connections** - Test scalability and performance
3. **Showcase async/await** - Production async patterns
4. **Production-ready features** - Rooms, presence, message history, authentication

---

## 📋 Features

### Core Chat
- [x] WebSocket connection management
- [x] Room creation and joining
- [x] Message broadcasting
- [x] Direct messages (1-to-1)
- [x] User presence (online/offline/typing)
- [x] Message history (last 100 messages per room)
- [x] User list per room

### Advanced
- [x] Authentication (JWT or session token)
- [x] Rate limiting (messages per second)
- [x] Message persistence (optional SQLite/PostgreSQL)
- [x] Connection recovery (automatic reconnect)
- [x] Heartbeat/ping-pong (keep-alive)
- [x] Graceful shutdown (close all connections)

### Monitoring
- [x] Prometheus metrics (connections, messages, rooms)
- [x] Health checks
- [x] Performance profiling
- [x] Connection statistics

---

## 🏗️ Architecture

```
wschat/
├── src/
│   ├── main.wj              # Server entry point
│   ├── server.wj            # WebSocket server
│   ├── connection.wj        # Connection handler
│   ├── room.wj              # Room management
│   ├── message.wj           # Message types
│   ├── presence.wj          # User presence tracking
│   ├── auth.wj              # Authentication
│   ├── rate_limit.wj        # Rate limiting
│   ├── storage.wj           # Message persistence
│   └── metrics.wj           # Prometheus metrics
├── tests/
│   ├── load_test.wj         # Load testing (10k connections)
│   └── integration_test.wj  # Integration tests
├── benches/
│   └── throughput_bench.wj  # Message throughput benchmarks
└── README.md
```

---

## 🚀 Usage Examples

### Client Connection
```javascript
// Connect to server
const ws = new WebSocket('ws://localhost:8080/ws?token=YOUR_JWT');

// Authenticate
ws.send(JSON.stringify({
  type: 'auth',
  token: 'YOUR_JWT'
}));

// Join a room
ws.send(JSON.stringify({
  type: 'join',
  room: 'general'
}));

// Send message
ws.send(JSON.stringify({
  type: 'message',
  room: 'general',
  text: 'Hello, world!'
}));

// Listen for messages
ws.onmessage = (event) => {
  const msg = JSON.parse(event.data);
  console.log(msg);
};
```

### Message Types

#### Client → Server
```json
// Join room
{"type": "join", "room": "general"}

// Leave room
{"type": "leave", "room": "general"}

// Send message
{"type": "message", "room": "general", "text": "Hello!"}

// Direct message
{"type": "dm", "to": "user123", "text": "Hi there!"}

// Typing indicator
{"type": "typing", "room": "general", "status": true}

// Request user list
{"type": "list_users", "room": "general"}

// Request message history
{"type": "history", "room": "general", "limit": 50}
```

#### Server → Client
```json
// Welcome message
{"type": "welcome", "user_id": "abc123", "username": "john"}

// Room joined
{"type": "joined", "room": "general", "users": [...]}

// New message
{"type": "message", "room": "general", "from": "alice", "text": "Hi!", "timestamp": 1234567890}

// User joined/left
{"type": "presence", "room": "general", "user": "bob", "status": "online"}

// Typing indicator
{"type": "typing", "room": "general", "user": "alice", "status": true}

// Error
{"type": "error", "message": "Room not found"}

// Pong (heartbeat response)
{"type": "pong"}
```

---

## ⚡ Performance Targets

**Goal**: Handle 10,000+ concurrent connections with low latency

| Metric | Target | Notes |
|--------|--------|-------|
| **Concurrent Connections** | 10,000+ | Tested with load generator |
| **Message Latency (p50)** | < 10ms | Median message delivery time |
| **Message Latency (p99)** | < 50ms | 99th percentile |
| **Throughput** | 100,000 msg/s | Total server capacity |
| **Memory per Connection** | < 1KB | Efficient connection state |
| **CPU per 1000 Connections** | < 5% | On 4-core system |

---

## 🧪 Testing Strategy

### Unit Tests
- Connection lifecycle (connect, auth, disconnect)
- Room management (create, join, leave)
- Message routing
- Rate limiting
- Presence tracking

### Integration Tests
- End-to-end message flow
- Multi-room scenarios
- Authentication failures
- Graceful shutdown

### Load Tests
- 10,000 concurrent connections
- 100,000 messages per second
- Memory leak detection
- Connection churn (rapid connect/disconnect)

### Benchmarks
- Message throughput
- Latency distribution
- Memory usage
- CPU usage

---

## 📦 Dependencies (via Windjammer stdlib)

```windjammer
use std.websocket   // WebSocket server
use std.http        // HTTP server for WS upgrade
use std.json        // Message serialization
use std.time        // Timestamps
use std.sync        // Concurrent data structures
use std.collections // HashMap, HashSet
use std.db          // Optional message persistence
use std.log         // Logging
```

---

## 🎨 Server State Management

### Global State
```windjammer
struct ServerState {
    connections: Arc<Mutex<HashMap<UserId, Connection>>>,
    rooms: Arc<Mutex<HashMap<RoomId, Room>>>,
    presence: Arc<Mutex<HashMap<UserId, PresenceInfo>>>,
    metrics: Arc<Metrics>,
}
```

### Connection State
```windjammer
struct Connection {
    id: string,
    user_id: string,
    username: string,
    socket: WebSocket,
    rooms: HashSet<string>,
    last_activity: Instant,
    authenticated: bool,
}
```

### Room State
```windjammer
struct Room {
    id: string,
    name: string,
    members: HashSet<string>,
    message_history: Vec<Message>,
    created_at: Instant,
}
```

---

## 🔧 Implementation Plan

### Phase 1: Core (Week 2)
- [x] WebSocket server setup
- [x] Connection handling
- [x] Basic message routing
- [x] Room management
- [ ] User presence

### Phase 2: Features (Week 2-3)
- [ ] Authentication
- [ ] Rate limiting
- [ ] Message history
- [ ] Direct messages
- [ ] Graceful shutdown

### Phase 3: Performance (Week 3)
- [ ] Load testing (10k connections)
- [ ] Performance profiling
- [ ] Memory optimization
- [ ] Benchmark against Go/Rust

### Phase 4: Polish (Week 3)
- [ ] Prometheus metrics
- [ ] Health checks
- [ ] Documentation
- [ ] Client libraries

---

## 📊 Success Metrics

1. **Performance**: Handle 10,000+ concurrent connections with <10ms p50 latency
2. **Reliability**: Zero message loss under normal conditions
3. **Scalability**: Linear scaling with CPU cores
4. **Usability**: Simple client API, easy to integrate

---

## 🎯 Learnings for Windjammer

This project will validate:
- ✅ WebSocket support (`std.websocket`)
- ✅ Async/await at scale
- ✅ Concurrent data structures (`std.sync`)
- ✅ Channel-based message passing
- ✅ Memory efficiency under load
- ✅ Graceful shutdown patterns
- ✅ Real-time performance

---

## 🔥 Comparison to Other Servers

| Server | Language | Connections | Latency (p50) | Notes |
|--------|----------|-------------|---------------|-------|
| **Go WebSocket** | Go | 10,000+ | ~5-10ms | Excellent baseline |
| **Rust Tokio** | Rust | 10,000+ | ~3-8ms | Best-in-class |
| **Node.js Socket.io** | JavaScript | ~5,000 | ~10-20ms | GC overhead |
| **wschat** | **Windjammer** | **10,000+** | **~8-12ms** | **Target** |

**Goal**: Match Go, approach Rust performance

---

*Design created: October 12, 2025*  
*Target: 2 weeks for production-ready v1.0*

