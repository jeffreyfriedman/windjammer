//! Tool for analyzing SSR and routing configurations

use crate::protocol::{Tool, ToolDefinition, ToolResult};
use serde_json::{json, Value};

pub fn definition() -> ToolDefinition {
    ToolDefinition {
        name: "analyze_ssr_routing".to_string(),
        description: "Analyze Server-Side Rendering and routing configurations, suggest optimizations and best practices".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "code": {
                    "type": "string",
                    "description": "Windjammer code containing SSR/routing logic"
                },
                "analysis_type": {
                    "type": "string",
                    "description": "Type of analysis to perform",
                    "enum": ["ssr", "routing", "both"]
                },
                "check_hydration": {
                    "type": "boolean",
                    "description": "Check for proper hydration setup"
                },
                "check_seo": {
                    "type": "boolean",
                    "description": "Check for SEO best practices"
                }
            },
            "required": ["code"]
        }),
    }
}

pub fn execute(arguments: Value) -> ToolResult {
    let code = arguments["code"]
        .as_str()
        .ok_or("Missing 'code' parameter")?;

    let analysis_type = arguments["analysis_type"].as_str().unwrap_or("both");

    let check_hydration = arguments["check_hydration"].as_bool().unwrap_or(true);
    let check_seo = arguments["check_seo"].as_bool().unwrap_or(true);

    let mut issues = Vec::new();
    let mut suggestions = Vec::new();
    let mut best_practices = Vec::new();

    // SSR Analysis
    if analysis_type == "ssr" || analysis_type == "both" {
        // Check for SSRRenderer usage
        if code.contains("SSRRenderer") {
            best_practices.push("✓ Using SSRRenderer for server-side rendering".to_string());

            // Check for proper HTML document generation
            if !code.contains("render_to_document") && !code.contains("render_to_string") {
                issues.push(
                    "⚠ SSRRenderer should use render_to_document() or render_to_string()"
                        .to_string(),
                );
                suggestions.push(
                    "Use renderer.render_to_document(title, component) for full HTML documents"
                        .to_string(),
                );
            }
        } else {
            suggestions.push(
                "Consider using SSRRenderer for better SEO and initial load performance"
                    .to_string(),
            );
        }

        // Check for hydration setup
        if check_hydration {
            if code.contains("hydrate") || code.contains("Hydration") {
                best_practices.push("✓ Hydration setup detected".to_string());
            } else if code.contains("SSRRenderer") {
                issues.push("⚠ SSR detected but no hydration setup found".to_string());
                suggestions
                    .push("Add client-side hydration: use windjammer_ui.ssr.hydrate()".to_string());
            }
        }

        // Check for SEO elements
        if check_seo {
            if code.contains("<title>") || code.contains("title:") {
                best_practices.push("✓ Page title found".to_string());
            } else {
                issues.push("⚠ No page title found - important for SEO".to_string());
                suggestions
                    .push("Add page title in render_to_document(\"Your Title\", ...)".to_string());
            }

            if code.contains("meta") || code.contains("description") {
                best_practices.push("✓ Meta tags detected".to_string());
            } else {
                suggestions.push("Consider adding meta tags for better SEO".to_string());
            }
        }
    }

    // Routing Analysis
    if analysis_type == "routing" || analysis_type == "both" {
        // Check for Router usage
        if code.contains("Router") || code.contains("Route") {
            best_practices.push("✓ Using routing system".to_string());

            // Check for route definitions
            if code.contains("add_route") || code.contains("Route {") {
                best_practices.push("✓ Routes properly defined".to_string());
            } else {
                issues.push("⚠ Router detected but no routes defined".to_string());
                suggestions.push("Add routes using router.add_route(path, handler)".to_string());
            }

            // Check for dynamic routes
            if code.contains(":") && code.contains("/") {
                best_practices.push("✓ Dynamic routes detected (e.g., /user/:id)".to_string());
            }

            // Check for not found handler
            if code.contains("404") || code.contains("not_found") || code.contains("NotFound") {
                best_practices.push("✓ 404 handler found".to_string());
            } else {
                suggestions.push("Add a 404/NotFound handler for undefined routes".to_string());
            }

            // Check for navigation
            if code.contains("navigate") || code.contains("push") {
                best_practices.push("✓ Programmatic navigation implemented".to_string());
            }
        } else {
            suggestions.push("Consider adding routing for multi-page applications".to_string());
            suggestions.push("Import: use windjammer_ui.routing.{Router, Route}".to_string());
        }

        // Check for file-based routing
        if code.contains("FileRouter") || code.contains("file_based") {
            best_practices.push("✓ Using file-based routing (Next.js style)".to_string());
        }
    }

    // Performance suggestions
    if code.contains("SSRRenderer") && !code.contains("cache") {
        suggestions.push("Consider implementing SSR caching for better performance".to_string());
    }

    // Generate analysis report
    let mut report = String::from("# SSR & Routing Analysis Report\n\n");

    if !issues.is_empty() {
        report.push_str("## ⚠ Issues Found\n\n");
        for issue in &issues {
            report.push_str(&format!("- {}\n", issue));
        }
        report.push('\n');
    }

    if !best_practices.is_empty() {
        report.push_str("## ✓ Best Practices Followed\n\n");
        for practice in &best_practices {
            report.push_str(&format!("- {}\n", practice));
        }
        report.push('\n');
    }

    if !suggestions.is_empty() {
        report.push_str("## 💡 Suggestions\n\n");
        for suggestion in &suggestions {
            report.push_str(&format!("- {}\n", suggestion));
        }
        report.push('\n');
    }

    // Add code examples
    report.push_str("## Example: Complete SSR + Routing Setup\n\n");
    report.push_str("```windjammer\n");
    report.push_str("use windjammer_ui.prelude.*\n");
    report.push_str("use windjammer_ui.ssr.SSRRenderer\n");
    report.push_str("use windjammer_ui.routing.{Router, Route}\n\n");
    report.push_str("fn setup_app() {\n");
    report.push_str("    // Setup router\n");
    report.push_str("    let router = Router::new()\n");
    report.push_str("    router.add_route(\"/\", home_page)\n");
    report.push_str("    router.add_route(\"/about\", about_page)\n");
    report.push_str("    router.add_route(\"/user/:id\", user_page)\n");
    report.push_str("    router.set_not_found(not_found_page)\n\n");
    report.push_str("    // SSR on server\n");
    report.push_str("    let renderer = SSRRenderer::new()\n");
    report.push_str("    let html = renderer.render_to_document(\n");
    report.push_str("        \"My App\",\n");
    report.push_str("        app_component\n");
    report.push_str("    )\n\n");
    report.push_str("    // Hydrate on client\n");
    report.push_str("    windjammer_ui.ssr.hydrate(\"app\", app_component)\n");
    report.push_str("}\n");
    report.push_str("```\n");

    Ok(vec![json!({
        "type": "text",
        "text": report
    })])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyze_ssr_code() {
        let code = r#"
            use windjammer_ui.ssr.SSRRenderer
            
            let renderer = SSRRenderer::new()
            let html = renderer.render_to_document("My App", component)
            windjammer_ui.ssr.hydrate("app", component)
        "#;

        let args = json!({
            "code": code,
            "analysis_type": "ssr"
        });

        let result = execute(args);
        assert!(result.is_ok());

        let content = result.unwrap();
        let text = content[0]["text"].as_str().unwrap();
        assert!(text.contains("Best Practices"));
        assert!(text.contains("SSRRenderer"));
    }

    #[test]
    fn test_analyze_routing_code() {
        let code = r#"
            use windjammer_ui.routing.Router
            
            let router = Router::new()
            router.add_route("/", home)
            router.add_route("/about", about)
            router.set_not_found(not_found)
        "#;

        let args = json!({
            "code": code,
            "analysis_type": "routing"
        });

        let result = execute(args);
        assert!(result.is_ok());

        let content = result.unwrap();
        let text = content[0]["text"].as_str().unwrap();
        assert!(text.contains("Router"));
        assert!(text.contains("404 handler"));
    }
}
