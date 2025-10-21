use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::collections::HashMap;

use super::{PackageParser, ReadmeParser, SchemaDetector};

/// Result of analyzing an MCP server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    /// Detected server configuration
    pub config: DetectedConfig,
    /// Confidence level (0.0 - 1.0)
    pub confidence: f32,
    /// Messages and warnings
    pub messages: Vec<String>,
    /// Whether analysis was successful
    pub success: bool,
}

/// Detected server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedConfig {
    /// Server name
    pub name: String,
    /// Description from package or README
    pub description: Option<String>,
    /// Command to run the server
    pub command: String,
    /// Required arguments
    pub args: Vec<String>,
    /// Detected environment variables
    pub env: HashMap<String, EnvVarConfig>,
    /// Optional arguments
    pub optional_args: Vec<ArgConfig>,
    /// Server type (stdio, sse, http)
    pub server_type: String,
    /// Installation command (if needed)
    pub install_command: Option<String>,
    /// Documentation URL
    pub docs_url: Option<String>,
    /// Author/Publisher
    pub author: Option<String>,
    /// Version
    pub version: Option<String>,
}

/// Environment variable configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvVarConfig {
    /// Variable name
    pub name: String,
    /// Description
    pub description: Option<String>,
    /// Is required?
    pub required: bool,
    /// Default value
    pub default: Option<String>,
    /// Example value
    pub example: Option<String>,
}

/// Argument configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArgConfig {
    /// Argument flag or name
    pub name: String,
    /// Description
    pub description: Option<String>,
    /// Default value
    pub default: Option<String>,
    /// Example value
    pub example: Option<String>,
}

/// Server analyzer for auto-detecting MCP server configuration
pub struct ServerAnalyzer {
    package_parser: PackageParser,
    readme_parser: ReadmeParser,
    schema_detector: SchemaDetector,
}

impl ServerAnalyzer {
    pub fn new() -> Self {
        Self {
            package_parser: PackageParser::new(),
            readme_parser: ReadmeParser::new(),
            schema_detector: SchemaDetector::new(),
        }
    }

    /// Analyze an MCP server package
    pub async fn analyze_package(&self, package_name: &str) -> Result<AnalysisResult> {
        let mut messages = Vec::new();
        messages.push(format!("Analyzing package: {}", package_name));

        // Try to analyze from npm package
        if package_name.starts_with("@") || package_name.contains('/') {
            return self.analyze_npm_package(package_name).await;
        }

        // Try to analyze from local path
        if PathBuf::from(package_name).exists() {
            return self.analyze_local_path(package_name).await;
        }

        // Try to analyze from URL
        if package_name.starts_with("http://") || package_name.starts_with("https://") {
            return self.analyze_url(package_name).await;
        }

        // Default to npm package analysis
        self.analyze_npm_package(package_name).await
    }

    /// Analyze npm package
    async fn analyze_npm_package(&self, package_name: &str) -> Result<AnalysisResult> {
        let mut messages = Vec::new();
        messages.push(format!("Fetching npm package info for: {}", package_name));

        // Fetch package.json from npm registry
        let package_json = self.package_parser.fetch_npm_package(package_name).await?;

        // Parse package.json
        let mut config = self.package_parser.parse_package_json(&package_json)?;
        messages.push("Parsed package.json successfully".to_string());

        // Try to fetch and parse README
        if let Ok(readme) = self.package_parser.fetch_npm_readme(package_name).await {
            if let Ok(readme_info) = self.readme_parser.parse_readme(&readme) {
                messages.push("Parsed README for additional configuration".to_string());

                // Merge README info with package.json info
                config = self.merge_configs(config, readme_info);
            }
        }

        // Calculate confidence based on available information
        let confidence = self.calculate_confidence(&config, &messages);

        Ok(AnalysisResult {
            config,
            confidence,
            messages,
            success: true,
        })
    }

    /// Analyze local path
    async fn analyze_local_path(&self, path: &str) -> Result<AnalysisResult> {
        let mut messages = Vec::new();
        messages.push(format!("Analyzing local path: {}", path));

        let path_buf = PathBuf::from(path);

        // Look for package.json
        let package_json_path = path_buf.join("package.json");
        let mut config = if package_json_path.exists() {
            let content = tokio::fs::read_to_string(&package_json_path).await?;
            messages.push("Found and parsed package.json".to_string());
            self.package_parser.parse_package_json(&content)?
        } else {
            // Create basic config from directory name
            DetectedConfig {
                name: path_buf.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string(),
                description: None,
                command: "node".to_string(),
                args: vec!["index.js".to_string()],
                env: HashMap::new(),
                optional_args: Vec::new(),
                server_type: "stdio".to_string(),
                install_command: None,
                docs_url: None,
                author: None,
                version: None,
            }
        };

        // Look for README
        for readme_name in &["README.md", "README.txt", "README"] {
            let readme_path = path_buf.join(readme_name);
            if readme_path.exists() {
                if let Ok(content) = tokio::fs::read_to_string(&readme_path).await {
                    if let Ok(readme_info) = self.readme_parser.parse_readme(&content) {
                        messages.push(format!("Parsed {} for configuration", readme_name));
                        config = self.merge_configs(config, readme_info);
                    }
                }
                break;
            }
        }

        let confidence = self.calculate_confidence(&config, &messages);

        Ok(AnalysisResult {
            config,
            confidence,
            messages,
            success: true,
        })
    }

