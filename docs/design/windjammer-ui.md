# Windjammer UI Framework Design

**Version:** 0.34.0  
**Status:** In Development  
**Philosophy:** Svelte-inspired simplicity, multi-target support (JavaScript + WASM)

---

## Overview

Windjammer UI is a **complete, full-stack UI framework** that compiles to both JavaScript and WebAssembly. It combines the simplicity of Svelte with the performance of Rust, providing a unified solution for web development.

### Key Principles

1. **Simplicity First** - Closer to Svelte than React
2. **Compiler-Driven** - No runtime overhead where possible
3. **Multi-Target** - Same code runs as JavaScript or WASM
4. **Type-Safe** - Full type checking at compile time
5. **Zero-Config** - Sensible defaults, easy overrides

---

## Component Model

### Basic Component

```windjammer
@component
struct Counter {
    // Reactive state - automatically tracks dependencies
    state count: int = 0
    
    // Computed values - derived from state
    computed double: int {
        count * 2
    }
    
    // Event handlers
    fn increment() {
        count += 1
    }
    
    // Render function - JSX-like syntax
    fn render() -> Html {
        <div class="counter">
            <h1>"Count: {count}"</h1>
            <p>"Double: {double}"</p>
            <button onclick={increment}>"+"</button>
        </div>
    }
}
```

### Component with Props

```windjammer
@component
struct Greeting {
    // Props - passed from parent
    props name: string
    props age: int = 18  // Default value
    
    fn render() -> Html {
        <div>
            <h2>"Hello, {name}!"</h2>
            <p>"Age: {age}"</p>
        </div>
    }
}
```

### Lifecycle Methods

```windjammer
@component
struct Timer {
    state seconds: int = 0
    
    // Lifecycle hooks
    fn on_mount() {
        // Called when component is added to DOM
        set_interval(|| {
            seconds += 1
        }, 1000)
    }
    
    fn on_unmount() {
        // Cleanup
    }
    
    fn on_update(prev_props: Props) {
        // Called after props/state change
    }
    
    fn render() -> Html {
        <div>"Elapsed: {seconds}s"</div>
    }
}
```

---

## Reactivity System

### Fine-Grained Reactivity (Svelte-style)

The compiler tracks dependencies automatically:

```windjammer
@component
struct App {
    state first_name: string = "John"
    state last_name: string = "Doe"
    
    // Automatically re-computes when first_name or last_name changes
    computed full_name: string {
        format!("{} {}", first_name, last_name)
    }
    
    fn render() -> Html {
        <div>
            <input value={first_name} onchange={|e| first_name = e.target.value} />
            <input value={last_name} onchange={|e| last_name = e.target.value} />
            <p>"Full name: {full_name}"</p>
        </div>
    }
}
```

**How it works:**
1. Compiler analyzes `computed` blocks to find dependencies
2. Generates subscription code for each dependency
3. Only re-computes when dependencies change (fine-grained updates)

---

## Multi-Target Support

### JavaScript Target

```bash
wj build --target=javascript --ui app.wj
```

**Generated:**
- Vanilla JavaScript (ES2020+)
- Virtual DOM for efficient updates
- Event delegation for performance
- Tree-shakable output

### WASM Target

```bash
wj build --target=wasm --ui app.wj
```

**Generated:**
- WebAssembly module
- JavaScript glue code (minimal)
- DOM manipulation via web-sys
- Zero-copy strings where possible

### Shared Code

The same component works for both targets:

```windjammer
@component
struct App {
    state count: int = 0
    
    fn render() -> Html {
        <button onclick={|| count += 1}>
            "Count: {count}"
        </button>
    }
}

// Compile to JavaScript:
// - Virtual DOM diffing
// - Event listeners via addEventListener

// Compile to WASM:
// - Direct DOM manipulation via web-sys
// - Event listeners via closures
```

---

## Server-Side Rendering (SSR)

### Component with SSR

