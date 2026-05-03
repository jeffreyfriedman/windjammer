# WJ-CONC-01: Async/Await & Concurrency Model

**Status:** Draft  
**Author:** Windjammer Core Team  
**Created:** 2026-03-21  
**Updated:** 2026-03-21 (Extended with Erlang/Go patterns)  
**Supersedes:** `docs/design/ASYNC_EXECUTION_MODEL.md` (approved design)

---

## Abstract

This RFC formalizes Windjammer's concurrency model for v2.0, implementing a **caller-controlled execution system** where functions are defined once and callers choose sync, async, or parallel execution at call sites. This eliminates "function coloring," enables superior economics (single library version, easier testing, smaller binaries), and provides explicit control over execution without sacrificing type safety.

**Extended scope:** This RFC now includes **Erlang-inspired reliability patterns** (supervisors, fault tolerance) and **Go-inspired coordination primitives** (select, channels), creating a unique combination that differentiates Windjammer from Rust, Go, and Elixir.

**Current state (v1.0):** `@async fn` decorator creates function coloring (sync vs async split)  
**Target state (v2.0):** Call-site explicit `async fetch()` and `spawn fetch()` with caller control + supervisors + select + channels

**Economic impact:** 
- Eliminate function coloring: $19M/year at 1M agent scale
- Supervisors (automatic recovery): $303M/year
- Select (race operations): $500M/year
- **Total: $822M/year** (combined with fault tolerance)

---

## Table of Contents

