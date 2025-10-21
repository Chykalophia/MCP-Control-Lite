use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration structure type for MCP servers
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConfigStructure {
    /// Direct mcpServers object (e.g., Claude Desktop, Amazon Q)
    DirectMcpServers,
    /// Nested mcp.servers object (e.g., Cursor, Warp)
    NestedMcpServers,
    /// Custom structure (requires special handling)
    Custom(String),
}

/// Represents a known MCP-enabled application with detection patterns
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ApplicationProfile {
    /// Unique identifier for the application
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// macOS bundle identifier
    pub bundle_id: String,
    /// Primary configuration file path (with ~ expansion support)
    pub config_path: String,
    /// Alternative configuration paths to check
    pub alt_config_paths: Vec<String>,
    /// Expected configuration file format
    pub config_format: ConfigFormat,
    /// Configuration structure type
    pub config_structure: ConfigStructure,
    /// Standard installation paths to check
    pub executable_paths: Vec<String>,
    /// Alternative installation paths
    pub alt_executable_paths: Vec<String>,
    /// Detection strategy preferences
    pub detection_strategy: DetectionStrategy,
    /// Application-specific metadata
    pub metadata: ApplicationMetadata,
}

impl ApplicationProfile {
    /// Check if this application uses nested mcp.servers structure
    pub fn uses_nested_config(&self) -> bool {
        matches!(self.config_structure, ConfigStructure::NestedMcpServers)
    }

    /// Get the JSON path to MCP servers configuration
    pub fn get_mcp_servers_path(&self) -> Vec<&str> {
        match &self.config_structure {
            ConfigStructure::DirectMcpServers => vec!["mcpServers"],
            ConfigStructure::NestedMcpServers => vec!["mcp", "servers"],
            ConfigStructure::Custom(_) => vec!["mcpServers"], // Default fallback
        }
    }

    /// Validate that a config file matches the declared structure
    ///
    /// Returns a result with validation details:
    /// - Ok(()) if structure matches
    /// - Err(message) with description if mismatch detected
    pub fn validate_config_structure(&self, config: &serde_json::Value) -> Result<(), String> {
        match &self.config_structure {
            ConfigStructure::DirectMcpServers => {
                // Should have mcpServers at root level
                let has_direct = config.get("mcpServers").is_some();
                let has_nested = config.get("mcp")
                    .and_then(|m| m.get("servers"))
                    .is_some();

                if !has_direct && has_nested {
                    return Err(format!(
                        "Application '{}' is configured as DirectMcpServers but config uses nested mcp.servers structure",
                        self.name
                    ));
                }

                if !has_direct && !has_nested {
                    // Neither structure found - might be empty config
                    log::debug!("No MCP servers configuration found in {} config", self.name);
                }

                Ok(())
            }
            ConfigStructure::NestedMcpServers => {
                // Should have mcp.servers nested structure
                let has_nested = config.get("mcp")
                    .and_then(|m| m.get("servers"))
                    .is_some();
                let has_direct = config.get("mcpServers").is_some();

                if !has_nested && has_direct {
                    return Err(format!(
                        "Application '{}' is configured as NestedMcpServers but config uses direct mcpServers structure",
                        self.name
                    ));
                }

                if !has_nested && !has_direct {
                    // Neither structure found - might be empty config
                    log::debug!("No MCP servers configuration found in {} config", self.name);
                }

                Ok(())
            }
            ConfigStructure::Custom(expected) => {
                // For custom structures, just log the expectation
                log::debug!("Application '{}' uses custom structure: {}", self.name, expected);
                Ok(())
            }
        }
    }
}

/// Configuration file formats supported by MCP applications
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConfigFormat {
    Json,
    Yaml,
    Toml,
    Plist,
    Custom(String),
}

/// Detection strategies for finding applications
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DetectionStrategy {
    /// Check for bundle ID using macOS APIs
    pub use_bundle_lookup: bool,
    /// Check for executable files in standard paths
    pub use_executable_check: bool,
    /// Check for configuration files
    pub use_config_check: bool,
    /// Use Spotlight/mdfind for advanced searching
    pub use_spotlight: bool,
    /// Priority order for detection methods
    pub priority_order: Vec<DetectionMethod>,
}