```windjammer
@component
@ssr  // Enable server-side rendering
struct Page {
    props title: string
    state data: Vec<Item> = vec![]
    
    // Server-only code
    #[server]
    async fn load_data() -> Vec<Item> {
        database::query("SELECT * FROM items").await
    }
    
    // Called on server
    async fn on_server_load() {
        data = load_data().await
    }
    
    fn render() -> Html {
        <html>
            <head><title>{title}</title></head>
            <body>
                <h1>{title}</h1>
                <ul>
                    {data.iter().map(|item| {
                        <li>{item.name}</li>
                    })}
                </ul>
            </body>
        </html>
    }
}
```

**SSR Flow:**
1. Server renders component to HTML string
2. HTML sent to browser
3. Client-side JavaScript/WASM hydrates (attaches event listeners)
4. Component becomes interactive

---

## Routing

### File-Based Routing

```
pages/
  index.wj          → /
  about.wj          → /about
  blog/
    index.wj        → /blog
    [slug].wj       → /blog/:slug
  users/
    [id]/
      profile.wj    → /users/:id/profile
```

### Route Component

```windjammer
// pages/blog/[slug].wj
@component
@route("/blog/:slug")
struct BlogPost {
    route slug: string  // Extracted from URL
    state post: Option<Post> = None
    
    async fn on_mount() {
        post = fetch_post(slug).await
    }
    
    fn render() -> Html {
        match post {
            Some(p) => <article>
                <h1>{p.title}</h1>
                <div>{p.content}</div>
            </article>,
            None => <div>"Loading..."</div>
        }
    }
}
```

---

## Styling

### Component-Scoped Styles

```windjammer
@component
struct Card {
    props title: string
    
    fn render() -> Html {
        <div class="card">
            <h2>{title}</h2>
            <slot />  // Children
        </div>
    }
    
    // Scoped styles - automatically prefixed
    style {
        .card {
            padding: 1rem;
            border-radius: 8px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
        }
        
        h2 {
            margin: 0 0 1rem 0;
            font-size: 1.5rem;
        }
    }
}
```

**Generated CSS:**
```css
.card-abc123 {
    padding: 1rem;
    border-radius: 8px;
    box-shadow: 0 2px 4px rgba(0,0,0,0.1);
}

.card-abc123 h2 {
    margin: 0 0 1rem 0;
    font-size: 1.5rem;
}
```

---

## Forms & Validation

### Form Component

```windjammer
@component
struct LoginForm {
    state email: string = ""
    state password: string = ""
    state errors: HashMap<string, string> = HashMap::new()
    
    fn validate() -> bool {
        errors.clear()
        
        if email.is_empty() {
            errors.insert("email", "Email is required")
            return false
        }
        
        if !email.contains("@") {
            errors.insert("email", "Invalid email")
            return false
        }
        
        if password.len() < 8 {
            errors.insert("password", "Password must be 8+ characters")
            return false
        }
        
        true
    }
    
    async fn handle_submit(e: Event) {
        e.prevent_default()
        
        if validate() {
            let result = api::login(email, password).await
            // Handle result...
        }
    }
    
    fn render() -> Html {
        <form onsubmit={handle_submit}>
            <div>
                <label>"Email"</label>
                <input 
                    type="email" 
                    value={email} 
                    onchange={|e| email = e.target.value}
                    class={errors.contains_key("email") ? "error" : ""}
                />
                {errors.get("email").map(|err| <span class="error">{err}</span>)}
            </div>
            
            <div>
                <label>"Password"</label>
                <input 
                    type="password" 
                    value={password} 
                    onchange={|e| password = e.target.value}
                    class={errors.contains_key("password") ? "error" : ""}
                />
                {errors.get("password").map(|err| <span class="error">{err}</span>)}
            </div>
            
            <button type="submit">"Log In"</button>
        </form>
    }
}
```

---

## State Management

### Global Store

