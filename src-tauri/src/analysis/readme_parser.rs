use anyhow::Result;
use regex::Regex;
use std::collections::HashMap;

use super::server_analyzer::{DetectedConfig, EnvVarConfig, ArgConfig};

/// Parser for README.md files
pub struct ReadmeParser;

impl ReadmeParser {
    pub fn new() -> Self {
        Self
    }

    /// Parse README content for configuration information
    pub fn parse_readme(&self, content: &str) -> Result<DetectedConfig> {
        let mut config = DetectedConfig {
            name: "unknown".to_string(),
            description: None,
            command: "npx".to_string(),
            args: Vec::new(),
            env: HashMap::new(),
            optional_args: Vec::new(),
            server_type: "stdio".to_string(),
            install_command: None,
            docs_url: None,
            author: None,
            version: None,
        };

        // Extract description from first paragraph
        config.description = self.extract_description(content);

        // Extract environment variables
        config.env = self.extract_env_vars_from_readme(content);

        // Extract command examples
        if let Some((cmd, args)) = self.extract_command_example(content) {
            config.command = cmd;
            config.args = args;
        }

        // Extract installation command
        config.install_command = self.extract_install_command(content);

        Ok(config)
    }

    /// Extract description from README
    fn extract_description(&self, content: &str) -> Option<String> {
        // Look for first paragraph after title
        let lines: Vec<&str> = content.lines().collect();
        let mut found_title = false;
        let mut description = String::new();

        for line in lines {
            let trimmed = line.trim();

            // Skip empty lines
            if trimmed.is_empty() {
                if found_title && !description.is_empty() {
                    break;
                }
                continue;
            }

            // Skip title lines (# heading)
            if trimmed.starts_with('#') {
                found_title = true;
                continue;
            }

            // Skip badges and images
            if trimmed.starts_with("[![") || trimmed.starts_with("![") {
                continue;
            }

            // Found first content paragraph
            if found_title {
                if description.is_empty() {
                    description = trimmed.to_string();
                } else {
                    description.push(' ');
                    description.push_str(trimmed);
                }

                // Stop at reasonable length
                if description.len() > 200 {
                    break;
                }
            }
        }

        if description.is_empty() {
            None
        } else {
            Some(description)
        }
    }

    /// Extract environment variables from README
    fn extract_env_vars_from_readme(&self, content: &str) -> HashMap<String, EnvVarConfig> {
        let mut env_vars = HashMap::new();

        // Pattern 1: Environment Variables section with table or list
        if let Some(env_section) = self.extract_section(content, &["Environment Variables", "Environment", "Configuration", "Setup"]) {
            env_vars.extend(self.parse_env_section(&env_section));
        }

        // Pattern 2: Inline code blocks with export or env var patterns
        let env_pattern = Regex::new(r"(?m)^(?:export\s+)?([A-Z][A-Z0-9_]+)=(.*)$").unwrap();
        for cap in env_pattern.captures_iter(content) {
            let var_name = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let var_value = cap.get(2).map(|m| m.as_str()).unwrap_or("");

            if !var_name.is_empty() {
                env_vars.entry(var_name.to_string()).or_insert(EnvVarConfig {
                    name: var_name.to_string(),
                    description: None,
                    required: false,
                    default: None,
                    example: Some(var_value.trim().trim_matches('"').to_string()),
                });
            }
        }

        // Pattern 3: ${VAR_NAME} or $VAR_NAME in code blocks
        let var_ref_pattern = Regex::new(r"\$\{?([A-Z][A-Z0-9_]+)\}?").unwrap();
        for cap in var_ref_pattern.captures_iter(content) {
            let var_name = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            if !var_name.is_empty() && var_name != "PATH" && var_name != "HOME" {
                env_vars.entry(var_name.to_string()).or_insert(EnvVarConfig {
                    name: var_name.to_string(),
                    description: Some(format!("Required environment variable (detected from README)")),
                    required: true,
                    default: None,
                    example: None,
                });
            }
        }

        env_vars
    }

    /// Extract a specific section from README
    fn extract_section(&self, content: &str, section_names: &[&str]) -> Option<String> {
        let lines: Vec<&str> = content.lines().collect();
        let mut in_section = false;
        let mut section_content = String::new();
        let mut section_level = 0;

        for line in lines {
            let trimmed = line.trim();

            // Check if this is a heading
            if trimmed.starts_with('#') {
                let level = trimmed.chars().take_while(|&c| c == '#').count();
                let heading_text = trimmed.trim_start_matches('#').trim().to_lowercase();

                // Check if this is our target section
                if section_names.iter().any(|&name| heading_text.contains(&name.to_lowercase())) {
                    in_section = true;
                    section_level = level;
                    continue;
                }

                // If we're in a section and hit a same or higher level heading, we're done
                if in_section && level <= section_level {
                    break;
                }
            }

            if in_section {
                section_content.push_str(line);
                section_content.push('\n');
            }
        }

        if section_content.is_empty() {
            None
        } else {
            Some(section_content)
        }
    }