/// Individual detection methods
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DetectionMethod {
    BundleLookup,
    ExecutableCheck,
    ConfigCheck,
    SpotlightSearch,
}

/// Application-specific metadata
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ApplicationMetadata {
    /// Application version (if detectable)
    pub version: Option<String>,
    /// Developer/publisher name
    pub developer: String,
    /// Application category
    pub category: ApplicationCategory,
    /// MCP protocol version supported (defaults to "1.0")
    #[serde(default = "default_mcp_version")]
    pub mcp_version: String,
    /// Additional notes or special handling requirements
    pub notes: Option<String>,
    /// Whether this application requires special permissions
    #[serde(default)]
    pub requires_permissions: bool,
    /// Year the application was first released
    #[serde(default)]
    pub release_year: Option<u32>,
    /// Official documentation URL
    #[serde(default)]
    pub official_docs_url: Option<String>,
    /// MCP configuration documentation URL
    #[serde(default)]
    pub config_docs_url: Option<String>,
    /// Support/help URL
    #[serde(default)]
    pub support_url: Option<String>,
    /// Software license
    #[serde(default)]
    pub license: Option<String>,
    /// Supported platforms
    #[serde(default)]
    pub platforms: Vec<String>,
    /// Minimum version required for MCP support
    #[serde(default)]
    pub min_version: Option<String>,
}

fn default_mcp_version() -> String {
    "1.0".to_string()
}

/// Categories of MCP-enabled applications
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ApplicationCategory {
    #[serde(rename = "IDE")]
    IDE,
    #[serde(rename = "AIAssistant")]
    AIAssistant,
    #[serde(rename = "DeveloperTool")]
    DeveloperTool,
    #[serde(rename = "Terminal")]
    Terminal,
    #[serde(rename = "CodeEditor")]
    CodeEditor,
    #[serde(rename = "ChatClient")]
    ChatClient,
    #[serde(rename = "ProductivityTool")]
    ProductivityTool,
    Other(String),
}

/// Registry of known MCP-enabled applications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationRegistry {
    /// Map of application ID to profile
    pub applications: HashMap<String, ApplicationProfile>,
    /// Registry metadata
    pub metadata: RegistryMetadata,
}

/// Metadata about the application registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryMetadata {
    /// Registry version
    pub version: String,
    /// Last updated timestamp
    pub last_updated: chrono::DateTime<chrono::Utc>,
    /// Total number of applications
    pub application_count: usize,
}

impl ApplicationRegistry {
    /// Create a new registry with default known applications
    pub fn new() -> Self {
        let mut applications = HashMap::new();
        
        // Add known MCP-enabled applications
        applications.insert("claude-desktop".to_string(), Self::claude_desktop_profile());
        applications.insert("claude-code".to_string(), Self::claude_code_profile());
        applications.insert("cursor".to_string(), Self::cursor_profile());
        applications.insert("zed".to_string(), Self::zed_profile());
        applications.insert("vscode".to_string(), Self::vscode_profile());
        applications.insert("continue-dev".to_string(), Self::continue_dev_profile());
        applications.insert("amazon-q".to_string(), Self::amazon_q_profile());
        applications.insert("warp".to_string(), Self::warp_profile());
        applications.insert("jetbrains-idea".to_string(), Self::jetbrains_idea_profile());
        applications.insert("jetbrains-phpstorm".to_string(), Self::jetbrains_phpstorm_profile());
        applications.insert("jetbrains-webstorm".to_string(), Self::jetbrains_webstorm_profile());
        applications.insert("jetbrains-pycharm".to_string(), Self::jetbrains_pycharm_profile());
        
        let application_count = applications.len();
        
        Self {
            applications,
            metadata: RegistryMetadata {
                version: "1.0.0".to_string(),
                last_updated: chrono::Utc::now(),
                application_count,
            },
        }
    }