```windjammer
@store
struct AppStore {
    state user: Option<User> = None
    state theme: Theme = Theme::Light
    state notifications: Vec<Notification> = vec![]
    
    fn login(user: User) {
        self.user = Some(user)
    }
    
    fn logout() {
        self.user = None
    }
    
    fn toggle_theme() {
        self.theme = match self.theme {
            Theme::Light => Theme::Dark,
            Theme::Dark => Theme::Light
        }
    }
}

@component
struct Header {
    // Access global store
    use store: AppStore
    
    fn render() -> Html {
        <header class={store.theme}>
            {match store.user {
                Some(u) => <div>
                    "Hello, {u.name}"
                    <button onclick={|| store.logout()}>"Logout"</button>
                </div>,
                None => <a href="/login">"Login"</a>
            }}
        </header>
    }
}
```

---

## WebSocket Support

### Real-Time Component

```windjammer
@component
struct Chat {
    state messages: Vec<Message> = vec![]
    state socket: WebSocket
    
    async fn on_mount() {
        socket = WebSocket::connect("ws://localhost:8080/chat").await?
        
        socket.on_message(|msg| {
            messages.push(msg)
        })
    }
    
    fn send_message(text: string) {
        socket.send(Message { text, user: current_user() })
    }
    
    fn render() -> Html {
        <div class="chat">
            <div class="messages">
                {messages.iter().map(|msg| {
                    <div class="message">
                        <strong>{msg.user}:</strong> {msg.text}
                    </div>
                })}
            </div>
            <input onsubmit={|e| {
                send_message(e.target.value)
                e.target.value = ""
            }} />
        </div>
    }
}
```

---

## Compilation Strategy

### JavaScript Target

1. **Parse** Windjammer component
2. **Generate** Virtual DOM code
3. **Optimize** with minification, tree shaking
4. **Output** ES2020+ JavaScript

### WASM Target

1. **Parse** Windjammer component
2. **Generate** Rust code with web-sys bindings
3. **Compile** to WebAssembly with wasm-bindgen
4. **Output** .wasm + minimal JS glue

### Comparison

| Feature | JavaScript | WASM |
|---------|-----------|------|
| **Size** | Smaller for simple apps | Smaller for complex apps |
| **Performance** | Good | Excellent |
| **Startup** | Fast | Slower (WASM load) |
| **Best For** | Content sites, forms | Games, data viz, heavy computation |

---

## Implementation Plan

### Phase 1: Core (v0.34.0)
- [x] Design document
- [ ] Component model & @component macro
- [ ] Reactive state system
- [ ] Virtual DOM (JavaScript target)
- [ ] Basic rendering

### Phase 2: Multi-Target (v0.34.1)
- [ ] WASM target support
- [ ] web-sys integration
- [ ] Unified API for both targets

### Phase 3: Full-Stack (v0.34.2)
- [ ] Server-side rendering
- [ ] Client-side hydration
- [ ] File-based routing
- [ ] HTTP server integration

### Phase 4: Polish (v0.34.3)
- [ ] Component-scoped styling
- [ ] Form handling
- [ ] WebSocket support
- [ ] Global state management
- [ ] Comprehensive docs & examples

---

## Example: Full App

```windjammer
// App.wj
@component
struct App {
    state current_page: Page = Page::Home
    
    fn render() -> Html {
        <div id="app">
            <Header />
            <main>
                {match current_page {
                    Page::Home => <HomePage />,
                    Page::About => <AboutPage />,
                    Page::Blog(slug) => <BlogPost slug={slug} />
                }}
            </main>
            <Footer />
        </div>
    }
}

// main.wj
fn main() {
    windjammer_ui::mount("#app", App::new())
}
```

**Compile to JavaScript:**
```bash
wj build --target=javascript --ui --minify --ssr main.wj
```

**Compile to WASM:**
```bash
wj build --target=wasm --ui main.wj
```

---

## Competitive Advantages

1. **Multi-Target** - JavaScript OR WASM from same code
2. **Simple** - Svelte-like API, easy to learn
3. **Type-Safe** - Full Rust type checking
4. **Fast** - Compiler optimizations, fine-grained reactivity
5. **Complete** - SSR, routing, forms, WebSockets built-in
6. **No Runtime** - Most reactivity compiled away

**Windjammer UI = Svelte's simplicity + Rust's safety + Multi-target flexibility**

