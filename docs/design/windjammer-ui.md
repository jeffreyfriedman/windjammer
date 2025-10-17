# Windjammer UI Framework Design

**Version:** 0.34.0  
**Status:** In Development  
**Philosophy:** Svelte-inspired simplicity, true cross-platform (Web + Desktop + Mobile)  
**Inspiration:** Svelte + Dioxus + Tauri + Flutter

---

## Overview

Windjammer UI is a **complete, cross-platform UI framework** that compiles to Web (JavaScript/WASM), Desktop (native), and Mobile (iOS/Android). Write once, run everywhere—with the simplicity of Svelte, the cross-platform power of Dioxus, and the native integration of Tauri.

### Key Principles

1. **Simplicity First** - Closer to Svelte than React
2. **Compiler-Driven** - No runtime overhead where possible
3. **True Cross-Platform** - Web, Desktop, Mobile from same code
4. **Native Performance** - Use platform's native UI when available
5. **Type-Safe** - Full type checking at compile time
6. **Zero-Config** - Sensible defaults, easy overrides

### Target Platforms

| Platform | Rendering | Package Format | Native APIs |
|----------|-----------|---------------|-------------|
| **Web** | Virtual DOM or WASM+web-sys | JavaScript bundle | Web APIs |
| **Desktop** | Native (Tauri + webview or native widgets) | .app, .exe, .deb | Filesystem, system tray, etc |
| **Mobile** | Native (iOS UIKit / Android Views) | .ipa, .apk | Camera, GPS, contacts, etc |

### Architecture Inspiration

- **Svelte**: Simplicity, compile-time reactivity, minimal runtime
- **Dioxus**: Cross-platform component model, RSX syntax, renderer abstraction
- **Tauri**: Native desktop integration, small bundles, Rust backend
- **Flutter**: Widget composition, platform channels, hot reload

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

### Web Target (JavaScript)

1. **Parse** Windjammer component
2. **Generate** Virtual DOM code
3. **Optimize** with minification, tree shaking
4. **Output** ES2020+ JavaScript

### Web Target (WASM)

1. **Parse** Windjammer component
2. **Generate** Rust code with web-sys bindings
3. **Compile** to WebAssembly with wasm-bindgen
4. **Output** .wasm + minimal JS glue

### Desktop Target (Tauri-style)

1. **Parse** Windjammer component
2. **Generate** Rust code with native UI bindings
3. **Compile** Rust backend + webview frontend
4. **Package** as .app/.exe/.deb with Tauri
5. **Output** Native app (2-10MB, tiny compared to Electron!)

### Mobile Target (iOS/Android)

1. **Parse** Windjammer component
2. **Generate** platform-specific code:
   - iOS: Swift/UIKit or Rust with objc bindings
   - Android: Kotlin/Views or Rust with JNI bindings
3. **Compile** to native binary
4. **Package** as .ipa or .apk
5. **Output** Native mobile app

### Platform Comparison

| Feature | Web (JS) | Web (WASM) | Desktop | Mobile |
|---------|----------|------------|---------|--------|
| **Size** | Small | Medium | 2-10MB | 5-15MB |
| **Performance** | Good | Excellent | Excellent | Native |
| **Startup** | Instant | Fast | Fast | Instant |
| **Native APIs** | Web only | Web only | Full OS | Full device |
| **Distribution** | URL | URL | App stores | App stores |
| **Best For** | Websites | Heavy compute | Productivity | Apps on-the-go |

---

## Implementation Plan

### Phase 1: Core & Web (v0.34.0)
- [x] Cross-platform design document
- [ ] Component model & @component macro
- [ ] Reactive state system (fine-grained)
- [ ] Virtual DOM (JavaScript target)
- [ ] WASM rendering (web-sys target)
- [ ] Basic event system
- [ ] Platform abstraction layer (foundation)