    /// Load registry from external JSON file
    ///
    /// Attempts to load application profiles from an external applications.json file.
    /// This allows for configuration without recompilation.
    pub fn from_json_file(path: &std::path::Path) -> anyhow::Result<Self> {
        use std::fs;

        let content = fs::read_to_string(path)?;
        let json: serde_json::Value = serde_json::from_str(&content)?;

        let mut applications = HashMap::new();

        if let Some(apps_array) = json.get("applications").and_then(|a| a.as_array()) {
            for app_json in apps_array {
                let profile: ApplicationProfile = serde_json::from_value(app_json.clone())?;
                applications.insert(profile.id.clone(), profile);
            }
        }

        let application_count = applications.len();
        let version = json.get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("1.0.0")
            .to_string();

        Ok(Self {
            applications,
            metadata: RegistryMetadata {
                version,
                last_updated: chrono::Utc::now(),
                application_count,
            },
        })
    }

    /// Create registry with automatic loading from external file if available
    ///
    /// Tries to load from these locations in order:
    /// 1. ./resources/applications.json (development)
    /// 2. Bundled resource (production)
    /// 3. Falls back to hardcoded profiles
    pub fn with_auto_load() -> Self {
        // Try development path first
        let dev_path = std::path::PathBuf::from("./resources/applications.json");
        if dev_path.exists() {
            if let Ok(registry) = Self::from_json_file(&dev_path) {
                log::info!("Loaded application registry from development path");
                return registry;
            }
        }

        // Try relative to src-tauri directory
        let src_tauri_path = std::path::PathBuf::from("./src-tauri/resources/applications.json");
        if src_tauri_path.exists() {
            if let Ok(registry) = Self::from_json_file(&src_tauri_path) {
                log::info!("Loaded application registry from src-tauri path");
                return registry;
            }
        }

        // Try config directory
        if let Some(config_dir) = dirs::config_dir() {
            let config_path = config_dir.join("mcp-control").join("applications.json");
            if config_path.exists() {
                if let Ok(registry) = Self::from_json_file(&config_path) {
                    log::info!("Loaded application registry from config directory");
                    return registry;
                }
            }
        }

        // Fall back to hardcoded profiles
        log::info!("Using hardcoded application profiles");
        Self::new()
    }

    /// Get Claude Desktop application profile
    fn claude_desktop_profile() -> ApplicationProfile {
        ApplicationProfile {
            id: "claude-desktop".to_string(),
            name: "Claude Desktop".to_string(),
            bundle_id: "com.anthropic.claude".to_string(),
            config_path: "~/Library/Application Support/Claude/claude_desktop_config.json".to_string(),
            alt_config_paths: vec![
                "~/.config/claude/claude_desktop_config.json".to_string(),
            ],
            config_format: ConfigFormat::Json,
            config_structure: ConfigStructure::DirectMcpServers,
            executable_paths: vec![
                "/Applications/Claude.app".to_string(),
            ],
            alt_executable_paths: vec![
                "~/Applications/Claude.app".to_string(),
            ],
            detection_strategy: DetectionStrategy {
                use_bundle_lookup: true,
                use_executable_check: true,
                use_config_check: true,
                use_spotlight: true,
                priority_order: vec![
                    DetectionMethod::BundleLookup,
                    DetectionMethod::ExecutableCheck,
                    DetectionMethod::ConfigCheck,
                ],
            },
            metadata: ApplicationMetadata {
                version: None,
                developer: "Anthropic".to_string(),
                category: ApplicationCategory::ChatClient,
                mcp_version: "1.0".to_string(),
                notes: Some("Primary MCP client from Anthropic".to_string()),
                requires_permissions: false,
            },
        }
    }
    
