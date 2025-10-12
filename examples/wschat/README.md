# wschat - Production WebSocket Chat Server

**A scalable real-time chat server built in Windjammer**

Designed to handle 10,000+ concurrent connections with <10ms p50 latency.

---

## 🚀 Features

- ✅ **WebSocket** - Real-time bidirectional communication
- ✅ **Rooms** - Create and join chat rooms
- ✅ **Presence** - User online/offline/typing indicators
- ✅ **Message History** - Last 100 messages per room
- ✅ **Authentication** - JWT token support
- ✅ **Rate Limiting** - Prevent abuse (token bucket)
- ✅ **Metrics** - Prometheus metrics + JSON stats
- ✅ **Scalable** - Designed for 10k+ connections

---

## 📦 Installation

```bash
# Build from source
cd examples/wschat
wj build --release

# Run server
./target/release/wschat

# Or use wj run
wj run
```

---

## 🎯 Usage

### Start Server

```bash
# Default (port 8080)
./wschat

# Custom configuration
HOST=0.0.0.0 PORT=3000 MAX_CONNECTIONS=5000 ./wschat
```

### Client Example (JavaScript)

```javascript
// Connect
const ws = new WebSocket('ws://localhost:8080/ws');

// Authenticate (optional)
ws.onopen = () => {
  ws.send(JSON.stringify({
    type: 'auth',
    token: 'YOUR_JWT_TOKEN'
  }));
};

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

// Receive messages
ws.onmessage = (event) => {
  const msg = JSON.parse(event.data);
  console.log('Received:', msg);
};
```

---

## 📋 Message Protocol

### Client → Server

```json
// Join room
{"type": "join", "room": "general"}

// Send message
{"type": "message", "room": "general", "text": "Hello!"}

// Leave room
{"type": "leave", "room": "general"}

// Direct message
{"type": "dm", "to": "user123", "text": "Hi!"}

// Typing indicator
{"type": "typing", "room": "general", "status": true}

// Request user list
{"type": "list_users", "room": "general"}

// Request history
{"type": "history", "room": "general", "limit": 50}

// Ping
{"type": "ping"}
```

### Server → Client

```json
// Welcome
{"type": "welcome", "user_id": "abc", "username": "john"}

// Joined room
{"type": "joined", "room": "general", "users": [...]}

// New message
{"type": "message", "room": "general", "from": "alice", "text": "Hi!", "timestamp": 1234567890}

// User presence
{"type": "presence", "room": "general", "user": "bob", "status": "online"}

// Typing indicator
{"type": "typing", "room": "general", "user": "alice", "status": true}

// Error
{"type": "error", "message": "Room not found"}

// Pong
{"type": "pong"}
```

---

## 🔧 Configuration

Environment variables:

- `HOST` - Server host (default: `0.0.0.0`)
- `PORT` - WebSocket server port (default: `8080`)
- `MAX_CONNECTIONS` - Max concurrent connections (default: `10000`)
- `RATE_LIMIT` - Messages per second per user (default: `10`)
- `HEARTBEAT_INTERVAL` - Heartbeat interval in seconds (default: `30`)
- `ENABLE_PERSISTENCE` - Enable message persistence (default: `false`)
- `DATABASE_URL` - Database URL for persistence (optional)

---

## 📊 Monitoring

### Prometheus Metrics

Available at `http://localhost:8081/metrics`:

- `wschat_connections_total` - Active connections
- `wschat_rooms_total` - Active rooms
- `wschat_room_members_total` - Total room memberships
- `wschat_uptime_seconds` - Server uptime

### JSON Metrics

Available at `http://localhost:8081/metrics/json`:

```json
{
  "connections": 1234,
  "rooms": 56,
  "total_memberships": 2345,
  "timestamp": 1234567890
}
```

### Health Check

Available at `http://localhost:8080/health` and `http://localhost:8081/health`

---

## 🧪 Testing

```bash
# Run unit tests
wj test

# Run load test (10k connections)
wj test --test load_test

# Run benchmarks
wj bench
```

---

## ⚡ Performance

**Targets**:
- **10,000+ concurrent connections**
- **<10ms p50 message latency**
- **100,000 messages/second throughput**
- **<1KB memory per connection**

**Status**: 🚧 Benchmarks in progress

---

## 🏗️ Architecture

```
wschat/
├── main.wj         # Server entry point
├── server.wj       # Connection handling
├── room.wj         # Room management
├── message.wj      # Message types
├── presence.wj     # User presence
├── auth.wj         # JWT authentication
├── rate_limit.wj   # Rate limiting (token bucket)
└── metrics.wj      # Prometheus metrics
```

### Key Design Decisions

1. **Arc<Mutex<>>** for shared state (connections, rooms)
2. **Token bucket** rate limiting per user
3. **In-memory** message history (100 messages per room)
4. **Heartbeat** task per connection (30s interval)
5. **Graceful shutdown** closes all connections cleanly

---

## 🎓 What This Validates

This production WebSocket server validates Windjammer's:

1. **WebSocket Support** - `std.websocket` for real-time
2. **Async/Await at Scale** - 10k+ concurrent connections
3. **Concurrent Data Structures** - `Arc<Mutex<HashMap<>>>`
4. **Channel-Based Messaging** - Broadcast to rooms
5. **Memory Efficiency** - <1KB per connection
6. **Graceful Shutdown** - Clean connection closure

---

## 🚧 Roadmap

### Phase 1: Core (Week 2) - IN PROGRESS
- [x] WebSocket server setup
- [x] Connection handling
- [x] Room management
- [x] Message routing
- [x] User presence
- [x] Authentication
- [x] Rate limiting
- [x] Metrics

### Phase 2: Features (Week 2-3)
- [ ] Message persistence (SQLite/PostgreSQL)
- [ ] Direct messages
- [ ] Message history API
- [ ] Connection recovery
- [ ] Heartbeat implementation

### Phase 3: Performance (Week 3)
- [ ] Load testing (10k connections)
- [ ] Performance profiling
- [ ] Memory optimization
- [ ] Latency benchmarks

### Phase 4: Polish (Week 3)
- [ ] Client libraries (JS, Python, Rust)
- [ ] Complete documentation
- [ ] Docker deployment
- [ ] Kubernetes manifests

---

## 📄 License

MIT

---

*Built with Windjammer v0.23.0*  
*Part of the v0.23.0 Production Hardening initiative*