### Phase 2: Desktop (v0.34.1)
- [ ] Tauri integration
- [ ] Native webview rendering
- [ ] Desktop-specific APIs (file dialogs, system tray, notifications)
- [ ] Platform detection (#[platform(desktop)])
- [ ] Desktop packaging (.app, .exe, .deb)

### Phase 3: Mobile (v0.34.2)
- [ ] iOS rendering (UIKit bindings)
- [ ] Android rendering (Views/JNI bindings)
- [ ] Mobile-specific APIs (camera, GPS, contacts, permissions)
- [ ] Platform detection (#[platform(mobile)])
- [ ] Mobile packaging (.ipa, .apk)

### Phase 4: Full-Stack Features (v0.34.3)
- [ ] Server-side rendering (web only)
- [ ] Client-side hydration
- [ ] File-based routing (all platforms)
- [ ] HTTP server integration
- [ ] WebSocket support (all platforms)

### Phase 5: Polish & Production (v0.34.4)
- [ ] Component-scoped styling
- [ ] Form handling and validation
- [ ] Global state management (@store)
- [ ] Hot reload (development)
- [ ] Performance optimizations
- [ ] Comprehensive docs & examples
- [ ] Sample apps (TodoMVC, chat, desktop editor, mobile app)

---

## Cross-Platform Native APIs

### Platform Abstraction Layer

```windjammer
// Write once, works everywhere
@component
struct PhotoPicker {
    state photo: Option<Image> = None
    
    async fn pick_photo() {
        // Compiler generates platform-specific code:
        // - Web: <input type="file" accept="image/*">
        // - Desktop: native file picker dialog
        // - Mobile: camera roll / camera
        photo = native::pick_image().await
    }
    
    fn render() -> Html {
        <div>
            <button onclick={pick_photo}>"Pick Photo"</button>
            {photo.map(|img| <img src={img.url} />)}
        </div>
    }
}
```

### Native Capabilities

```windjammer
use windjammer::native::*

@component
struct NativeFeatures {
    state location: Option<Location> = None
    state contacts: Vec<Contact> = vec![]
    
    // Platform detection (compile-time)
    #[platform(desktop)]
    fn show_system_notification() {
        notification::show("Hello from Windjammer!")
    }
    
    #[platform(mobile)]
    async fn get_location() {
        location = gps::get_current_location().await
    }
    
    #[platform(web)]
    fn use_web_api() {
        console::log("Running in browser")
    }
    
    fn render() -> Html {
        <div>
            <button onclick={get_location}>"Get Location"</button>
            {location.map(|loc| <p>"Lat: {loc.lat}, Lng: {loc.lng}"</p>)}
        </div>
    }
}
```

### Desktop-Specific (Tauri Integration)

```windjammer
@component
@platform(desktop)
struct DesktopApp {
    state files: Vec<PathBuf> = vec![]
    
    async fn open_file_dialog() {
        files = desktop::file_dialog()
            .set_title("Open Files")
            .pick_multiple()
            .await
    }
    
    fn create_system_tray() {
        desktop::system_tray()
            .with_icon("icon.png")
            .with_menu(vec![
                MenuItem::new("Show", show_window),
                MenuItem::new("Quit", quit_app),
            ])
            .build()
    }
    
    fn render() -> Html {
        <div>
            <button onclick={open_file_dialog}>"Open Files"</button>
            <ul>
                {files.iter().map(|path| <li>{path.display()}</li>)}
            </ul>
        </div>
    }
}
```

### Mobile-Specific

```windjammer
@component
@platform(mobile)
struct MobileApp {
    state contacts: Vec<Contact> = vec![]
    state photo: Option<Image> = None
    
    async fn request_permissions() {
        mobile::permissions()
            .request_camera()
            .request_contacts()
            .await
    }
    
    async fn take_photo() {
        photo = mobile::camera::capture().await
    }
    
    async fn load_contacts() {
        contacts = mobile::contacts::fetch_all().await
    }
    
    fn render() -> Html {
        <ScrollView>
            <Button onclick={take_photo}>"Take Photo"</Button>
            <Button onclick={load_contacts}>"Load Contacts"</Button>
            
            {photo.map(|img| <Image source={img} />)}
            
            <List data={contacts}>
                {|contact| <ListItem>
                    <Text>{contact.name}</Text>
                    <Text>{contact.phone}</Text>
                </ListItem>}
            </List>
        </ScrollView>
    }
}
```

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

**Compile to Web (JavaScript):**
```bash
wj build --target=javascript --ui --minify --ssr main.wj
```

**Compile to Web (WASM):**
```bash
wj build --target=wasm --ui main.wj
```

**Compile to Desktop:**
```bash
wj build --target=desktop --ui main.wj
# Outputs: myapp.app (macOS), myapp.exe (Windows), myapp.deb (Linux)
```

**Compile to Mobile:**
```bash
wj build --target=ios --ui main.wj        # → myapp.ipa
wj build --target=android --ui main.wj    # → myapp.apk
```

---

## Competitive Advantages

1. **True Cross-Platform** - Web, Desktop, Mobile from same code
2. **Simple** - Svelte-like API, easy to learn
3. **Type-Safe** - Full Rust type checking
4. **Native Performance** - Real native apps (not Electron!)
5. **Small Bundles** - 2-10MB desktop apps (vs 100MB+ Electron)
6. **Fast** - Compiler optimizations, fine-grained reactivity
7. **Complete** - SSR, routing, forms, WebSockets, native APIs built-in
8. **No Runtime** - Most reactivity compiled away

### vs Competition

| Framework | Web | Desktop | Mobile | Bundle Size | Language |
|-----------|-----|---------|--------|-------------|----------|
| **Windjammer** | ✅ | ✅ | ✅ | 2-10MB | Windjammer |
| React Native | ✅ | ❌ | ✅ | Large | JavaScript |
| Flutter | ✅ | ✅ | ✅ | 15-30MB | Dart |
| Electron | ❌ | ✅ | ❌ | 100MB+ | JavaScript |
| Tauri | ✅ | ✅ | ❌ | 2-10MB | Rust+JS |
| Dioxus | ✅ | ✅ | ✅ | Medium | Rust |
| Svelte | ✅ | ❌ | ❌ | Small | JavaScript |

**Windjammer UI = Svelte's simplicity + Dioxus's cross-platform + Tauri's native integration + Rust's safety**