    /// Analyze from URL (GitHub, etc.)
    async fn analyze_url(&self, url: &str) -> Result<AnalysisResult> {
        let mut messages = Vec::new();
        messages.push(format!("Analyzing URL: {}", url));

        // Handle GitHub URLs specially
        if url.contains("github.com") {
            return self.analyze_github_url(url).await;
        }

        Err(anyhow::anyhow!("URL analysis not yet implemented for non-GitHub URLs"))
    }

    /// Analyze GitHub repository
    async fn analyze_github_url(&self, url: &str) -> Result<AnalysisResult> {
        let mut messages = Vec::new();

        // Extract owner and repo from GitHub URL
        let parts: Vec<&str> = url.split('/').collect();
        if parts.len() < 5 {
            return Err(anyhow::anyhow!("Invalid GitHub URL format"));
        }

        let owner = parts[parts.len() - 2];
        let repo = parts[parts.len() - 1].trim_end_matches(".git");

        messages.push(format!("Fetching from GitHub: {}/{}", owner, repo));

        // Fetch package.json from GitHub raw content
        let package_url = format!(
            "https://raw.githubusercontent.com/{}/{}/main/package.json",
            owner, repo
        );

        let mut config = match self.fetch_url_content(&package_url).await {
            Ok(content) => {
                messages.push("Found package.json on main branch".to_string());
                self.package_parser.parse_package_json(&content)?
            }
            Err(_) => {
                // Try master branch
                let package_url = format!(
                    "https://raw.githubusercontent.com/{}/{}/master/package.json",
                    owner, repo
                );
                match self.fetch_url_content(&package_url).await {
                    Ok(content) => {
                        messages.push("Found package.json on master branch".to_string());
                        self.package_parser.parse_package_json(&content)?
                    }
                    Err(_) => {
                        // Create basic config
                        DetectedConfig {
                            name: repo.to_string(),
                            description: None,
                            command: "npx".to_string(),
                            args: vec!["-y".to_string(), format!("github:{}/{}", owner, repo)],
                            env: HashMap::new(),
                            optional_args: Vec::new(),
                            server_type: "stdio".to_string(),
                            install_command: Some(format!("npm install github:{}/{}", owner, repo)),
                            docs_url: Some(url.to_string()),
                            author: Some(owner.to_string()),
                            version: None,
                        }
                    }
                }
            }
        };

        // Try to fetch README
        for branch in &["main", "master"] {
            for readme in &["README.md", "README.MD", "readme.md"] {
                let readme_url = format!(
                    "https://raw.githubusercontent.com/{}/{}/{}/{}",
                    owner, repo, branch, readme
                );

                if let Ok(content) = self.fetch_url_content(&readme_url).await {
                    if let Ok(readme_info) = self.readme_parser.parse_readme(&content) {
                        messages.push(format!("Parsed README from {} branch", branch));
                        config = self.merge_configs(config, readme_info);
                        break;
                    }
                }
            }
        }

        let confidence = self.calculate_confidence(&config, &messages);

        Ok(AnalysisResult {
            config,
            confidence,
            messages,
            success: true,
        })
    }

    /// Fetch content from URL
    async fn fetch_url_content(&self, url: &str) -> Result<String> {
        let client = reqwest::Client::builder()
            .user_agent("MCP-Control/1.0")
            .build()?;

        let response = client.get(url).send().await?;
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("HTTP error: {}", response.status()));
        }

        Ok(response.text().await?)
    }

    /// Merge two configs, preferring more detailed information
    fn merge_configs(&self, mut base: DetectedConfig, overlay: DetectedConfig) -> DetectedConfig {
        // Prefer non-empty description
        if base.description.is_none() && overlay.description.is_some() {
            base.description = overlay.description;
        }

        // Merge environment variables
        for (key, value) in overlay.env {
            base.env.entry(key).or_insert(value);
        }

        // Merge optional arguments
        base.optional_args.extend(overlay.optional_args);

        // Prefer non-empty fields
        if base.docs_url.is_none() {
            base.docs_url = overlay.docs_url;
        }
        if base.author.is_none() {
            base.author = overlay.author;
        }

        base
    }

    /// Calculate confidence score
    fn calculate_confidence(&self, config: &DetectedConfig, messages: &[String]) -> f32 {
        let mut score = 0.0;
        let mut total = 0.0;

        // Has description
        total += 0.1;
        if config.description.is_some() {
            score += 0.1;
        }

        // Has command
        total += 0.2;
        if !config.command.is_empty() {
            score += 0.2;
        }

        // Has args
        total += 0.1;
        if !config.args.is_empty() {
            score += 0.1;
        }

        // Has env vars
        total += 0.15;
        if !config.env.is_empty() {
            score += 0.15;
        }

        // Has docs
        total += 0.1;
        if config.docs_url.is_some() {
            score += 0.1;
        }

        // Has author
        total += 0.05;
        if config.author.is_some() {
            score += 0.05;
        }

        // Successful parsing
        total += 0.3;
        if messages.iter().any(|m| m.contains("Parsed")) {
            score += 0.3;
        }

        if total > 0.0 {
            score / total
        } else {
            0.0
        }
    }
}

impl Default for ServerAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