    /// Get Cursor application profile
    fn cursor_profile() -> ApplicationProfile {
        ApplicationProfile {
            id: "cursor".to_string(),
            name: "Cursor".to_string(),
            bundle_id: "com.cursor.Cursor".to_string(),
            config_path: "~/Library/Application Support/Cursor/User/settings.json".to_string(),
            alt_config_paths: vec![
                "~/.config/cursor/settings.json".to_string(),
                "~/Library/Application Support/Cursor/User/globalStorage/settings.json".to_string(),
            ],
            config_format: ConfigFormat::Json,
            config_structure: ConfigStructure::NestedMcpServers,
            executable_paths: vec![
                "/Applications/Cursor.app".to_string(),
            ],
            alt_executable_paths: vec![
                "~/Applications/Cursor.app".to_string(),
                "/usr/local/bin/cursor".to_string(),
            ],
            detection_strategy: DetectionStrategy {
                use_bundle_lookup: true,
                use_executable_check: true,
                use_config_check: true,
                use_spotlight: true,
                priority_order: vec![
                    DetectionMethod::BundleLookup,
                    DetectionMethod::ExecutableCheck,
                    DetectionMethod::ConfigCheck,
                ],
            },
            metadata: ApplicationMetadata {
                version: None,
                developer: "Cursor Team".to_string(),
                category: ApplicationCategory::CodeEditor,
                mcp_version: "1.0".to_string(),
                notes: Some("AI-powered code editor with MCP support".to_string()),
                requires_permissions: false,
            },
        }
    }
    
    /// Get Zed application profile
    fn zed_profile() -> ApplicationProfile {
        ApplicationProfile {
            id: "zed".to_string(),
            name: "Zed".to_string(),
            bundle_id: "dev.zed.Zed".to_string(),
            config_path: "~/Library/Application Support/Zed/settings.json".to_string(),
            alt_config_paths: vec![
                "~/.config/zed/settings.json".to_string(),
            ],
            config_format: ConfigFormat::Json,
            config_structure: ConfigStructure::DirectMcpServers,
            executable_paths: vec![
                "/Applications/Zed.app".to_string(),
            ],
            alt_executable_paths: vec![
                "~/Applications/Zed.app".to_string(),
                "/usr/local/bin/zed".to_string(),
            ],
            detection_strategy: DetectionStrategy {
                use_bundle_lookup: true,
                use_executable_check: true,
                use_config_check: true,
                use_spotlight: true,
                priority_order: vec![
                    DetectionMethod::BundleLookup,
                    DetectionMethod::ExecutableCheck,
                    DetectionMethod::ConfigCheck,
                ],
            },
            metadata: ApplicationMetadata {
                version: None,
                developer: "Zed Industries".to_string(),
                category: ApplicationCategory::CodeEditor,
                mcp_version: "1.0".to_string(),
                notes: Some("High-performance collaborative code editor".to_string()),
                requires_permissions: false,
            },
        }
    }
    
    /// Get VS Code application profile
    fn vscode_profile() -> ApplicationProfile {
        ApplicationProfile {
            id: "vscode".to_string(),
            name: "Visual Studio Code".to_string(),
            bundle_id: "com.microsoft.VSCode".to_string(),
            config_path: "~/Library/Application Support/Code/User/settings.json".to_string(),
            alt_config_paths: vec![
                "~/.config/Code/User/settings.json".to_string(),
                "~/Library/Application Support/Code - Insiders/User/settings.json".to_string(),
            ],
            config_format: ConfigFormat::Json,
            config_structure: ConfigStructure::DirectMcpServers,
            executable_paths: vec![
                "/Applications/Visual Studio Code.app".to_string(),
            ],
            alt_executable_paths: vec![
                "~/Applications/Visual Studio Code.app".to_string(),
                "/usr/local/bin/code".to_string(),
                "/Applications/Visual Studio Code - Insiders.app".to_string(),
            ],
            detection_strategy: DetectionStrategy {
                use_bundle_lookup: true,
                use_executable_check: true,
                use_config_check: true,
                use_spotlight: true,
                priority_order: vec![
                    DetectionMethod::BundleLookup,
                    DetectionMethod::ExecutableCheck,
                    DetectionMethod::ConfigCheck,
                ],
            },
            metadata: ApplicationMetadata {
                version: None,
                developer: "Microsoft".to_string(),
                category: ApplicationCategory::CodeEditor,
                mcp_version: "1.0".to_string(),
                notes: Some("Popular code editor with MCP extension support".to_string()),
                requires_permissions: false,
            },
        }
    }
    
