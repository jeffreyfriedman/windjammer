//! File-based routing system for all platforms

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Route definition
#[derive(Debug, Clone)]
pub struct Route {
    /// Route path pattern (e.g., "/users/:id")
    pub path: String,
    /// Component name or handler
    pub handler: String,
    /// Route parameters extracted from URL
    pub params: HashMap<String, String>,
    /// Query parameters
    pub query: HashMap<String, String>,
    /// Child routes (for nested routing)
    pub children: Vec<Route>,
}

impl Route {
    /// Create a new route
    pub fn new(path: String, handler: String) -> Self {
        Self {
            path,
            handler,
            params: HashMap::new(),
            query: HashMap::new(),
            children: Vec::new(),
        }
    }

    /// Add a child route
    pub fn child(mut self, route: Route) -> Self {
        self.children.push(route);
        self
    }

    /// Match a path against this route
    pub fn matches(&self, path: &str) -> Option<HashMap<String, String>> {
        let route_parts: Vec<&str> = self.path.split('/').filter(|s| !s.is_empty()).collect();
        let path_parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

        if route_parts.len() != path_parts.len() {
            return None;
        }

        let mut params = HashMap::new();

        for (route_part, path_part) in route_parts.iter().zip(path_parts.iter()) {
            if let Some(param_name) = route_part.strip_prefix(':') {
                // Dynamic parameter
                params.insert(param_name.to_string(), path_part.to_string());
            } else if let Some(param_name) = route_part.strip_prefix('*') {
                // Wildcard - matches everything
                params.insert(param_name.to_string(), path_part.to_string());
            } else if route_part != path_part {
                // Static part doesn't match
                return None;
            }
        }

        Some(params)
    }
}

/// Router for managing routes and navigation
pub struct Router {
    /// Registered routes
    routes: Arc<Mutex<Vec<Route>>>,
    /// Current route
    current: Arc<Mutex<Option<Route>>>,
    /// Navigation history
    history: Arc<Mutex<Vec<String>>>,
    /// Navigation listeners
    #[allow(clippy::type_complexity)]
    listeners: Arc<Mutex<Vec<Arc<dyn Fn(&Route) + Send + Sync>>>>,
}

