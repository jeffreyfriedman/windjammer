//! Differential loading - modern + legacy bundles
//!
//! Generates two bundles: modern ES2015+ and legacy ES5 for older browsers.

/// Target browser configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowserTarget {
    /// Modern browsers (ES2015+): Chrome 60+, Firefox 60+, Safari 12+, Edge 79+
    Modern,
    /// Legacy browsers (ES5): IE 11+, older Chrome/Firefox/Safari
    Legacy,
}

/// Differential loading configuration
#[derive(Debug, Clone)]
pub struct DifferentialConfig {
    /// Generate modern bundle
    pub generate_modern: bool,
    /// Generate legacy bundle
    pub generate_legacy: bool,
    /// Module type for modern bundle
    pub modern_module_type: String,
    /// Module type for legacy bundle
    pub legacy_module_type: String,
}

impl Default for DifferentialConfig {
    fn default() -> Self {
        Self {
            generate_modern: true,
            generate_legacy: true,
            modern_module_type: "es".to_string(),
            legacy_module_type: "system".to_string(),
        }
    }
}

/// Bundle variant
#[derive(Debug, Clone)]
pub struct BundleVariant {
    /// Bundle code
    pub code: String,
    /// Target
    pub target: BrowserTarget,
    /// File name
    pub filename: String,
}

/// Differential loader
pub struct DifferentialLoader {
    config: DifferentialConfig,
}

impl DifferentialLoader {
    /// Create a new differential loader
    pub fn new(config: DifferentialConfig) -> Self {
        Self { config }
    }

    /// Generate bundles for different browser targets
    pub fn generate_bundles(&self, modern_code: &str, base_filename: &str) -> Vec<BundleVariant> {
        let mut bundles = Vec::new();

        if self.config.generate_modern {
            bundles.push(BundleVariant {
                code: self.generate_modern_bundle(modern_code),
                target: BrowserTarget::Modern,
                filename: format!("{}.modern.js", base_filename),
            });
        }

        if self.config.generate_legacy {
            bundles.push(BundleVariant {
                code: self.generate_legacy_bundle(modern_code),
                target: BrowserTarget::Legacy,
                filename: format!("{}.legacy.js", base_filename),
            });
        }

        bundles
    }

    /// Generate modern bundle (ES2015+)
    fn generate_modern_bundle(&self, code: &str) -> String {
        let mut bundle = String::new();

        // Add modern JavaScript features header
        bundle.push_str("// Windjammer Modern Bundle (ES2015+)\n");
        bundle.push_str("// Chrome 60+, Firefox 60+, Safari 12+, Edge 79+\n\n");

        // Modern code can use:
        // - Arrow functions
        // - Classes
        // - Template literals
        // - Destructuring
        // - Promises
        // - Async/await
        // - Modules (import/export)

        bundle.push_str(code);
        bundle
    }

    /// Generate legacy bundle (ES5)
    fn generate_legacy_bundle(&self, code: &str) -> String {
        let mut bundle = String::new();

        // Add legacy JavaScript header
        bundle.push_str("// Windjammer Legacy Bundle (ES5)\n");
        bundle.push_str("// IE 11+, older browsers\n\n");

        // Transpile modern features to ES5
        let transpiled = self.transpile_to_es5(code);

        // Add polyfills
        bundle.push_str(&self.generate_legacy_polyfills());
        bundle.push('\n');
        bundle.push_str(&transpiled);

        bundle
    }

    /// Transpile modern JavaScript to ES5
    fn transpile_to_es5(&self, code: &str) -> String {
        let mut result = code.to_string();

        // Arrow functions => regular functions
        // (x) => x + 1  =>  function(x) { return x + 1; }
        result = result.replace("() =>", "function()");
        result = result.replace(") =>", ") { return");

        // Const/let => var
        result = result.replace("const ", "var ");
        result = result.replace("let ", "var ");

        // Template literals => string concatenation (simplified)
        // This is a simplified version; real transpilation is much more complex
        result = result.replace("`${", "\" + (");
        result = result.replace("}`", ") + \"");
        result = result.replace("`", "\"");

        result
    }

    /// Generate polyfills for legacy browsers
    fn generate_legacy_polyfills(&self) -> String {
        r#"// Legacy Polyfills
if (!Array.prototype.includes) {
    Array.prototype.includes = function(item) {
        return this.indexOf(item) !== -1;
    };
}

if (!String.prototype.startsWith) {
    String.prototype.startsWith = function(search) {
        return this.substr(0, search.length) === search;
    };
}

if (!String.prototype.endsWith) {
    String.prototype.endsWith = function(search) {
        return this.substr(-search.length) === search;
    };
}

if (!Object.assign) {
    Object.assign = function(target) {
        for (var i = 1; i < arguments.length; i++) {
            var source = arguments[i];
            for (var key in source) {
                if (source.hasOwnProperty(key)) {
                    target[key] = source[key];
                }
            }
        }
        return target;
    };
}
"#
        .to_string()
    }

    /// Generate HTML script loader for differential loading
    pub fn generate_html_loader(&self, base_filename: &str) -> String {
        format!(
            r#"<!-- Windjammer Differential Loading -->
<script type="module">
    // Modern browsers (supports ES modules)
    import './{}.modern.js';
</script>
<script nomodule>
    // Legacy browsers (fallback)
    document.write('<script src="{}.legacy.js"><\/script>');
</script>
"#,
            base_filename, base_filename
        )
    }
}

/// Generate bundles for differential loading
pub fn generate_differential_bundles(
    code: &str,
    base_filename: &str,
    config: DifferentialConfig,
) -> Vec<BundleVariant> {
    let loader = DifferentialLoader::new(config);
    loader.generate_bundles(code, base_filename)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_differential_config() {
        let config = DifferentialConfig::default();
        assert!(config.generate_modern);
        assert!(config.generate_legacy);
    }

    #[test]
    fn test_generate_bundles() {
        let code = "const x = () => 42;";
        let config = DifferentialConfig::default();
        let loader = DifferentialLoader::new(config);
        let bundles = loader.generate_bundles(code, "app");

        assert_eq!(bundles.len(), 2);
        assert_eq!(bundles[0].target, BrowserTarget::Modern);
        assert_eq!(bundles[1].target, BrowserTarget::Legacy);
    }

    #[test]
    fn test_transpile_to_es5() {
        let code = "const x = 42;";
        let config = DifferentialConfig::default();
        let loader = DifferentialLoader::new(config);
        let transpiled = loader.transpile_to_es5(code);

        assert!(transpiled.contains("var"));
        assert!(!transpiled.contains("const"));
    }

    #[test]
    fn test_html_loader_generation() {
        let config = DifferentialConfig::default();
        let loader = DifferentialLoader::new(config);
        let html = loader.generate_html_loader("app");

        assert!(html.contains("type=\"module\""));
        assert!(html.contains("nomodule"));
        assert!(html.contains(".modern.js"));
        assert!(html.contains(".legacy.js"));
    }
}
