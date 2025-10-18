//! Tool for analyzing SSR and routing configurations

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Serialize, Deserialize)]
pub struct AnalyzeSsrRoutingArgs {
    pub code: String,
    pub analysis_type: String, // "ssr", "routing", "both"
    pub check_hydration: Option<bool>,
    pub check_seo: Option<bool>,
}

pub fn execute(args: Value) -> Result<Vec<Value>, String> {
    let parsed_args: AnalyzeSsrRoutingArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {}", e))?;

    let code = &parsed_args.code;
    let analysis_type = &parsed_args.analysis_type;
    let check_hydration = parsed_args.check_hydration.unwrap_or(false);
    let check_seo = parsed_args.check_seo.unwrap_or(false);

    let mut results = Vec::new();

    if analysis_type == "ssr" || analysis_type == "both" {
        results.push(json!({
            "text": "Performing SSR analysis on provided code.",
            "type": "info"
        }));
        if code.contains("SSRRenderer::new()") {
            results.push(json!({
                "text": "Detected SSRRenderer usage. Ensure proper hydration setup.",
                "type": "success"
            }));
            if check_hydration && !code.contains("hydrate_app()") {
                results.push(json!({
                    "text": "Warning: SSRRenderer found, but no explicit `hydrate_app()` call for client-side hydration. This might lead to re-rendering issues.",
                    "type": "warning"
                }));
            }
        } else {
            results.push(json!({
                "text": "No explicit SSRRenderer usage detected. If SSR is intended, ensure it's configured.",
                "type": "info"
            }));
        }
        if check_seo && !code.contains("<title>") && !code.contains("<meta name=\"description\"") {
            results.push(json!({
                "text": "SEO check: Missing <title> or <meta name=\"description\"> in SSR output. This is crucial for search engine visibility.",
                "type": "warning"
            }));
        }
    }

    if analysis_type == "routing" || analysis_type == "both" {
        results.push(json!({
            "text": "Performing routing analysis on provided code.",
            "type": "info"
        }));
        if code.contains("Router::new()") && code.contains("add_route") {
            results.push(json!({
                "text": "Detected Router and route definitions. Analyzing routes...",
                "type": "success"
            }));
            // Simple check for a root route
            if !code.contains(r#"router.add_route("/", "#) {
                results.push(json!({
                    "text": "Warning: No root route (`/`) defined. Users might hit a 404 on the base URL.",
                    "type": "warning"
                }));
            }
            // Check for a not-found handler
            if !code.contains("set_not_found") {
                results.push(json!({
                    "text": "Warning: No `set_not_found` handler defined for the router. Undefined routes will not be gracefully handled.",
                    "type": "warning"
                }));
            }
        } else {
            results.push(json!({
                "text": "No explicit Router usage detected. If client-side routing is intended, ensure it's configured.",
                "type": "info"
            }));
        }
    }

    if results.is_empty() {
        Ok(vec![json!({
            "text": "No specific SSR or routing patterns detected based on the provided code and analysis type.",
            "type": "info"
        })])
    } else {
        Ok(results)
    }
}