impl Router {
    /// Create a new router
    pub fn new() -> Self {
        Self {
            routes: Arc::new(Mutex::new(Vec::new())),
            current: Arc::new(Mutex::new(None)),
            history: Arc::new(Mutex::new(Vec::new())),
            listeners: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Register a route
    pub fn add_route(&self, route: Route) {
        let mut routes = self.routes.lock().unwrap();
        routes.push(route);
    }

    /// Navigate to a path
    pub fn navigate(&self, path: &str) -> Result<(), String> {
        let (matched_route, params) = self.find_route(path)?;

        // Update history
        let mut history = self.history.lock().unwrap();
        history.push(path.to_string());

        // Update current route
        let mut current = self.current.lock().unwrap();
        let mut route = matched_route.clone();
        route.params = params;

        // Parse query string if present
        if let Some(query_start) = path.find('?') {
            route.query = self.parse_query(&path[query_start + 1..]);
        }

        *current = Some(route.clone());

        // Notify listeners
        self.notify_listeners(&route);

        Ok(())
    }

    /// Go back in history
    pub fn back(&self) -> Result<(), String> {
        let mut history = self.history.lock().unwrap();
        if history.len() > 1 {
            history.pop(); // Remove current
            if let Some(previous) = history.last() {
                let path = previous.clone();
                drop(history); // Release lock before navigate
                return self.navigate(&path);
            }
        }
        Err("No history to go back to".to_string())
    }

    /// Go forward in history (if available)
    pub fn forward(&self) -> Result<(), String> {
        // In a full implementation, would track forward history
        Err("Forward navigation not yet implemented".to_string())
    }

    /// Get current route
    pub fn current(&self) -> Option<Route> {
        self.current.lock().unwrap().clone()
    }

    /// Get route parameter
    pub fn param(&self, name: &str) -> Option<String> {
        self.current
            .lock()
            .unwrap()
            .as_ref()
            .and_then(|r| r.params.get(name).cloned())
    }

    /// Get query parameter
    pub fn query(&self, name: &str) -> Option<String> {
        self.current
            .lock()
            .unwrap()
            .as_ref()
            .and_then(|r| r.query.get(name).cloned())
    }

    /// Add a navigation listener
    pub fn on_navigate<F>(&self, listener: F)
    where
        F: Fn(&Route) + Send + Sync + 'static,
    {
        let mut listeners = self.listeners.lock().unwrap();
        listeners.push(Arc::new(listener));
    }

    fn find_route(&self, path: &str) -> Result<(Route, HashMap<String, String>), String> {
        let routes = self.routes.lock().unwrap();

        // Remove query string for matching
        let path_without_query = path.split('?').next().unwrap_or(path);

        for route in routes.iter() {
            if let Some(params) = route.matches(path_without_query) {
                return Ok((route.clone(), params));
            }
        }

        Err(format!("No route found for path: {}", path))
    }

    fn parse_query(&self, query: &str) -> HashMap<String, String> {
        let mut result = HashMap::new();
        for pair in query.split('&') {
            if let Some((key, value)) = pair.split_once('=') {
                result.insert(
                    urlencoding::decode(key).unwrap_or_default().to_string(),
                    urlencoding::decode(value).unwrap_or_default().to_string(),
                );
            }
        }
        result
    }

    fn notify_listeners(&self, route: &Route) {
        let listeners = self.listeners.lock().unwrap();
        for listener in listeners.iter() {
            listener(route);
        }
    }
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

/// File-based router that automatically discovers routes from filesystem
pub struct FileBasedRouter {
    /// Base directory for routes (e.g., "src/pages")
    base_dir: PathBuf,
    /// Underlying router
    router: Router,
}

impl FileBasedRouter {
    /// Create a new file-based router
    pub fn new<P: AsRef<Path>>(base_dir: P) -> Self {
        Self {
            base_dir: base_dir.as_ref().to_path_buf(),
            router: Router::new(),
        }
    }

    /// Scan directory and register routes
    pub fn scan(&mut self) -> Result<(), String> {
        let base_dir = self.base_dir.clone();
        self.scan_directory(&base_dir, "")?;
        Ok(())
    }

    /// Get the underlying router
    pub fn router(&self) -> &Router {
        &self.router
    }

    fn scan_directory(&mut self, dir: &Path, prefix: &str) -> Result<(), String> {
        if !dir.exists() {
            return Ok(()); // Directory doesn't exist yet, that's ok
        }

        let entries = std::fs::read_dir(dir).map_err(|e| e.to_string())?;

        for entry in entries {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            let file_name = entry.file_name().to_string_lossy().to_string();

            if path.is_dir() {
                // Recurse into subdirectories
                let new_prefix = if prefix.is_empty() {
                    format!("/{}", file_name)
                } else {
                    format!("{}/{}", prefix, file_name)
                };
                self.scan_directory(&path, &new_prefix)?;
            } else if path.is_file() {
                // Register file as route
                let route_path = self.file_to_route(&file_name, prefix);
                let handler = path.to_string_lossy().to_string();
                self.router.add_route(Route::new(route_path, handler));
            }
        }

        Ok(())
    }

    fn file_to_route(&self, file_name: &str, prefix: &str) -> String {
        // Convert file names to routes:
        // index.wj -> /
        // about.wj -> /about
        // users/[id].wj -> /users/:id
        // blog/[...slug].wj -> /blog/*slug

        let mut route = prefix.to_string();

        // Remove extension
        let name = file_name
            .strip_suffix(".wj")
            .or_else(|| file_name.strip_suffix(".rs"))
            .unwrap_or(file_name);

        if name == "index" {
            // index.wj -> /prefix (or / if no prefix)
            if route.is_empty() {
                route = "/".to_string();
            }
        } else if name.starts_with('[') && name.ends_with(']') {
            // Dynamic route: [id].wj -> /:id
            let param = &name[1..name.len() - 1];
            if let Some(stripped) = param.strip_prefix("...") {
                // Catch-all: [...slug].wj -> /*slug
                route = format!("{}/*{}", route, stripped);
            } else {
                // Regular param: [id].wj -> /:id
                route = format!("{}/:{}", route, param);
            }
        } else {
            // Static route: about.wj -> /about
            route = format!("{}/{}", route, name);
        }

        route
    }
}

/// Platform-specific route guard
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoutePlatform {
    Web,
    Desktop,
    Mobile,
    All,
}

/// Route guard for authentication, authorization, etc.
pub trait RouteGuard: Send + Sync {
    /// Check if navigation is allowed
    fn can_activate(&self, route: &Route) -> bool;

    /// Get redirect path if navigation is blocked
    fn redirect(&self) -> Option<String> {
        None
    }
}

/// Authentication guard example
pub struct AuthGuard {
    authenticated: Arc<Mutex<bool>>,
}

impl AuthGuard {
    pub fn new(authenticated: bool) -> Self {
        Self {
            authenticated: Arc::new(Mutex::new(authenticated)),
        }
    }

    pub fn set_authenticated(&self, value: bool) {
        *self.authenticated.lock().unwrap() = value;
    }
}

impl RouteGuard for AuthGuard {
    fn can_activate(&self, _route: &Route) -> bool {
        *self.authenticated.lock().unwrap()
    }

    fn redirect(&self) -> Option<String> {
        Some("/login".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_matches_static() {
        let route = Route::new("/about".to_string(), "AboutPage".to_string());
        assert!(route.matches("/about").is_some());
        assert!(route.matches("/contact").is_none());
    }

    #[test]
    fn test_route_matches_dynamic() {
        let route = Route::new("/users/:id".to_string(), "UserPage".to_string());
        let params = route.matches("/users/123").unwrap();
        assert_eq!(params.get("id"), Some(&"123".to_string()));
    }

    #[test]
    fn test_route_matches_multiple_params() {
        let route = Route::new("/posts/:category/:id".to_string(), "PostPage".to_string());
        let params = route.matches("/posts/tech/456").unwrap();
        assert_eq!(params.get("category"), Some(&"tech".to_string()));
        assert_eq!(params.get("id"), Some(&"456".to_string()));
    }

    #[test]
    fn test_router_navigate() {
        let router = Router::new();
        router.add_route(Route::new("/home".to_string(), "HomePage".to_string()));

        assert!(router.navigate("/home").is_ok());
        assert!(router.current().is_some());
        assert_eq!(router.current().unwrap().handler, "HomePage");
    }

    #[test]
    fn test_router_params() {
        let router = Router::new();
        router.add_route(Route::new("/users/:id".to_string(), "UserPage".to_string()));

        router.navigate("/users/789").unwrap();
        assert_eq!(router.param("id"), Some("789".to_string()));
    }

    #[test]
    fn test_router_query() {
        let router = Router::new();
        router.add_route(Route::new("/search".to_string(), "SearchPage".to_string()));

        router.navigate("/search?q=rust&page=2").unwrap();
        assert_eq!(router.query("q"), Some("rust".to_string()));
        assert_eq!(router.query("page"), Some("2".to_string()));
    }

    #[test]
    fn test_router_back() {
        let router = Router::new();
        router.add_route(Route::new("/home".to_string(), "HomePage".to_string()));
        router.add_route(Route::new("/about".to_string(), "AboutPage".to_string()));

        router.navigate("/home").unwrap();
        router.navigate("/about").unwrap();
        assert_eq!(router.current().unwrap().path, "/about");

        router.back().unwrap();
        assert_eq!(router.current().unwrap().path, "/home");
    }

    #[test]
    fn test_route_guard_authenticated() {
        let guard = AuthGuard::new(true);
        let route = Route::new("/dashboard".to_string(), "DashboardPage".to_string());
        assert!(guard.can_activate(&route));
    }

    #[test]
    fn test_route_guard_not_authenticated() {
        let guard = AuthGuard::new(false);
        let route = Route::new("/dashboard".to_string(), "DashboardPage".to_string());
        assert!(!guard.can_activate(&route));
        assert_eq!(guard.redirect(), Some("/login".to_string()));
    }

    #[test]
    fn test_file_to_route_index() {
        let router = FileBasedRouter::new("pages");
        assert_eq!(router.file_to_route("index.wj", ""), "/");
    }

    #[test]
    fn test_file_to_route_static() {
        let router = FileBasedRouter::new("pages");
        assert_eq!(router.file_to_route("about.wj", ""), "/about");
    }

    #[test]
    fn test_file_to_route_dynamic() {
        let router = FileBasedRouter::new("pages");
        assert_eq!(router.file_to_route("[id].wj", "/users"), "/users/:id");
    }

    #[test]
    fn test_file_to_route_catchall() {
        let router = FileBasedRouter::new("pages");
        assert_eq!(router.file_to_route("[...slug].wj", "/blog"), "/blog/*slug");
    }
}
