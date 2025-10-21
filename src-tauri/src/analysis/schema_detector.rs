use anyhow::Result;
use serde_json::Value as JsonValue;

/// Detector for MCP server schemas and configurations
pub struct SchemaDetector;

impl SchemaDetector {
    pub fn new() -> Self {
        Self
    }

    /// Detect server type from configuration
    pub fn detect_server_type(&self, config: &JsonValue) -> String {
        // Check for explicit server type
        if let Some(server_type) = config.get("type").and_then(|t| t.as_str()) {
            return server_type.to_string();
        }

        // Check for transport hints
        if config.get("stdio").is_some() {
            return "stdio".to_string();
        }

        if config.get("sse").is_some() || config.get("url").is_some() {
            return "sse".to_string();
        }

        if config.get("http").is_some() || config.get("port").is_some() {
            return "http".to_string();
        }

        // Default to stdio
        "stdio".to_string()
    }

    /// Validate MCP server configuration
    pub fn validate_config(&self, config: &JsonValue) -> Result<bool> {
        // Must have either command or url
        if config.get("command").is_none() && config.get("url").is_none() {
            return Ok(false);
        }

        // If has command, args should be array if present
        if config.get("command").is_some() {
            if let Some(args) = config.get("args") {
                if !args.is_array() {
                    return Ok(false);
                }
            }
        }

        // If has env, it should be an object
        if let Some(env) = config.get("env") {
            if !env.is_object() {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Extract capabilities from server metadata
    pub fn extract_capabilities(&self, metadata: &JsonValue) -> Vec<String> {
        let mut capabilities = Vec::new();

        if let Some(caps) = metadata.get("capabilities").and_then(|c| c.as_array()) {
            for cap in caps {
                if let Some(cap_str) = cap.as_str() {
                    capabilities.push(cap_str.to_string());
                }
            }
        }

        // Detect common capability patterns
        if metadata.get("tools").is_some() {
            capabilities.push("tools".to_string());
        }

        if metadata.get("prompts").is_some() {
            capabilities.push("prompts".to_string());
        }

        if metadata.get("resources").is_some() {
            capabilities.push("resources".to_string());
        }

        capabilities
    }
}

impl Default for SchemaDetector {
    fn default() -> Self {
        Self::new()
    }
}
