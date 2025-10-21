use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value as JsonValue;

use crate::detection::{McpServerConfig, ApplicationProfile, ConfigFormat};

pub mod claude_desktop;
pub mod cursor;
pub mod amazon_q;
pub mod generic;
pub mod warp;
pub mod claude_code;
pub mod jetbrains;

/// Result of configuration extraction
#[derive(Debug, Clone)]
pub struct ExtractionResult {
    /// Extracted MCP server configurations
    pub servers: Vec<McpServerConfig>,
    /// Any warnings or issues encountered
    pub messages: Vec<String>,
    /// Whether extraction was successful
    pub success: bool,
}

/// Result of configuration application
#[derive(Debug, Clone)]
pub struct ApplicationResult {
    /// Modified configuration
    pub config: JsonValue,
    /// Any warnings or issues encountered
    pub messages: Vec<String>,
    /// Whether application was successful
    pub success: bool,
}

/// Application adapter trait for handling MCP configurations
#[async_trait]
pub trait ApplicationAdapter: Send + Sync {
    /// Extract MCP server configurations from application config
    async fn extract_server_configs(&self, config: &JsonValue) -> Result<ExtractionResult>;
    
    /// Apply MCP server configurations to application config
    async fn apply_server_configs(&self, config: &JsonValue, servers: &[McpServerConfig]) -> Result<ApplicationResult>;
    
    /// Validate application-specific configuration
    async fn validate_config(&self, config: &JsonValue) -> Result<bool>;
    
    /// Get supported configuration formats
    fn get_supported_formats(&self) -> Vec<ConfigFormat>;
    
    /// Get adapter name/identifier
    fn get_name(&self) -> &str;
    
    /// Check if this adapter can handle the given application profile
    fn can_handle(&self, profile: &ApplicationProfile) -> bool;
}

/// Factory for creating application adapters
pub struct AdapterFactory;

impl AdapterFactory {
    /// Create an adapter for the given application profile
    pub fn create_adapter(profile: &ApplicationProfile) -> Result<Box<dyn ApplicationAdapter>> {
        match profile.id.as_str() {
            "claude-desktop" => Ok(Box::new(claude_desktop::ClaudeDesktopAdapter::new())),
            "claude-code" => Ok(Box::new(claude_code::ClaudeCodeAdapter::new())),
            "cursor" => Ok(Box::new(cursor::CursorAdapter::new())),
            "amazon-q" => Ok(Box::new(amazon_q::AmazonQAdapter::new())),
            "warp" => Ok(Box::new(warp::WarpAdapter::new())),
            id if id.starts_with("jetbrains-") => Ok(Box::new(jetbrains::JetBrainsAdapter::new())),
            _ => Ok(Box::new(generic::GenericAdapter::new())),
        }
    }
    
    /// Get all available adapter types
    pub fn get_available_adapters() -> Vec<&'static str> {
        vec![
            "claude-desktop",
            "claude-code",
            "cursor",
            "amazon-q",
            "warp",
            "jetbrains-idea",
            "jetbrains-phpstorm",
            "jetbrains-webstorm",
            "jetbrains-pycharm",
            "generic"
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_adapter_factory_creation() {
        use crate::detection::{DetectionStrategy, DetectionMethod, ApplicationMetadata, ApplicationCategory};
        
        let profile = ApplicationProfile {
            id: "claude-desktop".to_string(),
            name: "Claude Desktop".to_string(),
            bundle_id: "com.anthropic.claude".to_string(),
            config_path: "test".to_string(),
            alt_config_paths: vec![],
            config_format: ConfigFormat::Json,
            executable_paths: vec![],
            alt_executable_paths: vec![],
            detection_strategy: DetectionStrategy {
                use_bundle_lookup: true,
                use_executable_check: true,
                use_config_check: true,
                use_spotlight: true,
                priority_order: vec![DetectionMethod::BundleLookup],
            },
            metadata: ApplicationMetadata {
                version: None,
                developer: "Test".to_string(),
                category: ApplicationCategory::ChatClient,
                mcp_version: "1.0".to_string(),
                notes: None,
                requires_permissions: false,
            },
        };
        
        let adapter = AdapterFactory::create_adapter(&profile);
        assert!(adapter.is_ok());
        assert_eq!(adapter.unwrap().get_name(), "claude-desktop");
    }
    
    #[test]
    fn test_available_adapters() {
        let adapters = AdapterFactory::get_available_adapters();
        assert!(adapters.contains(&"claude-desktop"));
        assert!(adapters.contains(&"cursor"));
        assert!(adapters.contains(&"amazon-q"));
        assert!(adapters.contains(&"generic"));
    }
}
