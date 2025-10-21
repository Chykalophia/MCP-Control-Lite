use anyhow::{Context, Result};
use async_trait::async_trait;
use serde_json::Value as JsonValue;

use crate::detection::{McpServerConfig, ServerType, ApplicationProfile, ConfigFormat};
use super::{ApplicationAdapter, ExtractionResult, ApplicationResult};

/// Warp terminal application adapter
pub struct WarpAdapter;

impl WarpAdapter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ApplicationAdapter for WarpAdapter {
    async fn extract_server_configs(&self, config: &JsonValue) -> Result<ExtractionResult> {
        let mut servers = Vec::new();
        let mut messages = Vec::new();

        // Warp stores MCP configs in config.mcp.servers (similar to Cursor)
        if let Some(mcp_config) = config.get("mcp") {
            if let Some(mcp_servers) = mcp_config.get("servers").and_then(|v| v.as_object()) {
                for (name, server_config) in mcp_servers {
                    match self.parse_server_config(name, server_config) {
                        Ok(server) => servers.push(server),
                        Err(e) => {
                            messages.push(format!("Failed to parse server '{}': {}", name, e));
                        }
                    }
                }
            } else {
                messages.push("No servers section found in mcp configuration".to_string());
            }
        } else {
            messages.push("No mcp section found in configuration".to_string());
        }

        Ok(ExtractionResult {
            servers,
            messages,
            success: true,
        })
    }

    async fn apply_server_configs(&self, config: &JsonValue, servers: &[McpServerConfig]) -> Result<ApplicationResult> {
        let mut new_config = config.clone();
        let mut messages = Vec::new();

        // Ensure mcp.servers structure exists
        if new_config.get("mcp").is_none() {
            new_config["mcp"] = serde_json::json!({});
        }
        if new_config["mcp"].get("servers").is_none() {
            new_config["mcp"]["servers"] = serde_json::json!({});
        }

        let mcp_servers = new_config["mcp"]["servers"]
            .as_object_mut()
            .context("Failed to get mcp.servers as object")?;

        // Clear existing servers
        mcp_servers.clear();

        // Add new servers
        for server in servers {
            let server_config = self.format_server_config(server)?;
            mcp_servers.insert(server.name.clone(), server_config);
            messages.push(format!("Added server '{}'", server.name));
        }

        Ok(ApplicationResult {
            config: new_config,
            messages,
            success: true,
        })
    }

    async fn validate_config(&self, config: &JsonValue) -> Result<bool> {
        // Check if mcp.servers exists and is an object
        if let Some(mcp_config) = config.get("mcp") {
            if let Some(mcp_servers) = mcp_config.get("servers") {
                if !mcp_servers.is_object() {
                    return Ok(false);
                }

                // Validate each server configuration
                if let Some(servers) = mcp_servers.as_object() {
                    for (name, server_config) in servers {
                        if !self.validate_server_config(name, server_config) {
                            return Ok(false);
                        }
                    }
                }
            }
        }

        Ok(true)
    }

    fn get_supported_formats(&self) -> Vec<ConfigFormat> {
        vec![ConfigFormat::Json]
    }

    fn get_name(&self) -> &str {
        "warp"
    }

    fn can_handle(&self, profile: &ApplicationProfile) -> bool {
        profile.id == "warp"
    }
}

impl WarpAdapter {
    fn parse_server_config(&self, name: &str, config: &JsonValue) -> Result<McpServerConfig> {
        use crate::detection::{ServerMetadata, ConfigSource};

        let command = config.get("command")
            .and_then(|v| v.as_str())
            .context("Missing or invalid command")?;

        let args = config.get("args")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_default();

        let env = config.get("env")
            .and_then(|v| v.as_object())
            .map(|obj| {
                obj.iter()
                    .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                    .collect()
            })
            .unwrap_or_default();

        Ok(McpServerConfig {
            name: name.to_string(),
            command: Some(command.to_string()),
            args,
            env,
            cwd: None,
            server_type: ServerType::Stdio,
            metadata: ServerMetadata {
                description: None,
                version: None,
                author: None,
                capabilities: vec![],
                enabled: !config.get("disabled").and_then(|v| v.as_bool()).unwrap_or(false),
                source: ConfigSource::MainConfig,
            },
        })
    }

    fn format_server_config(&self, server: &McpServerConfig) -> Result<JsonValue> {
        let mut config = serde_json::json!({
            "command": server.command,
            "args": server.args
        });

        if !server.env.is_empty() {
            config["env"] = serde_json::json!(server.env);
        }

        if !server.metadata.enabled {
            config["disabled"] = serde_json::json!(true);
        }

        Ok(config)
    }

    fn validate_server_config(&self, _name: &str, config: &JsonValue) -> bool {
        // Must have command
        if config.get("command").and_then(|v| v.as_str()).is_none() {
            return false;
        }

        // Args must be array if present
        if let Some(args) = config.get("args") {
            if !args.is_array() {
                return false;
            }
        }

        // Env must be object if present
        if let Some(env) = config.get("env") {
            if !env.is_object() {
                return false;
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_extract_server_configs() {
        let adapter = WarpAdapter::new();
        let config = json!({
            "mcp": {
                "servers": {
                    "test-server": {
                        "command": "node",
                        "args": ["server.js"],
                        "env": {
                            "API_KEY": "your-api-key-here"
                        }
                    }
                }
            }
        });

        let result = adapter.extract_server_configs(&config).await.unwrap();
        assert!(result.success);
        assert_eq!(result.servers.len(), 1);
        assert_eq!(result.servers[0].name, "test-server");
        assert_eq!(result.servers[0].command, Some("node".to_string()));
    }

    #[tokio::test]
    async fn test_validate_config() {
        let adapter = WarpAdapter::new();
        let valid_config = json!({
            "mcp": {
                "servers": {
                    "test-server": {
                        "command": "node",
                        "args": ["server.js"]
                    }
                }
            }
        });

        let result = adapter.validate_config(&valid_config).await.unwrap();
        assert!(result);
    }
}
