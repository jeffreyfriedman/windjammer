//! Module federation - share code between independent applications
//!
//! Implements Webpack Module Federation-like functionality for Windjammer.

use std::collections::HashMap;

/// Remote module configuration
#[derive(Debug, Clone)]
pub struct RemoteModule {
    /// Remote name
    pub name: String,
    /// Remote URL
    pub url: String,
    /// Exposed modules
    pub exposes: Vec<String>,
}

/// Module federation configuration
#[derive(Debug, Clone)]
pub struct FederationConfig {
    /// Application name
    pub name: String,
    /// Filename for the remote entry
    pub filename: String,
    /// Modules to expose
    pub exposes: HashMap<String, String>,
    /// Remote modules to consume
    pub remotes: Vec<RemoteModule>,
    /// Shared dependencies
    pub shared: Vec<String>,
}

impl Default for FederationConfig {
    fn default() -> Self {
        Self {
            name: "app".to_string(),
            filename: "remoteEntry.js".to_string(),
            exposes: HashMap::new(),
            remotes: Vec::new(),
            shared: Vec::new(),
        }
    }
}

/// Module federation runtime
pub struct ModuleFederation {
    config: FederationConfig,
}

impl ModuleFederation {
    /// Create a new module federation instance
    pub fn new(config: FederationConfig) -> Self {
        Self { config }
    }

    /// Generate remote entry point
    pub fn generate_remote_entry(&self) -> String {
        let mut entry = String::new();

        // Header
        entry.push_str(&format!(
            "// Windjammer Module Federation - Remote Entry: {}\n\n",
            self.config.name
        ));

        // Container initialization
        entry.push_str(&self.generate_container_init());

        // Expose modules
        entry.push_str(&self.generate_exposes());

        // Export container
        entry.push_str(&format!(
            "\nexport default {}Container;\n",
            self.config.name
        ));

        entry
    }

    /// Generate container initialization code
    fn generate_container_init(&self) -> String {
        format!(
            r#"const {}Container = {{
    init(shareScope) {{
        if (!this.__initialized) {{
            this.__shareScope = shareScope;
            this.__initialized = true;
        }}
    }},
    
    get(module) {{
        return this.__modules[module];
    }},
    
    __modules: {{}},
    __initialized: false,
    __shareScope: {{}}
}};

"#,
            self.config.name
        )
    }

    /// Generate module exposes code
    fn generate_exposes(&self) -> String {
        let mut code = String::new();

        for (expose_name, module_path) in &self.config.exposes {
            code.push_str(&format!(
                "{}Container.__modules['{}'] = () => import('{}');\n",
                self.config.name, expose_name, module_path
            ));
        }

        code
    }

    /// Generate consumer code for remotes
    pub fn generate_consumer_code(&self) -> String {
        let mut code = String::new();

        // Header
        code.push_str("// Windjammer Module Federation - Consumer\n\n");

        // Runtime
        code.push_str(&self.generate_federation_runtime());

        // Load remotes
        for remote in &self.config.remotes {
            code.push_str(&self.generate_remote_loader(remote));
        }

        code
    }

    /// Generate federation runtime
    fn generate_federation_runtime(&self) -> String {
        r#"const __wj_federation = {
    remotes: {},
    shared: {},
    
    async loadRemote(name, url) {
        if (this.remotes[name]) {
            return this.remotes[name];
        }
        
        try {
            const script = document.createElement('script');
            script.src = url;
            script.type = 'module';
            
            await new Promise((resolve, reject) => {
                script.onload = resolve;
                script.onerror = reject;
                document.head.appendChild(script);
            });
            
            // Get the remote container
            const container = window[name];
            if (!container) {
                throw new Error(`Remote container '${name}' not found`);
            }
            
            // Initialize with shared scope
            await container.init(this.shared);
            
            this.remotes[name] = container;
            return container;
        } catch (e) {
            console.error(`Failed to load remote: ${name}`, e);
            throw e;
        }
    },
    
    async getModule(remoteName, moduleName) {
        const container = await this.loadRemote(remoteName);
        const factory = await container.get(moduleName);
        const module = factory();
        return module;
    }
};

"#
        .to_string()
    }

    /// Generate remote loader code
    fn generate_remote_loader(&self, remote: &RemoteModule) -> String {
        format!(
            r#"// Load remote: {}
__wj_federation.loadRemote('{}', '{}').then(container => {{
    console.log('Remote {} loaded successfully');
}}).catch(err => {{
    console.error('Failed to load remote {}: ', err);
}});

"#,
            remote.name, remote.name, remote.url, remote.name, remote.name
        )
    }

    /// Generate shared dependencies code
    pub fn generate_shared_scope(&self) -> String {
        let mut code = String::new();

        code.push_str("// Shared Dependencies\n");
        code.push_str("const __wj_shared = {\n");

        for dep in &self.config.shared {
            code.push_str(&format!("    '{}': {{\n", dep));
            code.push_str("        loaded: false,\n");
            code.push_str(&format!("        get: () => import('{}')\n", dep));
            code.push_str("    },\n");
        }

        code.push_str("};\n\n");
        code.push_str("__wj_federation.shared = __wj_shared;\n\n");

        code
    }

    /// Generate import helper for federated modules
    pub fn generate_import_helper(&self) -> String {
        r#"// Import from federated module
async function importFederated(remoteName, moduleName) {
    try {
        const module = await __wj_federation.getModule(remoteName, moduleName);
        return module;
    } catch (e) {
        console.error(`Failed to import federated module: ${remoteName}/${moduleName}`, e);
        throw e;
    }
}

// Export helper
window.importFederated = importFederated;
"#
        .to_string()
    }
}

/// Generate module federation runtime
pub fn generate_federation_runtime(config: FederationConfig) -> String {
    let federation = ModuleFederation::new(config);
    let mut runtime = String::new();

    runtime.push_str(&federation.generate_remote_entry());
    runtime.push('\n');
    runtime.push_str(&federation.generate_consumer_code());
    runtime.push('\n');
    runtime.push_str(&federation.generate_shared_scope());
    runtime.push('\n');
    runtime.push_str(&federation.generate_import_helper());

    runtime
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_federation_config() {
        let config = FederationConfig::default();
        assert_eq!(config.name, "app");
        assert_eq!(config.filename, "remoteEntry.js");
    }

    #[test]
    fn test_remote_entry_generation() {
        let mut config = FederationConfig {
            name: "myapp".to_string(),
            ..Default::default()
        };
        config
            .exposes
            .insert("./Button".to_string(), "./src/Button.js".to_string());

        let federation = ModuleFederation::new(config);
        let entry = federation.generate_remote_entry();

        assert!(entry.contains("myappContainer"));
        assert!(entry.contains("./Button"));
    }

    #[test]
    fn test_consumer_code_generation() {
        let config = FederationConfig::default();
        let federation = ModuleFederation::new(config);
        let consumer = federation.generate_consumer_code();

        assert!(consumer.contains("__wj_federation"));
        assert!(consumer.contains("loadRemote"));
    }

    #[test]
    fn test_shared_scope_generation() {
        let mut config = FederationConfig::default();
        config.shared.push("react".to_string());
        config.shared.push("react-dom".to_string());

        let federation = ModuleFederation::new(config);
        let shared = federation.generate_shared_scope();

        assert!(shared.contains("react"));
        assert!(shared.contains("react-dom"));
    }
}