    /// Get Continue.dev application profile
    fn continue_dev_profile() -> ApplicationProfile {
        ApplicationProfile {
            id: "continue-dev".to_string(),
            name: "Continue.dev".to_string(),
            bundle_id: "dev.continue.continue".to_string(),
            config_path: "~/.continue/config.json".to_string(),
            alt_config_paths: vec![
                "~/Library/Application Support/continue/config.json".to_string(),
            ],
            config_format: ConfigFormat::Json,
            config_structure: ConfigStructure::DirectMcpServers,
            executable_paths: vec![
                "/Applications/Continue.app".to_string(),
            ],
            alt_executable_paths: vec![
                "~/Applications/Continue.app".to_string(),
            ],
            detection_strategy: DetectionStrategy {
                use_bundle_lookup: true,
                use_executable_check: true,
                use_config_check: true,
                use_spotlight: false, // Less common application
                priority_order: vec![
                    DetectionMethod::ConfigCheck,
                    DetectionMethod::ExecutableCheck,
                    DetectionMethod::BundleLookup,
                ],
            },
            metadata: ApplicationMetadata {
                version: None,
                developer: "Continue.dev".to_string(),
                category: ApplicationCategory::IDE,
                mcp_version: "1.0".to_string(),
                notes: Some("AI coding assistant with MCP integration".to_string()),
                requires_permissions: false,
            },
        }
    }
    
    /// Get Amazon Q Developer application profile
    fn amazon_q_profile() -> ApplicationProfile {
        ApplicationProfile {
            id: "amazon-q".to_string(),
            name: "Amazon Q Developer".to_string(),
            bundle_id: "com.amazon.q.developer".to_string(),
            config_path: "~/.aws/amazonq/mcp.json".to_string(),
            alt_config_paths: vec![
                "~/.aws/q/config.json".to_string(),
                "~/Library/Application Support/Amazon Q/config.json".to_string(),
            ],
            config_format: ConfigFormat::Json,
            config_structure: ConfigStructure::DirectMcpServers,
            executable_paths: vec![
                "/Applications/Amazon Q.app".to_string(),
            ],
            alt_executable_paths: vec![
                "~/Applications/Amazon Q.app".to_string(),
                "/usr/local/bin/q".to_string(),
            ],
            detection_strategy: DetectionStrategy {
                use_bundle_lookup: true,
                use_executable_check: true,
                use_config_check: true,
                use_spotlight: true,
                priority_order: vec![
                    DetectionMethod::BundleLookup,
                    DetectionMethod::ExecutableCheck,
                    DetectionMethod::ConfigCheck,
                ],
            },
            metadata: ApplicationMetadata {
                version: None,
                developer: "Amazon Web Services".to_string(),
                category: ApplicationCategory::IDE,
                mcp_version: "1.0".to_string(),
                notes: Some("AWS AI coding assistant with MCP support (global settings only)".to_string()),
                requires_permissions: false,
            },
        }
    }

    /// Get Warp terminal application profile
    fn warp_profile() -> ApplicationProfile {
        ApplicationProfile {
            id: "warp".to_string(),
            name: "Warp".to_string(),
            bundle_id: "dev.warp.Warp-Stable".to_string(),
            config_path: "~/.warp/mcp_config.json".to_string(),
            alt_config_paths: vec![
                "~/Library/Application Support/warp/mcp_config.json".to_string(),
                "~/.config/warp/mcp_config.json".to_string(),
            ],
            config_format: ConfigFormat::Json,
            config_structure: ConfigStructure::NestedMcpServers,
            executable_paths: vec![
                "/Applications/Warp.app".to_string(),
            ],
            alt_executable_paths: vec![
                "~/Applications/Warp.app".to_string(),
                "/usr/local/bin/warp".to_string(),
            ],
            detection_strategy: DetectionStrategy {
                use_bundle_lookup: true,
                use_executable_check: true,
                use_config_check: true,
                use_spotlight: true,
                priority_order: vec![
                    DetectionMethod::BundleLookup,
                    DetectionMethod::ExecutableCheck,
                    DetectionMethod::ConfigCheck,
                ],
            },
            metadata: ApplicationMetadata {
                version: None,
                developer: "Warp".to_string(),
                category: ApplicationCategory::ProductivityTool,
                mcp_version: "1.0".to_string(),
                notes: Some("Modern terminal with AI integration and MCP support".to_string()),
                requires_permissions: false,
            },
        }
    }

