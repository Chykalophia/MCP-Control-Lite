use anyhow::{Context, Result};
use serde_json::Value as JsonValue;
use std::collections::HashMap;

use super::server_analyzer::{DetectedConfig, EnvVarConfig};

/// Parser for package.json files
pub struct PackageParser;

impl PackageParser {
    pub fn new() -> Self {
        Self
    }

    /// Fetch package.json from npm registry
    pub async fn fetch_npm_package(&self, package_name: &str) -> Result<String> {
        let url = format!("https://registry.npmjs.org/{}", package_name);

        let client = reqwest::Client::builder()
            .user_agent("MCP-Control/1.0")
            .build()?;

        let response = client.get(&url).send().await?;
        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to fetch package from npm: {}",
                response.status()
            ));
        }

        let npm_data: JsonValue = response.json().await?;

        // Get the latest version
        let latest_version = npm_data
            .get("dist-tags")
            .and_then(|t| t.get("latest"))
            .and_then(|v| v.as_str())
            .context("No latest version found")?;

        // Get the package.json for the latest version
        let package_json = npm_data
            .get("versions")
            .and_then(|v| v.get(latest_version))
            .context("Version not found")?;

        Ok(serde_json::to_string_pretty(package_json)?)
    }

    /// Fetch README from npm registry
    pub async fn fetch_npm_readme(&self, package_name: &str) -> Result<String> {
        let url = format!("https://registry.npmjs.org/{}", package_name);

        let client = reqwest::Client::builder()
            .user_agent("MCP-Control/1.0")
            .build()?;

        let response = client.get(&url).send().await?;
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Failed to fetch package from npm"));
        }

        let npm_data: JsonValue = response.json().await?;

        npm_data
            .get("readme")
            .and_then(|r| r.as_str())
            .map(|s| s.to_string())
            .context("No README found in package")
    }

    /// Parse package.json content
    pub fn parse_package_json(&self, content: &str) -> Result<DetectedConfig> {
        let package: JsonValue = serde_json::from_str(content)?;

        let name = package
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("unknown")
            .to_string();

        let description = package
            .get("description")
            .and_then(|d| d.as_str())
            .map(|s| s.to_string());

        let version = package
            .get("version")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let author = self.extract_author(&package);

        // Determine command and args
        let (command, args) = self.determine_command_and_args(&package, &name);

        // Extract environment variables from various sources
        let env = self.extract_env_vars(&package);

        // Get repository URL for docs
        let docs_url = self.extract_docs_url(&package);

        Ok(DetectedConfig {
            name,
            description,
            command,
            args,
            env,
            optional_args: Vec::new(),
            server_type: "stdio".to_string(),
            install_command: Some(format!("npm install {}",
                package.get("name").and_then(|n| n.as_str()).unwrap_or("unknown")
            )),
            docs_url,
            author,
            version,
        })
    }

    /// Extract author from package.json
    fn extract_author(&self, package: &JsonValue) -> Option<String> {
        if let Some(author) = package.get("author") {
            if let Some(name) = author.as_str() {
                return Some(name.to_string());
            } else if let Some(name) = author.get("name").and_then(|n| n.as_str()) {
                return Some(name.to_string());
            }
        }
        None
    }

    /// Determine command and arguments from package.json
    fn determine_command_and_args(&self, package: &JsonValue, package_name: &str) -> (String, Vec<String>) {
        // Check for bin field (executable)
        if let Some(bin) = package.get("bin") {
            if let Some(bin_path) = bin.as_str() {
                return ("npx".to_string(), vec!["-y".to_string(), package_name.to_string()]);
            } else if let Some(bin_obj) = bin.as_object() {
                if let Some((bin_name, _)) = bin_obj.iter().next() {
                    return ("npx".to_string(), vec!["-y".to_string(), package_name.to_string()]);
                }
            }
        }

        // Check for main field
        if let Some(main) = package.get("main").and_then(|m| m.as_str()) {
            if main.ends_with(".js") || main.ends_with(".mjs") {
                return ("node".to_string(), vec![main.to_string()]);
            }
        }

        // Check scripts for start or mcp
        if let Some(scripts) = package.get("scripts").and_then(|s| s.as_object()) {
            if scripts.contains_key("mcp") {
                return ("npm".to_string(), vec!["run".to_string(), "mcp".to_string()]);
            }
            if scripts.contains_key("start") {
                return ("npm".to_string(), vec!["start".to_string()]);
            }
        }

        // Default to npx
        ("npx".to_string(), vec!["-y".to_string(), package_name.to_string()])
    }

    /// Extract environment variables from package.json
    fn extract_env_vars(&self, package: &JsonValue) -> HashMap<String, EnvVarConfig> {
        let mut env_vars = HashMap::new();

        // Look for mcp configuration
        if let Some(mcp_config) = package.get("mcp").and_then(|m| m.as_object()) {
            if let Some(env) = mcp_config.get("env").and_then(|e| e.as_object()) {
                for (key, value) in env {
                    let config = if let Some(obj) = value.as_object() {
                        EnvVarConfig {
                            name: key.clone(),
                            description: obj.get("description")
                                .and_then(|d| d.as_str())
                                .map(|s| s.to_string()),
                            required: obj.get("required")
                                .and_then(|r| r.as_bool())
                                .unwrap_or(false),
                            default: obj.get("default")
                                .and_then(|d| d.as_str())
                                .map(|s| s.to_string()),
                            example: obj.get("example")
                                .and_then(|e| e.as_str())
                                .map(|s| s.to_string()),
                        }
                    } else {
                        EnvVarConfig {
                            name: key.clone(),
                            description: None,
                            required: false,
                            default: value.as_str().map(|s| s.to_string()),
                            example: None,
                        }
                    };
                    env_vars.insert(key.clone(), config);
                }
            }
        }

        // Look for configuration in keywords or description
        if let Some(keywords) = package.get("keywords").and_then(|k| k.as_array()) {
            for keyword in keywords {
                if let Some(kw) = keyword.as_str() {
                    if kw.to_uppercase().ends_with("_KEY") || kw.to_uppercase().ends_with("_TOKEN") {
                        env_vars.entry(kw.to_uppercase()).or_insert(EnvVarConfig {
                            name: kw.to_uppercase(),
                            description: Some(format!("{} (detected from keywords)", kw)),
                            required: false,
                            default: None,
                            example: None,
                        });
                    }
                }
            }
        }

        env_vars
    }

    /// Extract documentation URL
    fn extract_docs_url(&self, package: &JsonValue) -> Option<String> {
        // Try homepage first
        if let Some(homepage) = package.get("homepage").and_then(|h| h.as_str()) {
            return Some(homepage.to_string());
        }

        // Try repository
        if let Some(repository) = package.get("repository") {
            if let Some(url) = repository.as_str() {
                return Some(url.to_string());
            } else if let Some(url) = repository.get("url").and_then(|u| u.as_str()) {
                // Clean up git+https URLs
                let clean_url = url
                    .trim_start_matches("git+")
                    .trim_end_matches(".git");
                return Some(clean_url.to_string());
            }
        }

        None
    }
}

impl Default for PackageParser {
    fn default() -> Self {
        Self::new()
    }
}