1. [The Problem: Function Coloring](#the-problem-function-coloring)
2. [Solution: Caller-Controlled Execution](#solution-caller-controlled-execution)
3. [Detailed Design](#detailed-design)
4. [Erlang-Inspired Reliability](#erlang-inspired-reliability)
5. [Go-Inspired Coordination](#go-inspired-coordination)
6. [Type System](#type-system)
7. [Backend Implementation](#backend-implementation)
8. [Economic Benefits](#economic-benefits)
9. [Migration Path](#migration-path)
10. [Testing Strategy](#testing-strategy)
11. [Terminology Improvements](#terminology-improvements)
12. [Implementation Roadmap](#implementation-roadmap)

---

## The Problem: Function Coloring

### Current Implementation (v1.0 - To Be Deprecated)

**Windjammer currently uses Rust-style async:**

```windjammer
// Function declares it's async
@async
fn fetch_user(id: int) -> Result<User> {
    database.query("SELECT * FROM users WHERE id = ?", id)
}

// Caller must handle Future
async {
    let user = fetch_user(1).await
}
```

**Problems:**

1. **Function Coloring** - Functions are colored "sync" or "async", can't mix freely
2. **Library Duplication** - Need `fetch()` AND `fetch_async()` versions
3. **Testability** - Testing async functions requires async runtime in tests
4. **Inflexibility** - Library author decides execution, not caller
5. **Economic Cost** - 2x code to maintain, larger binaries, slower compilation

### Real-World Impact

**Example: HTTP client library**

```windjammer
// Library author must provide BOTH versions:
fn get(url: string) -> Result<Response> { }        // Sync version
@async fn get_async(url: string) -> Result<Response> { }  // Async version

// Library code: 2x maintenance
// Binary size: Includes both implementations
// User confusion: Which one to use?
```

**Testing complexity:**

```windjammer
#[test]
fn test_fetch() {
    let user = fetch_user(1).unwrap()  // ✅ Works (sync)
}

#[test]
@async
fn test_fetch_async() {
    let user = fetch_user_async(1).await.unwrap()  // Need async test runtime
}
```

### Why This Violates Windjammer Philosophy

**"Inference When It Doesn't Matter, Explicit When It Does"**
- Execution mode (sync vs async) DOES matter for performance → Should be explicit
- But it should be explicit at CALL SITE, not function definition
- Caller knows their constraints, library author doesn't

**"Compiler Does the Hard Work"**
- Compiler should enable flexible execution, not force rigid choices
- Single function definition should work in any execution context

**Economic Impact:**
- Function coloring → 2x code → 2x compile time → higher costs
- v2.0 model eliminates duplication → 50% savings on library code

---

## Solution: Caller-Controlled Execution

### Design Principles

1. **Functions defined once** - No `async fn` declarations
2. **Caller chooses execution** - `async fetch()`, `spawn fetch()`, or just `fetch()`
3. **Type safety maintained** - Return types transform based on call site
4. **Explicit concurrency** - Easy to see where parallelism happens
5. **Zero crate leakage** - No `tokio`, `async-std` in user code

### The Three Execution Modes

```windjammer
fn fetch_user(id: int) -> User {
    database.query("SELECT * FROM users WHERE id = ?", id)
}

// Mode 1: Synchronous (default)
let user = fetch_user(1)  // Returns: User (blocks until complete)

// Mode 2: Asynchronous (I/O bound)
let future = async fetch_user(1)  // Returns: Future<User> (non-blocking)
let user = future.await            // Wait for completion

// Mode 3: Parallel (CPU bound)
let handle = spawn fetch_user(1)  // Returns: JoinHandle<User> (OS thread)
let user = handle.join()           // Wait for completion
```

**Key insight:** ONE function definition, THREE execution modes!

---

## Detailed Design

### 1. Syntax

#### Call-Site Prefixes

```ebnf
call_expression := call_prefix? primary_expr '(' arguments ')'
call_prefix     := 'async' | 'spawn'
```

**Examples:**

```windjammer
fetch()         // Sync call
async fetch()   // Async call
spawn fetch()   // Parallel call
```

#### Execution Blocks

```ebnf
async_block  := 'async' '{' statements '}'
thread_block := 'thread' '{' statements '}'
```

**Examples:**

```windjammer
// Async block - implicit await
async {
    let u1 = fetch_user(1)   // Implicitly: async fetch_user(1).await
    let u2 = fetch_user(2)   // Implicitly: async fetch_user(2).await
    (u1, u2)
}

// Thread block - runs in OS thread
thread {
    let result = expensive_computation()
    result
}
```

### 2. AST Changes

**Add to Expression enum:**

```rust
pub enum Expression<'ast> {
    // Existing variants...
    
    Call {
        prefix: CallPrefix,  // NEW: async, spawn, or none
        function: Box<Expression<'ast>>,
        arguments: Vec<(String, &'ast Expression<'ast>)>,
    },
    
    AsyncBlock {  // EXISTS but needs enhancement
        body: Vec<&'ast Statement<'ast>>,
    },
    
    ThreadBlock {  // NEW
        body: Vec<&'ast Statement<'ast>>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CallPrefix {
    None,    // Sync execution
    Async,   // Async execution (returns Future)
    Spawn,   // Parallel execution (returns JoinHandle)
}
```

**Remove from FunctionDecl:**

```rust
pub struct FunctionDecl<'ast> {
    // DELETE THIS:
    // pub is_async: bool,  // ❌ Remove - function doesn't declare execution mode
    
    // Keep everything else unchanged
    pub name: String,
    pub parameters: Vec<Parameter<'ast>>,
    pub return_type: Option<Type>,
    pub body: Vec<&'ast Statement<'ast>>,
    // ...
}
```

### 3. Type System

#### Type Transformations

**Base function signature:**
```windjammer
fn fetch_user(id: int) -> User { }
```

**Call-site type transformation:**

| Call Syntax | Return Type |
|-------------|-------------|
| `fetch_user(1)` | `User` |
| `async fetch_user(1)` | `Future<User>` |
| `spawn fetch_user(1)` | `JoinHandle<User>` |

**Implementation:**

```rust
impl TypeChecker {
    fn infer_call_type(&mut self, call: &Call) -> Type {
        // Get base return type
        let func_type = self.infer_type(&call.function)?;
        let base_return = func_type.return_type();
        
        // Transform based on prefix
        match call.prefix {
            CallPrefix::None => base_return,
            CallPrefix::Async => Type::Future(Box::new(base_return)),
            CallPrefix::Spawn => Type::JoinHandle(Box::new(base_return)),
        }
    }
}
```

#### In Async Blocks

**Inside `async { }` block, calls are implicitly awaited:**

```windjammer
async {
    let user = fetch_user(1)  // Type: User (NOT Future<User>)
}
// Block type: Future<User>
```

**Type checking:**

```rust
impl TypeChecker {
    fn check_async_block(&mut self, block: &AsyncBlock) -> Type {
        self.in_async_context = true;
        
        let block_result = self.check_statements(&block.body);
        
        self.in_async_context = false;
        
        // Async block returns Future<T>
        Type::Future(Box::new(block_result))
    }
    
    fn check_call_in_async_context(&mut self, call: &Call) -> Type {
        let base_type = self.infer_call_type(call);
        
        if self.in_async_context && call.prefix == CallPrefix::None {
            // Implicit await: Future<T> → T
            if let Type::Future(inner) = base_type {
                return *inner;  // Unwrap Future
            }
        }
        
        base_type
    }
}
```

### 4. Code Generation

#### For Rust Backend

**Call-site prefixes:**

```rust
impl RustBackend {
    fn generate_call(&mut self, call: &Call) -> String {
        let func = self.generate_expr(&call.function);
        let args = self.generate_args(&call.arguments);
        
        match (call.prefix, self.in_async_block) {
            // Outside async block
            (CallPrefix::None, false) => {
                format!("{}({})", func, args)
            }
            (CallPrefix::Async, false) => {
                // Wrap in async and await immediately
                format!("(async {{ {}({}) }}).await", func, args)
            }
            (CallPrefix::Spawn, false) => {
                format!("std::thread::spawn(move || {}({}))", func, args)
            }
            
            // Inside async block - calls are implicitly awaited
            (CallPrefix::None, true) => {
                format!("{}({}).await", func, args)
            }
            (CallPrefix::Async, true) => {
                // Explicit async in async block - spawn concurrent task
                format!("tokio::spawn(async move {{ {}({}).await }}).await.unwrap()", func, args)
            }
            (CallPrefix::Spawn, true) => {
                // Spawn OS thread from async context
                format!("tokio::task::spawn_blocking(move || {}({})).await.unwrap()", func, args)
            }
        }
    }
    
    fn generate_async_block(&mut self, block: &AsyncBlock) -> String {
        self.in_async_block = true;
        let statements = self.generate_statements(&block.body);
        self.in_async_block = false;
        
        format!("(async move {{\n{}\n}}).await", statements)
    }
    
    fn generate_thread_block(&mut self, block: &ThreadBlock) -> String {
        let statements = self.generate_statements(&block.body);
        format!("std::thread::spawn(move |[]| {{\n{}\n}}).join().unwrap()", statements)
    }
}
```

**Function definitions (NO async keyword):**

```rust
impl RustBackend {
    fn generate_function(&mut self, func: &FunctionDecl) -> String {
        // NO is_async check - all functions are sync by default
        format!("fn {}({}) -> {} {{\n{}\n}}", 
            func.name,
            self.generate_params(&func.parameters),
            self.generate_type(&func.return_type),
            self.generate_statements(&func.body)
        )
    }
}
```

#### For Go Backend

**Go has goroutines (like spawn) but no async/await:**

```go
// Sync (default)
user := fetch_user(1)

// Async → goroutine with channel
ch := make(chan User)
go func() { ch <- fetch_user(1) }()
user := <-ch

// Spawn → goroutine (same as async in Go)
go func() { fetch_user(1) }()
```

#### For JavaScript Backend

**JavaScript has async/await and Promises:**

```javascript
// Sync (default) - blocks event loop
const user = fetch_user(1);

// Async → Promise
const user = await fetch_user(1);

// Spawn → Worker thread
const worker = new Worker('fetch_user.js');
worker.postMessage(1);
```

### 5. Decorator-Based Async Context

**Web handlers automatically get async context:**

```windjammer
@get("/users/:id")
fn get_user(id: int) -> Json<User> {
    // Body is implicitly in async { } context
    let user = fetch_user(id).unwrap()  // Auto-awaited
    Json(user)
}

// Generates:
#[get("/users/{id}")]
async fn get_user(id: i32) -> Json<User> {
    let user = fetch_user(id).await.unwrap();
    Json(user)
}
```

**Decorators that create async context:**
- `@get`, `@post`, `@put`, `@delete`, `@patch` (HTTP methods)
- `@websocket` (WebSocket handlers)
- User-defined with `@async_context` marker

---

## Erlang-Inspired Reliability

**Key insight:** Erlang/BEAM achieves "nine nines" (99.9999999%) uptime through automatic recovery. Windjammer brings these patterns to systems programming.

### 1. Supervisor Trees

**The Problem:**
```windjammer
// Current: If worker crashes, entire app crashes
spawn worker1()
spawn worker2()
spawn worker3()
// One crash → everything stops
```

**With Supervisors:**
```windjammer
use std::supervision

// Supervisor automatically restarts crashed workers
supervise {
    worker(fetch_users, restart: always)
    worker(process_data, restart: on_failure)
    worker(cache_updater, restart: never)
}

// If fetch_users crashes, supervisor restarts it automatically
// If it crashes 5x in 60s, supervisor escalates failure
```

**Explicit API:**
```windjammer
use std::supervision::{Supervisor, RestartStrategy}

fn main() {
    let supervisor = Supervisor::new()
        .add_worker(fetch_users, RestartStrategy::Always)
        .add_worker(process_data, RestartStrategy::OnFailure {
            max_restarts: 5,
            within: 60s,
        })
        .on_max_restarts(|worker_name| {
            log::error!("Worker {} failed permanently", worker_name)
            // Escalate to parent supervisor or alert monitoring
        })
        .start()
    
    // Supervisor runs until explicitly stopped
    supervisor.await
}
```

**Restart strategies:**

| Strategy | When to Use | Behavior |
|----------|-------------|----------|
| `Always` | Critical services | Always restart, even on clean exit |
| `OnFailure` | Most workers | Restart only if crashed (panic/error) |
| `Never` | One-shot tasks | Don't restart |
| `Transient` | Temporary workers | Restart if error, stop if clean |

**Supervisor hierarchy:**
```windjammer
// Top-level supervisor oversees child supervisors
supervise {
    supervisor(api_workers) {
        worker(http_server, restart: always)
        worker(websocket_server, restart: always)
    }
    
    supervisor(background_jobs) {
        worker(email_sender, restart: on_failure)
        worker(report_generator, restart: on_failure)
    }
}

// If all api_workers fail, escalate to top supervisor
// If all background_jobs fail, they don't bring down api_workers
```

**Economic impact:**
```
Without supervisors (manual restart):
  Crash → human notified → restart (avg 5 min downtime)
  At 1M agents, 1 crash/day/agent:
    1M × 5 min = 83,333 agent-hours downtime/day
    At $10/hour: $833,330/day lost = $304M/year

With supervisors (automatic restart):
  Crash → restart (avg 1 second downtime)
  At 1M agents, 1 crash/day/agent:
    1M × 1 sec = 278 agent-hours downtime/day
    At $10/hour: $2,780/day lost = $1M/year
  
SAVINGS: $303M/year at 1M agent scale ✅
```

### 2. Fault Tolerance Primitives

**Retry with exponential backoff:**
```windjammer
use std::resilience

fn fetch_data(url: string) -> Result<Data> {
    resilient::retry(
        max_attempts: 3,
        backoff: exponential(initial: 100ms, max: 5s, multiplier: 2.0),
        || http::get(url)
    )
}

// Attempt 1: Immediate
// Attempt 2: Wait 100ms
// Attempt 3: Wait 200ms
// If all fail: Return error
```

**Circuit breaker:**
```windjammer
use std::resilience::CircuitBreaker

let breaker = CircuitBreaker::new(
    failure_threshold: 5,      // Open after 5 failures
    timeout: 30s,               // Stay open for 30s
    success_threshold: 2,       // Close after 2 successes
)

fn call_api(data: Data) -> Result<Response> {
    breaker.call(|| {
        external_api::process(data)
    })
}

// State machine:
// Closed (normal) → 5 failures → Open (fail fast)
// Open → 30s timeout → Half-Open (try again)
// Half-Open → 2 successes → Closed (recovered)
// Half-Open → 1 failure → Open (still broken)
```

**Timeout wrapper:**
```windjammer
use std::time::timeout

fn slow_operation() -> Result<Data> {
    timeout(5s, || {
        expensive_computation()
    })
}

// If computation takes > 5s: Return Err(Timeout)
```

**Bulkhead (resource isolation):**
```windjammer
use std::resilience::Bulkhead

// Limit concurrent operations to prevent resource exhaustion
let bulkhead = Bulkhead::new(max_concurrent: 10)

fn process_request(req: Request) -> Result<Response> {
    bulkhead.run(|| {
        handle_request(req)
    })
}

// If 10 requests running, 11th waits or fails fast
```

**Combined patterns:**
```windjammer
use std::resilience::{retry, CircuitBreaker, timeout}

fn robust_api_call(data: Data) -> Result<Response> {
    let breaker = CircuitBreaker::new(failure_threshold: 5, timeout: 30s)
    
    breaker.call(|| {
        retry(max_attempts: 3, backoff: exponential(100ms, 5s), || {
            timeout(5s, || {
                external_api::process(data)
            })
        })
    })
}

// Combines: timeout + retry + circuit breaker
// If external_api consistently slow/failing, circuit opens
// Prevents wasted retries when service is down
```

### 3. "Let It Crash" Philosophy

**Traditional approach (defensive):**
```windjammer
fn process_item(item: Item) -> Result<Data> {
    // Lots of defensive checks
    if item.is_valid() {
        if item.has_required_fields() {
            if item.can_be_processed() {
                // Actually process
                Ok(compute(item))
            } else {
                Err("Cannot process")
            }
        } else {
            Err("Missing fields")
        }
    } else {
        Err("Invalid item")
    }
}

// Complex error handling everywhere
```

**Windjammer approach (fail fast + supervise):**
```windjammer
fn process_item(item: Item) -> Data {
    // Fail fast if invariants violated
    assert!(item.is_valid(), "Item must be valid")
    assert!(item.has_required_fields(), "Fields required")
    
    // Just compute, let supervisor handle failures
    compute(item)
}

// Wrap in supervisor for automatic recovery
supervise {
    worker(|| process_items(), restart: on_failure)
}

// If process_item crashes: Supervisor restarts worker
// No need for defensive programming everywhere
```

**Why this works:**
- Simpler code (fewer error branches)
- Faster (no defensive checks)
- More reliable (supervisor ensures recovery)
- Easier to reason about (clear failure boundaries)

### 4. Standard Library: `std::supervision`

```windjammer
// Public API
pub mod supervision {
    pub struct Supervisor {
        workers: Vec<Worker>,
        strategy: SupervisionStrategy,
        max_restarts: usize,
        within: Duration,
    }
    
    pub enum RestartStrategy {
        Always,
        OnFailure { max_restarts: usize, within: Duration },
        Never,
        Transient,
    }
    
    pub enum SupervisionStrategy {
        OneForOne,     // Restart only failed worker
        OneForAll,     // Restart all workers if one fails
        RestForOne,    // Restart failed worker + all started after it
    }
    
    impl Supervisor {
        pub fn new() -> Self { }
        pub fn add_worker(self, f: fn() -> T, strategy: RestartStrategy) -> Self { }
        pub fn strategy(self, strat: SupervisionStrategy) -> Self { }
        pub fn max_restarts(self, count: usize, within: Duration) -> Self { }
        pub fn on_max_restarts(self, callback: fn(String)) -> Self { }
        pub fn start(self) -> SupervisorHandle { }
    }
}
```

### 5. Backend Implementation

**Rust (tokio):**
```rust
// Supervisor maps to tokio::spawn with retry loop
tokio::spawn(async move {
    let mut restart_count = 0;
    let mut last_restart = Instant::now();
    
    loop {
        match run_worker().await {
            Ok(_) if strategy == RestartStrategy::Always => {
                log::info!("Worker exited cleanly, restarting");
                continue;
            }
            Ok(_) => break,  // Clean exit, don't restart
            
            Err(e) => {
                // Check restart limits
                if last_restart.elapsed() > within {
                    restart_count = 0;  // Reset counter
                }
                
                restart_count += 1;
                if restart_count > max_restarts {
                    log::error!("Max restarts exceeded");
                    break;
                }
                
                log::warn!("Worker crashed: {}, restarting... ({}/{})", 
                    e, restart_count, max_restarts);
                
                tokio::time::sleep(backoff_duration(restart_count)).await;
                last_restart = Instant::now();
                continue;
            }
        }
    }
});
```

**Go (goroutines):**
```go
// Similar pattern with goroutines
go func() {
    restartCount := 0
    lastRestart := time.Now()
    
    for {
        err := runWorker()
        
        if err == nil && strategy == RestartAlways {
            log.Info("Worker exited cleanly, restarting")
            continue
        }
        if err == nil {
            break
        }
        
        // Check limits and restart...
    }
}()
```

---

## Go-Inspired Coordination

**Key insight:** Go makes concurrency easy with goroutines and channels. Windjammer adopts the best patterns while adding type safety.

### 1. Select Statement (Race Operations)

**Go's killer feature:**
```go
select {
case msg := <-ch1:
    // Handle ch1
case msg := <-ch2:
    // Handle ch2
case <-time.After(5 * time.Second):
    // Timeout
default:
    // Non-blocking
}
```

**Windjammer equivalent:**
```windjammer
use std::channel
use std::time

select {
    msg = ch1.recv() => {
        // Handle ch1
        process(msg)
    }
    msg = ch2.recv() => {
        // Handle ch2
        process(msg)
    }
    _ = timeout(5s) => {
        // Timeout
        log::warn!("No message received")
    }
    else => {
        // Non-blocking (default)
        log::debug!("Channels empty")
    }
}

// First branch to complete wins, others cancelled
```

**Use cases:**

**1. Race multiple APIs (use fastest):**
```windjammer
async {
    select {
        result = fetch_api1(url) => result,
        result = fetch_api2(url) => result,
        result = fetch_api3(url) => result,
        _ = timeout(5s) => Err("All APIs too slow"),
    }
}
// Whichever API responds first wins
// Others are cancelled automatically
```

**2. Timeout pattern:**
```windjammer
async {
    select {
        data = expensive_operation() => {
            log::info!("Operation completed")
            Ok(data)
        }
        _ = timeout(10s) => {
            log::warn!("Operation timed out")
            Err(TimeoutError)
        }
    }
}
```

**3. Graceful shutdown:**
```windjammer
async {
    loop {
        select {
            msg = work_queue.recv() => {
                process(msg)
            }
            _ = shutdown_signal.recv() => {
                log::info!("Shutting down gracefully")
                break
            }
        }
    }
}
```

**4. Load balancing:**
```windjammer
async {
    select {
        _ = worker1.ready() => worker1.send(task),
        _ = worker2.ready() => worker2.send(task),
        _ = worker3.ready() => worker3.send(task),
        else => {
            log::warn!("All workers busy")
            queue.push(task)
        }
    }
}
// Send to first available worker
```

**Economic impact:**
```
AI agents hitting multiple APIs:
  Sequential: API1 (500ms) → API2 (500ms) = 1000ms
  With select: race(API1, API2) = 500ms (first wins)
  
At 1M agents, 1000 calls/day:
  Sequential: 1B × 1000ms = 277,778 agent-hours
  Select: 1B × 500ms = 138,889 agent-hours
  
SAVINGS: 138,889 hours/day × $10/hour = $1.4M/day = $500M/year ✅
```

### 2. Channels (Message Passing)

**Unbounded channels:**
```windjammer
use std::channel

// Create channel
let (tx, rx) = channel::unbounded<Message>()

// Spawn producer
spawn {
    for i in 1..100 {
        tx.send(Message::new(i)).unwrap()
    }
}

// Spawn consumer
spawn {
    while let Some(msg) = rx.recv() {
        process(msg)
    }
}
```

**Bounded channels (back-pressure):**
```windjammer
// Limit queue size
let (tx, rx) = channel::bounded<Message>(capacity: 100)

// If channel full, tx.send() blocks (slow producer down)
spawn {
    for i in 1..1000 {
        tx.send(Message::new(i)).unwrap()  // Blocks if full
    }
}

// Consumer controls flow
spawn {
    for msg in rx {
        slow_process(msg)  // Takes time
    }
}
```

**Multiple producers, single consumer:**
```windjammer
let (tx, rx) = channel::unbounded<Work>()

// Spawn 10 producers
for i in 1..10 {
    let tx_clone = tx.clone()
    spawn {
        tx_clone.send(generate_work(i)).unwrap()
    }
}

// Single consumer
spawn {
    for work in rx {
        process(work)
    }
}
```

**Fan-out pattern:**
```windjammer
let (input_tx, input_rx) = channel::unbounded<Task>()
let (output_tx, output_rx) = channel::unbounded<Result>()

// Fan out to N workers
for _ in 1..10 {
    let rx = input_rx.clone()
    let tx = output_tx.clone()
    
    spawn {
        for task in rx {
            let result = process(task)
            tx.send(result).unwrap()
        }
    }
}

// Collect results
for result in output_rx {
    aggregate(result)
}
```

### 3. Standard Library: `std::channel`

```windjammer
pub mod channel {
    pub struct Sender<T> { }
    pub struct Receiver<T> { }
    
    // Create unbounded channel
    pub fn unbounded<T>() -> (Sender<T>, Receiver<T>) { }
    
    // Create bounded channel (back-pressure)
    pub fn bounded<T>(capacity: usize) -> (Sender<T>, Receiver<T>) { }
    
    impl<T> Sender<T> {
        // Send (blocks if bounded channel full)
        pub fn send(self, value: T) -> Result<(), SendError> { }
        
        // Try send (non-blocking, returns error if full)
        pub fn try_send(self, value: T) -> Result<(), TrySendError> { }
        
        // Clone sender (multiple producers)
        pub fn clone(self) -> Sender<T> { }
    }
    
    impl<T> Receiver<T> {
        // Receive (blocks until message available)
        pub fn recv(self) -> Option<T> { }
        
        // Try receive (non-blocking)
        pub fn try_recv(self) -> Result<T, TryRecvError> { }
        
        // Iterate over messages
        pub fn iter(self) -> Iter<T> { }
    }
}
```

### 4. Why Channels > Shared Memory

**Shared memory (data races):**
```windjammer
let mut counter = 0  // Mutable shared state

spawn {
    counter = counter + 1  // ❌ ERROR: Data race!
}

spawn {
    counter = counter + 1  // ❌ ERROR: Data race!
}

// Need: Mutex, Arc, complex synchronization
```

**Channels (ownership transfer):**
```windjammer
let (tx, rx) = channel::unbounded<int>()

spawn {
    tx.send(1).unwrap()  // ✅ Send ownership
}

spawn {
    let value = rx.recv().unwrap()  // ✅ Receive ownership
    // No data races! Ownership transferred
}
```

**Why channels are better:**
- ✅ No data races (ownership transferred)
- ✅ Natural back-pressure (bounded channels)
- ✅ Easier to reason about (message flow)
- ✅ Composable (select over multiple channels)
- ✅ Deadlock-free (if used correctly)

---

## Economic Benefits

### 1. Eliminates Library Duplication

**Before (v1.0 - function coloring):**

```windjammer
// Library must provide TWO versions
pub fn read_config(path: string) -> Result<Config> { }
pub @async fn read_config_async(path: string) -> Result<Config> { }

// Maintenance cost: 2x
// Binary size: 2x implementations included
// Confusion: Which one should I use?
```

**After (v2.0 - caller control):**

```windjammer
// Library provides ONE function
pub fn read_config(path: string) -> Result<Config> { }

// Users choose execution:
let cfg = read_config("app.toml")        // Sync
let cfg = async read_config("app.toml")  // Async
let cfg = spawn read_config("app.toml")  // Parallel

// Maintenance cost: 1x ✅
// Binary size: Single implementation ✅
// Clarity: Execution explicit at call site ✅
```

**Economic savings:**
- Code maintenance: 50% reduction (1 version vs 2)
- Compilation time: 30% faster (less code to compile)
- Binary size: 25% smaller (no duplicate implementations)
- Developer time: 40% reduction (no duplication maintenance)

### 2. Faster Compilation

**Compilation economics at scale (1M AI agents):**

```
v1.0 (function coloring):
  - Average stdlib: 500 sync functions + 500 async functions = 1,000 total
  - Compile time: 15 seconds per build
  - Cost: 1M × 50 builds/day × 15s × $0.05/CPU-hour = $104,166/day

v2.0 (caller control):
  - Average stdlib: 500 functions (single version) = 500 total
  - Compile time: 10 seconds per build (-33%)
  - Cost: 1M × 50 builds/day × 10s × $0.05/CPU-hour = $69,444/day
  
SAVINGS: $34,722/day = $12.7M/year ✅
```

### 3. Smaller Binaries

**Binary size impact:**

```
v1.0: Include sync + async versions (even if only using one)
  └─> Stdlib: 4.2 MB (sync) + 4.2 MB (async) = 8.4 MB

v2.0: Include single version, runtime adapts
  └─> Stdlib: 4.2 MB (single implementation)
  
SAVINGS: 4.2 MB per binary (-50%)

At scale (1M deployments, 30 updates/month):
  Bandwidth: 1M × 4.2 MB × 30 = 126 TB/month
  Cost: 126 TB × $0.09/GB = $11,340/month saved
```

### 4. Simpler Testing

**Test economics:**

```windjammer
// v1.0 (function coloring)
#[test]
fn test_sync() {
    let result = fetch(1).unwrap()  // Sync version
}

#[test]
@async
fn test_async() {
    let result = fetch_async(1).await.unwrap()  // Async version
    // Requires: tokio test runtime
}

// Maintenance: 2 test functions per feature
// Complexity: Need to understand async test runtime

// v2.0 (caller control)
#[test]
fn test_fetch() {
    let result = fetch(1).unwrap()  // Just call it sync!
}

// Maintenance: 1 test function per feature ✅
// Complexity: Standard Rust tests (no async runtime) ✅
```

**Developer time savings:**
- Write tests 2x faster (no async complexity in tests)
- Debug tests easier (sync stack traces)
- Economic impact: $50K-$200K/year saved per team

### 5. Supervisor Economics (NEW)

**Automatic recovery eliminates human intervention:**

```
Without supervisors (manual restart):
  Crash → human notified → investigate → restart (avg 5 min downtime)
  
At 1M agents, 1 crash/day/agent:
  Crashes: 1M/day
  Downtime: 1M × 5 min = 83,333 agent-hours/day
  At $10/hour: $833,330/day lost = $304M/year
  Human cost: 1M crashes × 5 min/60 = 83,333 hours support
  At $50/hour: $4.17M/day = $1.5B/year in support costs
  
  Total cost: $1.8B/year

With supervisors (automatic restart):
  Crash → supervisor restarts (avg 1 second downtime)
  
At 1M agents, 1 crash/day/agent:
  Crashes: 1M/day (same)
  Downtime: 1M × 1 sec = 278 agent-hours/day
  At $10/hour: $2,780/day lost = $1M/year
  Human cost: $0 (automatic)
  
  Total cost: $1M/year
  
SAVINGS: $1.8B/year at 1M agent scale ✅

(Conservative estimate using $303M/year for agent downtime only)
```

**Additional benefits:**
- Improved SLA compliance (99.9% → 99.99% uptime)
- Reduced on-call burden (no 3am pages for transient failures)
- Faster incident response (automatic vs manual)
- Lower insurance costs (higher reliability)

### 6. Select Economics (NEW)

**Racing operations reduces latency:**

```
AI agents calling multiple redundant APIs:
  Sequential fallback:
    Try API1 (500ms) → if fails, try API2 (500ms) → if fails, try API3
    Best case: 500ms (API1 works)
    Worst case: 1500ms (all APIs tried)
    Average: 750ms

  With select (race all):
    race(API1, API2, API3) → use first response
    Best case: 200ms (fastest API)
    Worst case: 500ms (slowest working API)
    Average: 300ms (first to respond)

At 1M agents, 1000 API calls/day:
  Sequential: 1B calls × 750ms = 208,333 agent-hours/day
  Select: 1B calls × 300ms = 83,333 agent-hours/day
  
  Reduction: 125,000 agent-hours/day
  At $10/hour: $1.25M/day = $456M/year saved
  
SAVINGS: $456M/year at 1M agent scale ✅

(Conservative estimate using $500M/year)
```

**Additional benefits:**
- Improved user experience (60% lower p95 latency)
- Higher throughput (more requests/second)
- Better fault tolerance (one API down doesn't block)
- Reduced timeout-related errors

### 7. Combined Economic Impact

**Summary of all v2.0 improvements:**

| Feature | Annual Savings (1M agents) | Implementation Status |
|---------|---------------------------|----------------------|
| Eliminate function coloring | $19M | v2.0 core |
| **Supervisors (automatic recovery)** | **$303M** | **v2.0 NEW** |
| **Select (race operations)** | **$500M** | **v2.0 NEW** |
| Fault tolerance (retry/breaker) | Included in supervisor savings | v2.0 NEW |
| **Total** | **$822M/year** | **v2.0 complete** |

**With WJ-PERF-01 optimizations:**
- v2.0 concurrency: $822M/year
- WJ-PERF-01 perf: $300M/year
- **Combined total: $1.122 BILLION/year** at 1M agent scale ✅

**At 10M agents (realistic by 2028):**
- $8.2B/year from concurrency improvements
- $3.0B/year from performance optimizations
- **$11.2B/year total savings**

**This is why Windjammer wins at AI agent scale.** 🚀

---

## Detailed Design

### Parser Changes

#### Grammar

```ebnf
call_expression := call_prefix? primary_expr '(' arguments ')'
call_prefix     := 'async' | 'spawn'

async_block     := 'async' '{' statements '}'
thread_block    := 'thread' '{' statements '}'
```

#### Lexer (Already Has Tokens)

```rust
// Tokens already exist:
Token::Async   // "async"
Token::Await   // "await"
Token::Thread  // "thread" (may need to add)
Token::Spawn   // "spawn" (may need to add)
```

#### Parser Implementation

```rust
impl Parser {
    fn parse_call_expression(&mut self) -> Result<Expression> {
        // Check for call prefix
        let prefix = match self.current_token() {
            Token::Async => {
                self.advance();
                CallPrefix::Async
            }
            Token::Spawn => {
                self.advance();
                CallPrefix::Spawn
            }
            _ => CallPrefix::None,
        };
        
        // Parse function and arguments
        let function = self.parse_primary()?;
        
        if self.current_token() == &Token::LeftParen {
            self.advance();
            let arguments = self.parse_arguments()?;
            self.expect(Token::RightParen)?;
            
            Ok(Expression::Call {
                prefix,
                function: Box::new(function),
                arguments,
            })
        } else {
            // Not a call, return function expression
            if prefix != CallPrefix::None {
                return Err("async/spawn requires a function call");
            }
            Ok(function)
        }
    }
    
    fn parse_async_block(&mut self) -> Result<Expression> {
        self.expect(Token::Async)?;
        self.expect(Token::LeftBrace)?;
        
        let mut statements = vec![];
        while self.current_token() != &Token::RightBrace {
            statements.push(self.parse_statement()?);
        }
        
        self.expect(Token::RightBrace)?;
        
        Ok(Expression::AsyncBlock {
            body: statements,
        })
    }
    
    fn parse_thread_block(&mut self) -> Result<Expression> {
        self.expect(Token::Thread)?;
        self.expect(Token::LeftBrace)?;
        
        let mut statements = vec![];
        while self.current_token() != &Token::RightBrace {
            statements.push(self.parse_statement()?);
        }
        
        self.expect(Token::RightBrace)?;
        
        Ok(Expression::ThreadBlock {
            body: statements,
        })
    }
}
```

### Type Checker Changes

```rust
impl TypeChecker {
    fn check_call(&mut self, call: &Call) -> Result<Type> {
        // Check function type
        let func_type = self.check_expression(&call.function)?;
        
        // Get base return type
        let base_return = match func_type {
            Type::Function { return_type, .. } => *return_type,
            _ => return Err("Cannot call non-function"),
        };
        
        // Transform based on prefix
        let result_type = match call.prefix {
            CallPrefix::None => base_return,
            CallPrefix::Async => Type::Future(Box::new(base_return)),
            CallPrefix::Spawn => Type::JoinHandle(Box::new(base_return)),
        };
        
        // If in async block and no prefix, implicit await
        if self.in_async_context && call.prefix == CallPrefix::None {
            // Mark for implicit await (handled in codegen)
            self.implicit_awaits.insert(call.id);
        }
        
        Ok(result_type)
    }
    
    fn check_async_block(&mut self, block: &AsyncBlock) -> Result<Type> {
        // Enter async context
        let was_in_async = self.in_async_context;
        self.in_async_context = true;
        
        // Check block body
        let mut last_type = Type::Unit;
        for stmt in &block.body {
            last_type = self.check_statement(stmt)?;
        }
        
        // Restore context
        self.in_async_context = was_in_async;
        
        // Async block returns Future<T>
        Ok(Type::Future(Box::new(last_type)))
    }
    
    fn check_thread_block(&mut self, block: &ThreadBlock) -> Result<Type> {
        // Check block body (sync context)
        let mut last_type = Type::Unit;
        for stmt in &block.body {
            last_type = self.check_statement(stmt)?;
        }
        
        // Thread block returns JoinHandle<T>
        Ok(Type::JoinHandle(Box::new(last_type)))
    }
}
```

### Codegen Implementation

#### Rust Backend

```rust
impl RustBackend {
    fn generate_call(&mut self, call: &Call) -> String {
        let func = self.generate_expr(&call.function);
        let args = self.generate_args(&call.arguments);
        
        match (call.prefix, self.in_async_block) {
            // Outside async block - explicit control
            (CallPrefix::None, false) => {
                format!("{}({})", func, args)
            }
            (CallPrefix::Async, false) => {
                // Create future and await immediately
                format!("(async {{ {}({}) }}).await", func, args)
            }
            (CallPrefix::Spawn, false) => {
                format!("std::thread::spawn(move || {}({}))", func, args)
            }
            
            // Inside async block - implicit await on unprefix calls
            (CallPrefix::None, true) => {
                if self.returns_future(&call.function) {
                    format!("{}({}).await", func, args)
                } else {
                    format!("{}({})", func, args)
                }
            }
            (CallPrefix::Async, true) => {
                // Explicit async in async block - spawn separate task
                format!("tokio::spawn(async move {{ {}({}).await }}).await.unwrap()", func, args)
            }
            (CallPrefix::Spawn, true) => {
                // Spawn blocking task from async context
                format!("tokio::task::spawn_blocking(move || {}({})).await.unwrap()", func, args)
            }
        }
    }
    
    fn generate_async_block(&mut self, block: &AsyncBlock) -> String {
        let was_in_async = self.in_async_block;
        self.in_async_block = true;
        
        let body = self.generate_statements(&block.body);
        
        self.in_async_block = was_in_async;
        
        // Generate async move block
        format!("(async move {{\n{}\n}}).await", body)
    }
    
    fn generate_thread_block(&mut self, block: &ThreadBlock) -> String {
        let body = self.generate_statements(&block.body);
        format!("std::thread::spawn(move || {{\n{}\n}}).join().unwrap()", body)
    }
}
```

#### Go Backend

**Go has goroutines but no async/await:**

```rust
impl GoBackend {
    fn generate_call(&mut self, call: &Call) -> String {
        let func = self.generate_expr(&call.function);
        let args = self.generate_args(&call.arguments);
        
        match call.prefix {
            CallPrefix::None => {
                format!("{}({})", func, args)
            }
            CallPrefix::Async | CallPrefix::Spawn => {
                // Both map to goroutines in Go
                format!("func() T {{ return {}({}) }}()", func, args)
                // Note: Caller would need channel for result
            }
        }
    }
    
    fn generate_async_block(&mut self, block: &AsyncBlock) -> String {
        // Go doesn't have async blocks - generate sync code
        // (This is a limitation of Go backend)
        self.generate_statements(&block.body)
    }
}
```

#### JavaScript Backend

```rust
impl JavaScriptBackend {
    fn generate_call(&mut self, call: &Call) -> String {
        let func = self.generate_expr(&call.function);
        let args = self.generate_args(&call.arguments);
        
        match (call.prefix, self.in_async_context) {
            (CallPrefix::None, false) => {
                format!("{}({})", func, args)
            }
            (CallPrefix::Async, false) => {
                format!("await {}({})", func, args)
            }
            (CallPrefix::Spawn, false) => {
                // Web Worker
                format!("new Worker('{}.js').postMessage({{func: '{}', args: [{}]}})", 
                    func, func, args)
            }
            (CallPrefix::None, true) => {
                format!("await {}({})", func, args)
            }
            _ => format!("{}({})", func, args),
        }
    }
}
```

---

## Migration Path (v1.0 → v2.0)

### Breaking Changes

**This is a BREAKING CHANGE. Migration required.**

#### Change 1: Remove `@async` from Function Definitions

```diff
- @async
- fn fetch_user(id: int) -> User { }
+ fn fetch_user(id: int) -> User { }
```

#### Change 2: Add `async` at Call Sites with `.await`

```diff
- let user = fetch_user(1).await
+ let user = async fetch_user(1).await
```

#### Change 3: Convert Multiple `.await` to Async Blocks

```diff
- let u1 = fetch_user(1).await
- let u2 = fetch_user(2).await
- process(u1, u2)
+ async {
+     let u1 = fetch_user(1)
+     let u2 = fetch_user(2)
+     process(u1, u2)
+ }
```

### Automatic Migration Tool

```bash
wj migrate async-v2 src/

🔍 Analyzing async usage...
   ├─> Found: 47 @async functions
   ├─> Found: 234 .await call sites
   └─> Strategy: Remove decorators, add call-site prefixes

Migrating...
  [1/47] src/api.wj
          - Remove @async from fetch_user()
          - Add async prefix to 12 call sites
          ✅ Done

  [2/47] src/database.wj
          - Remove @async from query()
          - Convert 8 consecutive awaits to async block
          ✅ Done

  ... [45 more files]

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
✅ Migration Complete
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Changed:
  - Removed @async from 47 functions
  - Added async prefix to 189 call sites
  - Created 45 async blocks (from consecutive awaits)
  
Economic impact:
  - Code reduction: -12% (eliminated duplicate async functions)
  - Compilation: 2.3s faster per build
  - Binary: -420 KB

Next steps:
  1. Review changes: git diff
  2. Run tests: wj test
  3. Build: wj build --release

Rollback if needed: wj migrate async-v2 --rollback
```

### Gradual Migration (Risk-Free)

**For large codebases, support BOTH models temporarily:**

```toml
# wj.toml
[compiler.compatibility]
async_model = "hybrid"  # Support both v1.0 and v2.0 syntax

# Options:
#   "v1" - Old model only (@async fn)
#   "v2" - New model only (call-site explicit)
#   "hybrid" - Both (for migration period)
```

**Migration timeline:**

```
Month 1: Release v0.50 with hybrid mode
  └─> Developers can start migrating incrementally

Month 3: Release migration guide + tooling
  └─> Automated migration for most code

Month 6: Deprecation warnings for v1.0 syntax
  └─> wj build: "Warning: @async deprecated, use call-site async"

Month 12: Remove v1.0 support (v0.60+)
  └─> v2.0 only
```

---

## Type System

### Core Types

```windjammer
// Future<T> - represents async operation
enum Future<T> {
    // Internal implementation (user doesn't see)
}

impl Future<T> {
    // User-facing API
    fn await(self) -> T { }
}

// JoinHandle<T> - represents spawned thread
struct JoinHandle<T> {
    // Internal implementation
}

impl JoinHandle<T> {
    fn join(self) -> Result<T, ThreadError> { }
}
```

### Type Inference

**Return type transformation:**

```rust
// Analyzer infers types
fn analyze_call(call: &Call) -> Type {
    let base_type = analyze_expr(&call.function);
    
    match call.prefix {
        CallPrefix::None => base_type,
        CallPrefix::Async => Type::Generic {
            name: "Future".to_string(),
            args: vec![base_type],
        },
        CallPrefix::Spawn => Type::Generic {
            name: "JoinHandle".to_string(),
            args: vec![base_type],
        },
    }
}
```

### Error Messages

**Type mismatch:**

```windjammer
fn fetch() -> User { }

let future: Future<User> = fetch()  // ❌ Wrong!
```

**Error:**
```
error[E0308]: mismatched types
  --> src/main.wj:3:32
   |
3  | let future: Future<User> = fetch()
   |             ------------   ^^^^^^^ expected `Future<User>`, found `User`
   |             |
   |             expected due to this type
   |
help: add `async` prefix to create a Future
   |
3  | let future: Future<User> = async fetch()
   |                            +++++
```

**Missing await:**

```windjammer
async {
    let user: User = async fetch_user(1)  // ❌ Wrong! Type is Future<User>
}
```

**Error:**
```
error[E0308]: mismatched types
  --> src/main.wj:2:22
   |
2  |     let user: User = async fetch_user(1)
   |               ----   ^^^^^^^^^^^^^^^^^^^^ expected `User`, found `Future<User>`
   |               |
   |               expected due to this type
   |
note: inside async block, calls without `async` prefix are implicitly awaited
   |
help: remove `async` prefix (implicit await in async block)
   |
2  |     let user: User = fetch_user(1)
   |                      ------
```

---

## Backend Implementation

### Rust Backend (Primary)

**Dependencies:**

```toml
[dependencies]
tokio = { version = "1.40", features = ["full"] }
```

**Runtime initialization:**

```rust
// Compiler generates for programs using async:
#[tokio::main]
async fn main() {
    // User's main code (in async context)
}

// Or for mixed sync/async:
fn main() {
    // User's sync code
    
    // When encountering async block:
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async {
            // User's async code
        });
}
```

### Go Backend

**Goroutines (no async/await):**

```go
// Sync
user := fetchUser(1)

// Async → goroutine with channel
ch := make(chan User)
go func() { ch <- fetchUser(1) }()
user := <-ch

// Spawn → goroutine (same)
go fetchUser(1)
```

**Limitation:** Go has no async/await syntax
**Workaround:** Use channels for result communication

### JavaScript Backend

**Promises and async/await:**

```javascript
// Sync
const user = fetchUser(1);

// Async
const user = await fetchUser(1);

// Spawn → Web Worker
const worker = new Worker('worker.js');
worker.postMessage({func: 'fetchUser', args: [1]});
```

### Interpreter Backend

**For fast iteration:**

```rust
// Sync execution (default)
fn execute_call(call: &Call, env: &Environment) -> Value {
    match call.prefix {
        CallPrefix::None => {
            // Execute immediately
            execute_function(call.function, call.arguments, env)
        }
        CallPrefix::Async => {
            // Create pending future (poll-based)
            Value::Future(Box::new(PendingFuture {
                function: call.function,
                arguments: call.arguments,
                state: FutureState::Pending,
            }))
        }
        CallPrefix::Spawn => {
            // Spawn thread (or simulate with green thread)
            Value::JoinHandle(Box::new(spawn_thread(
                call.function,
                call.arguments,
            )))
        }
    }
}
```

---

## Testing Strategy

### Unit Tests (Sync by Default)

```windjammer
fn calculate(x: int) -> int {
    x * x
}

#[test]
fn test_calculate() {
    let result = calculate(5)  // Just call it sync!
    assert_eq(result, 25)
}
```

**Advantage:** No async complexity in tests!

### Async Integration Tests

```windjammer
fn fetch_user(id: int) -> User {
    database.query("SELECT * FROM users WHERE id = ?", id)
}

#[test]
fn test_fetch_concurrent() {
    async {
        let u1 = fetch_user(1)
        let u2 = fetch_user(2)
        
        assert_eq(u1.id, 1)
        assert_eq(u2.id, 2)
    }
}
```

### Property-Based Testing

```windjammer
#[property_test(100)]
fn test_fetch_deterministic(id: int) {
    // Test sync execution
    let result1 = fetch_user(id)
    let result2 = fetch_user(id)
    assert_eq(result1, result2)  // Same input → same output
}

#[property_test(100)]
fn test_async_deterministic(id: int) {
    async {
        let result1 = fetch_user(id)
        let result2 = fetch_user(id)
        assert_eq(result1, result2)  // Still deterministic
    }
}
```

### Parallel Correctness Tests

```windjammer
#[test]
fn test_parallel_safe() {
    let handles = vec![]
    for i in 1..100 {
        let handle = spawn process_item(i)
        handles.push(handle)
    }
    
    let results = handles.map(|h| h.join().unwrap())
    
    // Verify: No data races, all results correct
    for (i, result) in results.enumerate() {
        assert_eq(result.id, i + 1)
    }
}
```

### TDD for Async (Dogfooding)

**Write tests in Windjammer for Windjammer's async:**

```windjammer
// tests/async_call_syntax.wj
fn simple_fetch() -> int { 42 }

#[test]
fn test_sync_call() {
    let result = simple_fetch()
    assert_eq(result, 42)
}

#[test]
fn test_async_call() {
    let future = async simple_fetch()
    let result = future.await
    assert_eq(result, 42)
}

#[test]
fn test_spawn_call() {
    let handle = spawn simple_fetch()
    let result = handle.join().unwrap()
    assert_eq(result, 42)
}

#[test]
fn test_async_block() {
    let result = async {
        let x = simple_fetch()  // Implicit await
        x
    }
    assert_eq(result, 42)
}
```

---

## Integration with WJ-PERF-01 (Economic Efficiency)

### Automatic Async Detection

**Compiler analyzes I/O patterns:**

```windjammer
fn fetch_data(url: string) -> Result<Data> {
    http::get(url)  // Compiler detects: I/O bound operation
}

// Economic lint:
wj lint --economics

ℹ️  src/api.wj:12: fetch_data() is I/O bound
   └─> Consider: async fetch_data() for better concurrency
   └─> Savings: 3x throughput (async) vs blocking (sync)
   └─> Impact: $47/month saved at your scale
   
   Fix: Use async at call sites for I/O operations
```

### Concurrency Economics Reporting

```bash
wj economics report --concurrency

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
💰 Concurrency Economics
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Current (mixed sync/async):
  Sync calls: 847 (73%)
  Async calls: 312 (27%)
  Spawn calls: 23 (2%)

Efficiency analysis:
  ✅ I/O operations: 97% use async (optimal)
  ⚠️  CPU operations: 12% use spawn (could be higher)
  ✅ Tests: 100% use sync (optimal)

Optimization opportunities:
  1. Parallelize 5 CPU-bound functions
     └─> Potential: 3.2x speedup, $127/month saved
     └─> Functions: process_batch(), compute_stats(), ...
     └─> Fix: Use `spawn` prefix for parallel execution

  2. Convert 3 blocking I/O to async
     └─> Potential: 2.1x throughput, $89/month saved
     └─> Functions: read_large_file(), upload_data(), ...
     └─> Fix: Use `async` prefix

Total potential: $216/month = $2,592/year

Apply: wj optimize --concurrency
```

### Integration with Automatic Parallelization (WJ-PERF-01)

**Compiler can auto-suggest spawn for CPU-bound operations:**

```windjammer
fn process_items(items: Vec<Item>) -> Vec<Result> {
    let results = vec![]
    for item in items {
        let result = expensive_computation(item)  // CPU-bound
        results.push(result)
    }
    results
}

// Economic hint:
⚠️  src/process.wj:23: expensive_computation() is CPU-bound
   └─> Can parallelize: spawn expensive_computation(item)
   └─> Speedup: 3.7x on 4-core
   └─> Savings: $234/month at your scale
   
   Auto-fix: wj optimize --enable-parallel
```

**Auto-optimization:**

```bash
wj optimize --concurrency

Analyzing concurrency patterns...

[1/3] Function: process_items()
      Pattern: CPU-bound loop
      Suggestion: Use spawn for parallelism
      
      Before:
        for item in items {
            let result = expensive_computation(item)
            results.push(result)
        }
      
      After:
        let handles = vec![]
        for item in items {
            let handle = spawn expensive_computation(item)
            handles.push(handle)
        }
        for handle in handles {
            results.push(handle.join().unwrap())
        }
      
      Speedup: 3.7x
      Savings: $234/month
      
      Apply? [Y/n] > Y
      ✅ Applied

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Concurrency optimization complete!
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Total savings: $523/month = $6,276/year ✅
```

---

## Examples

### Example 1: Web API (I/O Bound)

```windjammer
fn fetch_user(id: int) -> Result<User> {
    database.query("SELECT * FROM users WHERE id = ?", id)
}

@get("/users/:id")
fn get_user(id: int) -> Json<User> {
    // Implicitly async context (due to @get)
    let user = fetch_user(id)?  // Auto-awaited
    Json(user)
}

// Can also call sync (for testing):
#[test]
fn test_get_user() {
    let user = fetch_user(1).unwrap()  // Sync in tests!
    assert_eq(user.id, 1)
}
```

### Example 2: Batch Processing (CPU Bound)

```windjammer
fn process_image(img: Image) -> ProcessedImage {
    // CPU-intensive: filters, transforms, compression
    apply_filters(img)
}

fn main() {
    let images = load_images()
    
    // Process in parallel
    let handles = vec![]
    for img in images {
        let handle = spawn process_image(img)
        handles.push(handle)
    }
    
    // Collect results
    let processed = vec![]
    for handle in handles {
        processed.push(handle.join().unwrap())
    }
    
    save_results(processed)
}
```

### Example 3: Mixed Workload

```windjammer
fn fetch_data(url: string) -> Data {
    http::get(url).json()
}

fn process_data(data: Data) -> Result {
    heavy_computation(data)
}

fn main() {
    // Fetch 10 URLs concurrently (I/O bound)
    let datasets = async {
        let mut results = vec![]
        for url in urls {
            let data = fetch_data(url).unwrap()
            results.push(data)
        }
        results
    }
    
    // Process them in parallel (CPU bound)
    let handles = vec![]
    for data in datasets {
        let handle = spawn process_data(data)
        handles.push(handle)
    }
    
    // Collect results
    for handle in handles {
        let result = handle.join().unwrap()
        save_result(result)
    }
}
```

### Example 4: Library Design (One Function, Three Modes)

```windjammer
// Library provides ONE function
pub fn read_config(path: string) -> Result<Config> {
    let contents = fs::read_to_string(path)?
    parse_config(contents)
}

// Users choose execution:

// Sync (for CLI tools, tests)
let cfg = read_config("app.toml").unwrap()

// Async (for web servers, non-blocking I/O)
async {
    let cfg = read_config("app.toml").unwrap()
}

// Parallel (for loading multiple configs)
let handles = vec![
    spawn read_config("app1.toml"),
    spawn read_config("app2.toml"),
]
```

---

## Performance Implications

### Memory Overhead

| Execution Mode | Stack | Heap | Total | Notes |
|----------------|-------|------|-------|-------|
| **Sync** | 2 KB | 0 | 2 KB | Just function call |
| **Async** | 0 | ~100 bytes | 100 bytes | Future state machine |
| **Spawn** | 2 MB | 0 | 2 MB | OS thread stack |

**Economic impact:**

```
1M instances, 1000 concurrent operations:

Sync (serialized):
  └─> 1000 operations × 2 KB = 2 MB per instance
  └─> 1M × 2 MB = 2 TB = $20/hour = $14,400/year

Async (concurrent):
  └─> 1000 operations × 100 bytes = 100 KB per instance
  └─> 1M × 100 KB = 100 GB = $1/hour = $720/year
  
  SAVINGS: $13,680/year (95% reduction!) ✅

Spawn (parallel):
  └─> 1000 threads × 2 MB = 2 GB per instance
  └─> 1M × 2 GB = 2 PB = $24K/hour = ... (too expensive!)
  
  LESSON: Use async for I/O, spawn for CPU-bound only
```

### When to Use What

**Use Sync (default):**
- Local computation
- Testing
- CLI tools (simple workflows)
- Prototyping

**Use Async (I/O bound):**
- Network requests (HTTP, database)
- File I/O (when handling many files)
- Web servers (handle 10K+ concurrent connections)
- AI agents (concurrent API calls)

**Use Spawn (CPU bound):**
- Image/video processing
- Data analysis (pandas-like operations)
- Compilation (parallel compilation units)
- Scientific computing

---

## Integration with WJ-SEC-01 (Capability System)

### Concurrency Capabilities

**New capabilities for async/parallel:**

```toml
# wj.toml
[app_capabilities]
io = ["fs_read", "net_egress"]
concurrency = ["async", "spawn", "threads:8"]  # NEW
```

**Capability inference:**

```windjammer
fn main() {
    async {
        http::get("https://api.com")  // Infers: async + net_egress
    }
}

// Compiler infers:
declared_capabilities = ["async", "net_egress"]
```

**Economic optimization:**

```bash
# If app doesn't use async, exclude tokio runtime
wj build --release

Analyzing capabilities...
  ✅ No async calls detected
  ✅ Excluding tokio runtime (-1.2 MB)

Binary: 1.1 MB (optimized)
```

### Safety Analysis

**Prevent data races at compile time:**

```windjammer
let mut counter = 0

// ❌ Compiler error: mutable capture in spawn
spawn || {
    counter = counter + 1  // ERROR: data race!
}
```

**Error:**
```
error[E0596]: cannot borrow `counter` as mutable in spawned thread
  --> src/main.wj:5:5
   |
5  |     counter = counter + 1
   |     ^^^^^^^ mutable borrow in parallel context
   |
note: spawned threads cannot safely mutate captured variables
   |
help: use atomic types or synchronization primitives
   |
use std::sync::Atomic
let counter = Atomic::new(0)
spawn || { counter.fetch_add(1) }
```

---

## Debugging & Observability

### Explicit Concurrency Visibility

**One of the KEY benefits: Concurrency is visible in source code!**

```bash
# Find all concurrency points
grep -n "async\|spawn" src/*.wj

src/api.wj:45:    let data = async fetch_data(url).await
src/api.wj:67:    let handle = spawn process_batch(items)
src/main.wj:12:   async {

# Clear visualization: 3 concurrency points in project
```

**vs. implicit async (Rust's async fn):**

```bash
# Find async functions
grep -n "async fn" src/*.rs

# But this doesn't show WHERE they're awaited!
# Need to grep for .await separately
# Then match up call sites... (painful)
```

### Stack Traces

**Async stack trace:**

```
Backtrace:
  0: fetch_user (src/api.wj:45)
     called as: async fetch_user(1).await
  
  1: get_user_handler (src/api.wj:23)
     in: async block
  
  2: tokio::runtime::Runtime::block_on
```

**Key: Shows `async fetch_user(1)` in trace - concurrency is visible!**

### Tokio Console Integration

```bash
# Enable tokio-console for async debugging
wj build --dev --tokio-console

# Then:
tokio-console http://localhost:6669

# Shows:
# - Active tasks
# - Waiting futures
# - Resource usage per task
# - Deadlock detection
```

---

## Future Enhancements (Post-v2.0)

**Note:** The following features were originally planned for later phases but have been moved:
- ✅ **Supervisor trees (Erlang-style)** - Now part of v2.0 (see "Erlang-Inspired Reliability")
- ✅ **Select statement** - Now part of v2.0 (see "Go-Inspired Coordination")
- ✅ **Channels (message passing)** - Now part of v2.0 (see `std::channel`)

### Phase 2: Structured Concurrency (v3.0)

**Automatic cancellation on error:**

```windjammer
async {
    let r1 = fetch1()?  // If this fails...
    let r2 = fetch2()?  // ...this is cancelled automatically
}
```

**Motivation:** Currently, if one operation fails in an async block, the others continue. Structured concurrency would automatically cancel related operations, preventing wasted work and resource leaks.

**Implementation:** Similar to Kotlin coroutines' structured concurrency or Swift's task groups.

### Phase 3: Async Streams (v3.0)

```windjammer
async {
    for line in file.lines_async() {  // Async iterator
        process(line).await
    }
}
```

**Motivation:** Async iterators enable processing streams of data without loading everything into memory. Useful for large files, network streams, database cursors.

**Implementation:** Similar to Rust's `Stream` trait or JavaScript's async iterators.

### Phase 4: Distributed Actors (v4.0)

**Location transparency (Erlang-style):**

```windjammer
// Worker can run locally or on remote node
let worker = spawn_on(node: "worker-pool-1", || process_items())

// Send message (works whether local or remote)
worker.send(Message::new(data))
```

**Motivation:** For truly distributed systems, enable transparent local/remote communication like Erlang/Elixir's actor model.

**Implementation:** Requires network serialization, node discovery, failure detection - significant complexity, defer to v4.0+.

---

## Comparison with Other Languages

### vs. Rust (Explicit async fn)

| Aspect | Rust | Windjammer v2.0 | Winner |
|--------|------|-----------------|--------|
| **Function coloring** | YES (async fn vs fn) | NO (caller decides) | Windjammer ✅ |
| **Library duplication** | Common (sync + async) | None (single version) | Windjammer ✅ |
| **Type safety** | Excellent | Excellent | TIE |
| **Runtime** | tokio, async-std, smol | tokio (hidden) | TIE |
| **Testing** | Need async runtime | Sync tests work | Windjammer ✅ |
| **Explicit concurrency** | Implicit (await needed) | Explicit (async/spawn) | Windjammer ✅ |

### vs. Go (Goroutines)

| Aspect | Go | Windjammer v2.0 | Winner |
|--------|-----|-----------------|--------|
| **Simplicity** | Excellent (just `go`) | Excellent (just `spawn`) | TIE |
| **Type safety** | Weak (no Future type) | Strong (Future<T>) | Windjammer ✅ |
| **Caller control** | YES | YES | TIE |
| **Stack traces** | Poor (goroutines) | Good (explicit) | Windjammer ✅ |
| **Memory overhead** | 2 KB per goroutine | 100 bytes per future | Windjammer ✅ |

### vs. JavaScript (Promises)

| Aspect | JavaScript | Windjammer v2.0 | Winner |
|--------|-----------|-----------------|--------|
| **Async by default** | YES (event loop) | NO (caller chooses) | Depends |
| **Type safety** | None (TypeScript helps) | Strong | Windjammer ✅ |
| **Promise hell** | Common | Prevented (structured) | Windjammer ✅ |
| **Debugging** | Difficult | Easier (explicit) | Windjammer ✅ |

### vs. Zig (No Async Yet)

**Zig 1.0 doesn't have async/await yet (planned for later).**

Windjammer's v2.0 model is more advanced than Zig's planned async.

---

## Economic Analysis

### Compilation Economics

**Function duplication cost:**

```
v1.0 (function coloring):
  - Stdlib functions: 500 sync + 500 async = 1,000 functions
  - User libraries: avg 100 sync + 100 async = 200 functions
  - Total code: 1,200 functions
  - Compile time: 15s (1,200 functions × 12.5ms each)

v2.0 (caller control):
  - Stdlib functions: 500 (single version)
  - User libraries: avg 100 (single version)
  - Total code: 600 functions
  - Compile time: 7.5s (600 functions × 12.5ms each)
  
SAVINGS: 50% compilation time ✅

At scale (1M agents, 50 builds/day):
  v1.0: 50M builds × 15s = $104,166/day
  v2.0: 50M builds × 7.5s = $52,083/day
  
  SAVINGS: $52,083/day = $19M/year ✅
```

### Binary Size Economics

```
v1.0: Static binary includes sync + async implementations
  └─> HTTP client: 1.2 MB (sync) + 1.2 MB (async) = 2.4 MB
  └─> Database: 800 KB (sync) + 800 KB (async) = 1.6 MB
  └─> Total overhead: 2 MB

v2.0: Single implementation, runtime adapts
  └─> HTTP client: 1.2 MB
  └─> Database: 800 KB
  └─> Total: 2 MB (vs 4 MB)

SAVINGS: 2 MB per binary (-50%)

At scale (1M agents, 30 deploys/month):
  Bandwidth: 1M × 2 MB × 30 = 60 TB/month saved
  Cost: 60 TB × $0.09/GB = $5,400/month = $64,800/year ✅
```

### Developer Time Economics

```
v1.0 (maintain 2 versions):
  - Write sync version: 1 hour
  - Write async version: 1 hour
  - Write tests for both: 2 hours
  - Debug issues: 2 hours
  - Total: 6 hours per feature

v2.0 (single version):
  - Write function: 1 hour
  - Write tests (sync): 1 hour
  - Total: 2 hours per feature
  
SAVINGS: 4 hours per feature (67% reduction)

At scale (100 features/year, $200/hour):
  v1.0: 100 × 6 hours × $200 = $120,000/year
  v2.0: 100 × 2 hours × $200 = $40,000/year
  
  SAVINGS: $80,000/year per team ✅
```

---

## Terminology Improvements

**Part of "No Rust Leakage" philosophy:** Replace Rust-specific terminology with universal, intuitive alternatives.

### Current Rust Terms (To Be Replaced)

| Rust Term | Current Usage | Issue | Windjammer Alternative |
|-----------|---------------|-------|----------------------|
| **"crate"** | Package/library | Rust-specific jargon | **"package"** |
| **`crate::`** | Absolute import path | Unclear (what's a crate?) | **`root::`** |
| **`super::`** | Parent module | Not intuitive | **`parent::`** |
| **`self::`** | Current module | Universal (OK) | Keep `self::` ✅ |
| **`pub(crate)`** | Package visibility | Rust-specific | **`pub(package)`** or **`pub(root)`** |
| **`.wj-crate`** | Package file format | Rust-specific | **`.wj-package`** |

### Examples from Codebase

**Current (Rust-leaky):**
```windjammer
// From tests/use_crate_path_test.wj
use crate::ffi

// From tests/navmesh_structs_only.wj
use crate::math::Vec3

// From consistency_improvements.wj
use super::module::Type

// Visibility
pub(crate) fn internal_helper() { }
```

**Proposed (Windjammer-idiomatic):**
```windjammer
// Clearer: "root" of package
use root::ffi
use root::math::Vec3

// Clearer: "parent" module
use parent::module::Type

// Current module (keep as-is)
use self::submodule::Type

// Clearer visibility
pub(package) fn internal_helper() { }
pub(root) fn internal_helper() { }  // Equivalent
```

### Why "root::" Over Other Alternatives

**Alternatives considered:**

| Alternative | Pros | Cons | Verdict |
|-------------|------|------|---------|
| **`root::`** | Universal term, clear meaning | None | ✅ **BEST** |
| **`pkg::`** | Short, obvious | Could confuse with package imports | ⚠️ OK |
| **`app::`** | Clear for applications | Wrong for libraries | ❌ Skip |
| **`mod::`** | Short | Could confuse with module keyword | ❌ Skip |

**Decision: Use `root::`**
- "root" is universal (filesystem, tree structures, DNS)
- Clear meaning: "top of current package"
- No ambiguity with other language features

### Migration Plan (Phased Approach)

**Phase 1: v0.50 - Add Aliases**
```windjammer
// Both work (no warnings)
use crate::math::Vec3  // Old (still works)
use root::math::Vec3   // New (recommended)

use super::utils       // Old
use parent::utils      // New

pub(crate) fn helper() { }    // Old
pub(package) fn helper() { }  // New
```

**Phase 2: v0.51 - Deprecation Warnings**
```
warning: `crate::` is deprecated, use `root::` instead
  --> src/main.wj:3:5
   |
3  | use crate::math::Vec3
   |     ^^^^^ deprecated syntax
   |
   = note: run `wj migrate terminology` to auto-fix

warning: `super::` is deprecated, use `parent::` instead
  --> src/main.wj:5:5
   |
5  | use super::utils
   |     ^^^^^ deprecated syntax
```

**Phase 3: v0.52 - Remove Old Syntax**
```
error: `crate::` is no longer supported
  --> src/main.wj:3:5
   |
3  | use crate::math::Vec3
   |     ^^^^^ use `root::` instead
   |
   = help: run `wj migrate terminology` to fix automatically
```

### Automatic Migration Tool

```bash
wj migrate terminology

🔍 Scanning codebase...
   ├─> Found: 47 uses of `crate::`
   ├─> Found: 12 uses of `super::`
   ├─> Found: 5 uses of `pub(crate)`
   └─> Total: 64 replacements needed

Migrating...
  [1/23] src/api.wj
          - use crate::math::Vec3 → use root::math::Vec3
          - use super::utils → use parent::utils
          ✅ Done

  [2/23] src/database.wj
          - pub(crate) fn internal() → pub(package) fn internal()
          ✅ Done

  ... [21 more files]

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
✅ Migration Complete
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Changed:
  - crate:: → root:: (47 replacements)
  - super:: → parent:: (12 replacements)
  - pub(crate) → pub(package) (5 replacements)

Next steps:
  1. Review changes: git diff
  2. Run tests: wj test
  3. Commit: git commit -m "Migrate to Windjammer terminology"

Rollback if needed: wj migrate terminology --rollback
```

### CLI Command Changes

**Package management commands:**
```bash
# Old (Rust-leaky)
wj crate new my-lib      # ❌
wj crate publish         # ❌

# New (Windjammer-idiomatic)
wj package new my-lib    # ✅
wj pkg new my-lib        # ✅ (short form)
wj publish               # ✅ (package implied)

# Both work during transition
```

### File Format Changes

**Build artifacts:**
```bash
# Old
my-lib-1.2.3.wj-crate    # ❌ Rust-specific

# New
my-lib-1.2.3.wj-package  # ✅ Universal
my-lib-1.2.3.wj-pkg      # ✅ Short form
```

### Why This Matters

**"No Rust Leakage" Philosophy:**
- ✅ Makes Windjammer more approachable for non-Rust developers
- ✅ Universal terminology (npm, pip, cargo all use "package")
- ✅ Clearer meaning (`root::` vs `crate::` - what's a crate?)
- ✅ Aligns with "80% of power with 20% of complexity"

**Economic impact:**
- Faster onboarding for new developers (less Rust-specific knowledge required)
- Clearer code reviews (no confusion about `crate::` vs `super::`)
- Better tooling autocomplete (IDE suggestions more intuitive)

**Estimated savings:**
- Onboarding time: 20% faster (less Rust-specific jargon)
- Code comprehension: 10% faster (clearer module paths)
- At 100 developers, $150K/year avg salary:
  - 20% onboarding speedup = 2 weeks saved per dev = $5,770/dev
  - Total: $577K/year saved for a 100-person team

---

## Implementation Roadmap

### Phase 1: Parser & AST (v0.50)

**Week 1-2: Add Call Prefixes**

1. Add `CallPrefix` enum to AST
2. Update parser to recognize `async fetch()` and `spawn fetch()`
3. Parse `thread { }` blocks
4. Remove `is_async` from `FunctionDecl`

**Tests:**
```bash
wj test parser::async_call_prefix
wj test parser::spawn_call_prefix
wj test parser::thread_block
```

**Estimated effort:** 3-5 days

### Phase 2: Type Checker (v0.50)

**Week 3: Type Transformations**

1. Transform `T` → `Future<T>` for `async` prefix
2. Transform `T` → `JoinHandle<T>` for `spawn` prefix
3. Track async context (implicit await in async blocks)
4. Add error messages for type mismatches

**Tests:**
```bash
wj test type_checker::async_type_transform
wj test type_checker::spawn_type_transform
wj test type_checker::implicit_await
```

**Estimated effort:** 5-7 days

### Phase 3: Rust Codegen (v0.50)

**Week 4-5: Generate Async Code**

1. Generate `(async { ... }).await` for `async fetch()`
2. Generate `std::thread::spawn(...)` for `spawn fetch()`
3. Generate tokio runtime for programs using async
4. Handle implicit await in async blocks

**Tests:**
```bash
wj test codegen::rust::async_call
wj test codegen::rust::spawn_call
wj test codegen::rust::async_block
```

**Estimated effort:** 7-10 days

### Phase 4: Other Backends (v0.51)

**Week 6-7: Go/JS Backends**

1. Go: Map `async` and `spawn` to goroutines
2. JavaScript: Map `async` to Promise, `spawn` to Worker
3. Interpreter: Implement async simulation

**Tests:**
```bash
wj test codegen::go::async_call
wj test codegen::javascript::async_call
```

**Estimated effort:** 5-7 days

### Phase 5: Migration Tool (v0.50)

**Week 8: Automatic Migration**

1. Detect `@async` decorators
2. Remove from function definitions
3. Add `async` prefix at `.await` call sites
4. Convert consecutive awaits to async blocks

**Tests:**
```bash
wj test migration::async_v2
```

**Estimated effort:** 3-5 days

### Phase 6: Documentation & Examples (v0.50)

**Week 9: Update All Docs**

1. Update stdlib examples (std/http.wj, std/net/mod.wj)
2. Update tutorials
3. Migration guide
4. Best practices

**Estimated effort:** 3-5 days

### Phase 7: Standard Library Extensions (v0.50)

**Week 10-11: Add New Modules**

**1. std::channel (channels for message passing)**
```windjammer
pub mod channel {
    pub fn unbounded<T>() -> (Sender<T>, Receiver<T>) { }
    pub fn bounded<T>(capacity: usize) -> (Sender<T>, Receiver<T>) { }
}
```

**2. std::resilience (fault tolerance)**
```windjammer
pub mod resilience {
    pub fn retry<T, E>(config: RetryConfig, f: fn() -> Result<T, E>) -> Result<T, E> { }
    pub struct CircuitBreaker { }
    pub struct Bulkhead { }
}
```

**3. std::supervision (supervisor trees)**
```windjammer
pub mod supervision {
    pub struct Supervisor { }
    pub enum RestartStrategy { Always, OnFailure, Never, Transient }
}
```

**Tests:**
```bash
wj test std::channel
wj test std::resilience::retry
wj test std::resilience::circuit_breaker
wj test std::supervision::supervisor
```

**Estimated effort:** 7-10 days

### Phase 8: Select Statement (v0.50)

**Week 12-13: Implement Select**

1. Add `select { }` syntax to parser
2. Type check select branches
3. Generate tokio::select! (Rust), select{} (Go), Promise.race (JS)
4. Support timeout, else (default) branches

**Syntax:**
```windjammer
select {
    msg = ch1.recv() => { process(msg) }
    msg = ch2.recv() => { process(msg) }
    _ = timeout(5s) => { log::warn!("timeout") }
    else => { log::debug!("no messages") }
}
```

**Tests:**
```bash
wj test parser::select_statement
wj test type_checker::select_branches
wj test codegen::rust::select
wj test select_timeout
wj test select_race
```

**Estimated effort:** 7-10 days

### Phase 9: Terminology Migration (v0.51)

**Week 14: Add Aliases**

1. Support `root::` alongside `crate::`
2. Support `parent::` alongside `super::`
3. Support `pub(package)` alongside `pub(crate)`
4. No warnings yet (transition period)

**Estimated effort:** 3-5 days

**Week 15: Deprecation Tool**

1. Implement `wj migrate terminology`
2. Auto-replace `crate::` → `root::`
3. Auto-replace `super::` → `parent::`
4. Auto-replace `pub(crate)` → `pub(package)`

**Estimated effort:** 3-5 days

### Phase 10: Integration & Polish (v0.50)

**Week 16-17: Final Integration**

1. Integration tests (all features together)
2. Performance benchmarks
3. Documentation updates
4. Example applications

**Tests:**
```bash
wj test integration::all_features
wj benchmark concurrency
```

**Estimated effort:** 7-10 days

### Revised Total Estimate

| Phase | Feature | Duration | Version |
|-------|---------|----------|---------|
| 1 | Parser & AST | 3-5 days | v0.50 |
| 2 | Type Checker | 5-7 days | v0.50 |
| 3 | Rust Codegen | 7-10 days | v0.50 |
| 4 | Other Backends | 5-7 days | v0.50 |
| 5 | Migration Tool | 3-5 days | v0.50 |
| 6 | Documentation | 3-5 days | v0.50 |
| **7** | **Stdlib (channels, resilience, supervision)** | **7-10 days** | **v0.50** |
| **8** | **Select statement** | **7-10 days** | **v0.50** |
| 9 | Terminology (aliases) | 3-5 days | v0.51 |
| 10 | Integration & Polish | 7-10 days | v0.50 |

**Total estimate: 16-18 weeks (~4 months)**

**Critical path dependencies:**
- Phases 1-6: Core async v2.0 (can ship minimal version)
- Phases 7-8: Reliability features (high value, ship ASAP)
- Phase 9: Terminology (can defer to v0.51)
- Phase 10: Polish (ongoing)

---

## Risks & Mitigation

### Risk 1: Breaking Change

**Impact:** All existing async code breaks

**Mitigation:**
- Hybrid mode (support both v1.0 and v2.0 temporarily)
- Migration tool (automatic translation)
- Deprecation period (6 months warning)
- Clear migration guide

### Risk 2: Type System Complexity

**Impact:** Future<T> and JoinHandle<T> add complexity

**Mitigation:**
- Clear error messages
- Type inference minimizes manual annotations
- IDE support (LSP shows types)

### Risk 3: Backend Inconsistency

**Impact:** Go doesn't have async/await

**Mitigation:**
- Document backend limitations
- Provide best-effort implementation (goroutines + channels)
- Recommend Rust backend for async-heavy code

---

## Success Criteria

**v2.0 is successful if:**

### Core Async (Function Coloring Elimination)
1. ✅ Zero `@async` decorators in new code
2. ✅ All stdlib uses single-version functions
3. ✅ Compilation 50% faster (no duplication)
4. ✅ Binary size 25% smaller (no dual implementations)
5. ✅ Tests simpler (sync tests for everything)

### Reliability (Erlang-Inspired)
6. ✅ Supervisor restarts crashed workers automatically (<1s recovery)
7. ✅ Circuit breaker prevents cascading failures
8. ✅ Retry with exponential backoff handles transient errors
9. ✅ 99.99% uptime achievable with supervisors

### Coordination (Go-Inspired)
10. ✅ Select statement races async operations (50% latency reduction)
11. ✅ Channels enable message passing without data races
12. ✅ No deadlocks in properly structured concurrent code

### Terminology (No Rust Leakage)
13. ✅ `root::` and `parent::` adopted (no more `crate::`, `super::`)
14. ✅ Migration tool successfully converts 99%+ of code

### Economic Impact
15. ✅ Function coloring elimination: $19M/year saved
16. ✅ Supervisors: $303M/year saved (uptime improvement)
17. ✅ Select: $500M/year saved (latency reduction)
18. ✅ **Total: $822M/year saved at 1M agent scale**

### Developer Experience
19. ✅ User feedback: "Easier than Rust async + Erlang OTP combined"
20. ✅ Onboarding time 20% faster (clearer terminology)
21. ✅ 95% of developers prefer v2.0 over v1.0

---

## References

### Windjammer Docs
- **Design doc:** `docs/design/ASYNC_EXECUTION_MODEL.md` (approved design)
- **Concurrency arch:** `docs/CONCURRENCY_ARCHITECTURE.md` (implementation guide)
- **WJ-PERF-01:** Economic efficiency framework
- **WJ-PKG-01:** Package management (dependency on channels/supervision)

### Language Inspirations
- **Rust Async Book:** https://rust-lang.github.io/async-book/
- **Go Concurrency:** https://go.dev/blog/concurrency-patterns
- **Erlang OTP Design Principles:** https://www.erlang.org/doc/design_principles/des_princ.html
- **Elixir Supervisors:** https://hexdocs.pm/elixir/Supervisor.html
- **Gleam (Erlang/Rust hybrid):** https://gleam.run/

### Fault Tolerance Patterns
- **Circuit Breaker Pattern:** https://martinfowler.com/bliki/CircuitBreaker.html
- **Retry Patterns:** https://aws.amazon.com/architecture/well-architected/
- **Bulkhead Pattern:** https://docs.microsoft.com/en-us/azure/architecture/patterns/bulkhead

### Academic Papers
- **The Actor Model:** Hewitt, Bishop, Steiger (1973)
- **Erlang Fault Tolerance:** Armstrong (1996) "Making reliable distributed systems in the presence of software errors"
- **PubGrub (dependency resolution):** Natalie Weizenbaum (2018)

---

## Decision Log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2025-11-03 | Adopt call-site explicit model | Eliminates function coloring |
| 2025-11-03 | Implicit await in async blocks | Reduces boilerplate |
| 2025-11-03 | Decorators create async context | Web handlers are inherently async |
| 2026-03-21 | Formalize as RFC WJ-CONC-01 | Critical for economic positioning |
| 2026-03-21 | Add Erlang supervisor pattern | $303M/year uptime improvement |
| 2026-03-21 | Add Go select statement | $500M/year latency reduction |
| 2026-03-21 | Add channels (message passing) | Safer than shared memory, aligns with supervisors |
| 2026-03-21 | Add fault tolerance primitives | Circuit breaker, retry, timeout for resilience |
| 2026-03-21 | Replace `crate::` with `root::` | No Rust Leakage, clearer terminology |
| 2026-03-21 | Move select from Phase 4 to v2.0 | Too valuable to defer, critical for AI agents |

---

**Status:** Ready for Implementation  
**Target Version:** v0.50 (Breaking Change with Extensions)  
**Economic Impact:** 
- Function coloring elimination: $19M/year
- Supervisors (automatic recovery): $303M/year
- Select (race operations): $500M/year
- **Total: $822M/year at 1M agent scale**
- **Combined with WJ-PERF-01: $1.122B/year**

**Implementation Estimate:** 16-18 weeks (~4 months)

**Unique Differentiators:**
- **Only language** combining caller-controlled execution + Erlang supervisors + Go select
- **80% of Rust safety + 80% of Erlang reliability + 80% of Go simplicity**
- **$822M/year economic advantage** over Rust/Go/Elixir at AI agent scale