    /// Get Claude Code CLI application profile
    fn claude_code_profile() -> ApplicationProfile {
        ApplicationProfile {
            id: "claude-code".to_string(),
            name: "Claude Code".to_string(),
            bundle_id: "com.anthropic.claude-code".to_string(),
            config_path: "~/.claude/config.json".to_string(),
            alt_config_paths: vec![
                "~/.config/claude-code/config.json".to_string(),
                "~/Library/Application Support/Claude Code/config.json".to_string(),
            ],
            config_format: ConfigFormat::Json,
            config_structure: ConfigStructure::DirectMcpServers,
            executable_paths: vec![
                "/usr/local/bin/claude".to_string(),
                "/opt/homebrew/bin/claude".to_string(),
            ],
            alt_executable_paths: vec![
                "~/.local/bin/claude".to_string(),
                "/usr/bin/claude".to_string(),
            ],
            detection_strategy: DetectionStrategy {
                use_bundle_lookup: false,
                use_executable_check: true,
                use_config_check: true,
                use_spotlight: false,
                priority_order: vec![
                    DetectionMethod::ConfigCheck,
                    DetectionMethod::ExecutableCheck,
                ],
            },
            metadata: ApplicationMetadata {
                version: None,
                developer: "Anthropic".to_string(),
                category: ApplicationCategory::CodeEditor,
                mcp_version: "1.0".to_string(),
                notes: Some("Claude's official CLI tool with MCP support".to_string()),
                requires_permissions: false,
            },
        }
    }

    /// Get IntelliJ IDEA application profile
    fn jetbrains_idea_profile() -> ApplicationProfile {
        ApplicationProfile {
            id: "jetbrains-idea".to_string(),
            name: "IntelliJ IDEA".to_string(),
            bundle_id: "com.jetbrains.intellij".to_string(),
            config_path: "~/Library/Application Support/JetBrains/IntelliJIdea/mcp_settings.json".to_string(),
            alt_config_paths: vec![
                "~/.config/JetBrains/IntelliJIdea/mcp_settings.json".to_string(),
                "~/Library/Application Support/JetBrains/IdeaIC/mcp_settings.json".to_string(),
            ],
            config_format: ConfigFormat::Json,
            config_structure: ConfigStructure::NestedMcpServers,
            executable_paths: vec![
                "/Applications/IntelliJ IDEA.app".to_string(),
            ],
            alt_executable_paths: vec![
                "~/Applications/IntelliJ IDEA.app".to_string(),
                "/Applications/IntelliJ IDEA CE.app".to_string(),
                "/usr/local/bin/idea".to_string(),
            ],
            detection_strategy: DetectionStrategy {
                use_bundle_lookup: true,
                use_executable_check: true,
                use_config_check: true,
                use_spotlight: true,
                priority_order: vec![
                    DetectionMethod::BundleLookup,
                    DetectionMethod::ExecutableCheck,
                    DetectionMethod::ConfigCheck,
                ],
            },
            metadata: ApplicationMetadata {
                version: None,
                developer: "JetBrains".to_string(),
                category: ApplicationCategory::IDE,
                mcp_version: "1.0".to_string(),
                notes: Some("Java IDE with MCP plugin support".to_string()),
                requires_permissions: false,
            },
        }
    }

    /// Get PHPStorm application profile
    fn jetbrains_phpstorm_profile() -> ApplicationProfile {
        ApplicationProfile {
            id: "jetbrains-phpstorm".to_string(),
            name: "PHPStorm".to_string(),
            bundle_id: "com.jetbrains.phpstorm".to_string(),
            config_path: "~/Library/Application Support/JetBrains/PhpStorm/mcp_settings.json".to_string(),
            alt_config_paths: vec![
                "~/.config/JetBrains/PhpStorm/mcp_settings.json".to_string(),
            ],
            config_format: ConfigFormat::Json,
            config_structure: ConfigStructure::NestedMcpServers,
            executable_paths: vec![
                "/Applications/PhpStorm.app".to_string(),
            ],
            alt_executable_paths: vec![
                "~/Applications/PhpStorm.app".to_string(),
                "/usr/local/bin/phpstorm".to_string(),
            ],
            detection_strategy: DetectionStrategy {
                use_bundle_lookup: true,
                use_executable_check: true,
                use_config_check: true,
                use_spotlight: true,
                priority_order: vec![
                    DetectionMethod::BundleLookup,
                    DetectionMethod::ExecutableCheck,
                    DetectionMethod::ConfigCheck,
                ],
            },
            metadata: ApplicationMetadata {
                version: None,
                developer: "JetBrains".to_string(),
                category: ApplicationCategory::IDE,
                mcp_version: "1.0".to_string(),
                notes: Some("PHP IDE with MCP plugin support".to_string()),
                requires_permissions: false,
            },
        }
    }