    /// Parse environment variables from a section
    fn parse_env_section(&self, section: &str) -> HashMap<String, EnvVarConfig> {
        let mut env_vars = HashMap::new();

        // Try to parse as table
        if section.contains('|') {
            env_vars.extend(self.parse_table_format(section));
        }

        // Try to parse as list
        env_vars.extend(self.parse_list_format(section));

        env_vars
    }

    /// Parse markdown table format
    fn parse_table_format(&self, content: &str) -> HashMap<String, EnvVarConfig> {
        let mut env_vars = HashMap::new();
        let lines: Vec<&str> = content.lines().collect();

        // Find table header
        let mut header_idx = None;
        for (i, line) in lines.iter().enumerate() {
            if line.contains('|') && (line.to_lowercase().contains("name") || line.to_lowercase().contains("variable")) {
                header_idx = Some(i);
                break;
            }
        }

        if let Some(header_idx) = header_idx {
            // Skip separator line
            for line in lines.iter().skip(header_idx + 2) {
                if !line.contains('|') {
                    break;
                }

                let cells: Vec<&str> = line.split('|').map(|s| s.trim()).collect();
                if cells.len() >= 2 {
                    let name = cells[1].trim();
                    if !name.is_empty() && name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                        env_vars.insert(name.to_string(), EnvVarConfig {
                            name: name.to_string(),
                            description: cells.get(2).map(|s| s.trim().to_string()),
                            required: cells.iter().any(|&s| s.to_lowercase().contains("required") || s.to_lowercase().contains("yes")),
                            default: cells.iter()
                                .find(|&&s| s.to_lowercase().contains("default"))
                                .and_then(|s| {
                                    let parts: Vec<&str> = s.split(':').collect();
                                    parts.get(1).map(|p| p.trim().to_string())
                                }),
                            example: None,
                        });
                    }
                }
            }
        }

        env_vars
    }

    /// Parse list format
    fn parse_list_format(&self, content: &str) -> HashMap<String, EnvVarConfig> {
        let mut env_vars = HashMap::new();

        // Pattern: - `VAR_NAME`: description
        let list_pattern = Regex::new(r"(?m)^[-*]\s*`?([A-Z][A-Z0-9_]+)`?\s*[:–-]\s*(.*)$").unwrap();

        for cap in list_pattern.captures_iter(content) {
            let var_name = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let description = cap.get(2).map(|m| m.as_str()).unwrap_or("");

            if !var_name.is_empty() {
                let is_required = description.to_lowercase().contains("required");

                env_vars.insert(var_name.to_string(), EnvVarConfig {
                    name: var_name.to_string(),
                    description: Some(description.trim().to_string()),
                    required: is_required,
                    default: None,
                    example: None,
                });
            }
        }

        env_vars
    }

    /// Extract command example from code blocks
    fn extract_command_example(&self, content: &str) -> Option<(String, Vec<String>)> {
        // Look for code blocks with common MCP command patterns
        let code_block_pattern = Regex::new(r"```(?:bash|sh|shell)?\s*\n([\s\S]*?)\n```").unwrap();

        for cap in code_block_pattern.captures_iter(content) {
            let code = cap.get(1).map(|m| m.as_str()).unwrap_or("");

            // Look for npx, node, or npm commands
            for line in code.lines() {
                let trimmed = line.trim();

                // Skip comments and empty lines
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    continue;
                }

                // Parse command
                if let Some((cmd, args)) = self.parse_command_line(trimmed) {
                    // Filter out installation commands
                    if !args.iter().any(|a| a == "install" || a == "i") {
                        return Some((cmd, args));
                    }
                }
            }
        }

        None
    }

    /// Parse a command line into command and args
    fn parse_command_line(&self, line: &str) -> Option<(String, Vec<String>)> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            return None;
        }

        let cmd = parts[0].to_string();
        let args: Vec<String> = parts[1..].iter().map(|s| s.to_string()).collect();

        // Only return if it's a relevant command
        if cmd == "npx" || cmd == "node" || cmd == "npm" || cmd == "python" || cmd == "python3" {
            Some((cmd, args))
        } else {
            None
        }
    }

    /// Extract installation command
    fn extract_install_command(&self, content: &str) -> Option<String> {
        // Look for npm install commands
        let install_pattern = Regex::new(r"npm\s+(?:i|install)\s+([^\s\n]+)").unwrap();

        if let Some(cap) = install_pattern.captures(content) {
            let package = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            return Some(format!("npm install {}", package));
        }

        None
    }
}

impl Default for ReadmeParser {
    fn default() -> Self {
        Self::new()
    }
}