    /// Get WebStorm application profile
    fn jetbrains_webstorm_profile() -> ApplicationProfile {
        ApplicationProfile {
            id: "jetbrains-webstorm".to_string(),
            name: "WebStorm".to_string(),
            bundle_id: "com.jetbrains.webstorm".to_string(),
            config_path: "~/Library/Application Support/JetBrains/WebStorm/mcp_settings.json".to_string(),
            alt_config_paths: vec![
                "~/.config/JetBrains/WebStorm/mcp_settings.json".to_string(),
            ],
            config_format: ConfigFormat::Json,
            config_structure: ConfigStructure::NestedMcpServers,
            executable_paths: vec![
                "/Applications/WebStorm.app".to_string(),
            ],
            alt_executable_paths: vec![
                "~/Applications/WebStorm.app".to_string(),
                "/usr/local/bin/webstorm".to_string(),
            ],
            detection_strategy: DetectionStrategy {
                use_bundle_lookup: true,
                use_executable_check: true,
                use_config_check: true,
                use_spotlight: true,
                priority_order: vec![
                    DetectionMethod::BundleLookup,
                    DetectionMethod::ExecutableCheck,
                    DetectionMethod::ConfigCheck,
                ],
            },
            metadata: ApplicationMetadata {
                version: None,
                developer: "JetBrains".to_string(),
                category: ApplicationCategory::IDE,
                mcp_version: "1.0".to_string(),
                notes: Some("JavaScript IDE with MCP plugin support".to_string()),
                requires_permissions: false,
            },
        }
    }

    /// Get PyCharm application profile
    fn jetbrains_pycharm_profile() -> ApplicationProfile {
        ApplicationProfile {
            id: "jetbrains-pycharm".to_string(),
            name: "PyCharm".to_string(),
            bundle_id: "com.jetbrains.pycharm".to_string(),
            config_path: "~/Library/Application Support/JetBrains/PyCharm/mcp_settings.json".to_string(),
            alt_config_paths: vec![
                "~/.config/JetBrains/PyCharm/mcp_settings.json".to_string(),
                "~/Library/Application Support/JetBrains/PyCharmCE/mcp_settings.json".to_string(),
            ],
            config_format: ConfigFormat::Json,
            config_structure: ConfigStructure::NestedMcpServers,
            executable_paths: vec![
                "/Applications/PyCharm.app".to_string(),
            ],
            alt_executable_paths: vec![
                "~/Applications/PyCharm.app".to_string(),
                "/Applications/PyCharm CE.app".to_string(),
                "/usr/local/bin/pycharm".to_string(),
            ],
            detection_strategy: DetectionStrategy {
                use_bundle_lookup: true,
                use_executable_check: true,
                use_config_check: true,
                use_spotlight: true,
                priority_order: vec![
                    DetectionMethod::BundleLookup,
                    DetectionMethod::ExecutableCheck,
                    DetectionMethod::ConfigCheck,
                ],
            },
            metadata: ApplicationMetadata {
                version: None,
                developer: "JetBrains".to_string(),
                category: ApplicationCategory::IDE,
                mcp_version: "1.0".to_string(),
                notes: Some("Python IDE with MCP plugin support".to_string()),
                requires_permissions: false,
            },
        }
    }
    
    /// Add a new application profile to the registry
    pub fn add_application(&mut self, profile: ApplicationProfile) {
        self.applications.insert(profile.id.clone(), profile);
        self.metadata.application_count = self.applications.len();
        self.metadata.last_updated = chrono::Utc::now();
    }
    
    /// Remove an application profile from the registry
    pub fn remove_application(&mut self, id: &str) -> Option<ApplicationProfile> {
        let result = self.applications.remove(id);
        if result.is_some() {
            self.metadata.application_count = self.applications.len();
            self.metadata.last_updated = chrono::Utc::now();
        }
        result
    }
    
    /// Get an application profile by ID
    pub fn get_application(&self, id: &str) -> Option<&ApplicationProfile> {
        self.applications.get(id)
    }
    
    /// Get all application profiles
    pub fn get_all_applications(&self) -> Vec<&ApplicationProfile> {
        self.applications.values().collect()
    }
    
    /// Get applications by category
    pub fn get_applications_by_category(&self, category: &ApplicationCategory) -> Vec<&ApplicationProfile> {
        self.applications
            .values()
            .filter(|app| &app.metadata.category == category)
            .collect()
    }
    
    /// Update registry metadata
    pub fn update_metadata(&mut self) {
        self.metadata.application_count = self.applications.len();
        self.metadata.last_updated = chrono::Utc::now();
    }
}

impl Default for ApplicationRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_application_registry_creation() {
        let registry = ApplicationRegistry::new();
        assert!(!registry.applications.is_empty());
        assert_eq!(registry.metadata.application_count, registry.applications.len());
    }

    #[test]
    fn test_claude_desktop_profile() {
        let registry = ApplicationRegistry::new();
        let claude = registry.get_application("claude-desktop").unwrap();
        
        assert_eq!(claude.name, "Claude Desktop");
        assert_eq!(claude.bundle_id, "com.anthropic.claude");
        assert_eq!(claude.config_format, ConfigFormat::Json);
        assert!(claude.detection_strategy.use_bundle_lookup);
    }

    #[test]
    fn test_add_remove_application() {
        let mut registry = ApplicationRegistry::new();
        let initial_count = registry.metadata.application_count;
        
        let custom_app = ApplicationProfile {
            id: "test-app".to_string(),
            name: "Test App".to_string(),
            bundle_id: "com.test.app".to_string(),
            config_path: "~/test/config.json".to_string(),
            alt_config_paths: vec![],
            config_format: ConfigFormat::Json,
            executable_paths: vec!["/Applications/Test.app".to_string()],
            alt_executable_paths: vec![],
            detection_strategy: DetectionStrategy {
                use_bundle_lookup: true,
                use_executable_check: true,
                use_config_check: true,
                use_spotlight: false,
                priority_order: vec![DetectionMethod::BundleLookup],
            },
            metadata: ApplicationMetadata {
                version: None,
                developer: "Test Developer".to_string(),
                category: ApplicationCategory::Other("Test".to_string()),
                mcp_version: "1.0".to_string(),
                notes: None,
                requires_permissions: false,
            },
        };
        
        registry.add_application(custom_app);
        assert_eq!(registry.metadata.application_count, initial_count + 1);
        assert!(registry.get_application("test-app").is_some());
        
        let removed = registry.remove_application("test-app");
        assert!(removed.is_some());
        assert_eq!(registry.metadata.application_count, initial_count);
        assert!(registry.get_application("test-app").is_none());
    }

    #[test]
    fn test_get_applications_by_category() {
        let registry = ApplicationRegistry::new();
        let code_editors = registry.get_applications_by_category(&ApplicationCategory::CodeEditor);
        
        assert!(!code_editors.is_empty());
        assert!(code_editors.iter().any(|app| app.id == "cursor"));
        assert!(code_editors.iter().any(|app| app.id == "zed"));
    }

    #[test]
    fn test_detection_strategy_serialization() {
        let strategy = DetectionStrategy {
            use_bundle_lookup: true,
            use_executable_check: false,
            use_config_check: true,
            use_spotlight: false,
            priority_order: vec![
                DetectionMethod::BundleLookup,
                DetectionMethod::ConfigCheck,
            ],
        };
        
        let serialized = serde_json::to_string(&strategy).unwrap();
        let deserialized: DetectionStrategy = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(strategy, deserialized);
    }
}
