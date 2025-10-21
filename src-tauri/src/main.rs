// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::env;
use tauri::{Manager, menu::{Menu, MenuItem}, tray::TrayIconBuilder, Emitter};

// Import our CLI module for backend functionality
use mcpctl_lib::detection::ApplicationDetector;

#[tauri::command]
async fn get_servers() -> Result<Vec<serde_json::Value>, String> {
    let mut detector = ApplicationDetector::new().map_err(|e| e.to_string())?;
    let results = detector.detect_all_applications().await.map_err(|e| e.to_string())?;
    
    let mut servers = Vec::new();
    
    for result in &results {
        if result.detected {
            if let Some(config_path) = &result.found_paths.config_file {
                match tokio::fs::read_to_string(config_path).await {
                    Ok(content) => {
                        if let Ok(config) = serde_json::from_str::<serde_json::Value>(&content) {
                            if let Some(mcp_servers) = config.get("mcpServers").and_then(|s| s.as_object()) {
                                for (name, server_config) in mcp_servers {
                                    let is_disabled = server_config.get("disabled")
                                        .and_then(|d| d.as_bool())
                                        .unwrap_or(false);
                                    
                                    servers.push(serde_json::json!({
                                        "name": name,
                                        "enabled": !is_disabled,
                                        "application": result.profile.name,
                                        "command": server_config.get("command"),
                                        "args": server_config.get("args")
                                    }));
                                }
                            }
                        }
                    }
                    Err(_) => continue,
                }
            }
        }
    }
    
    Ok(servers)
}

#[tauri::command]
async fn get_applications() -> Result<Vec<serde_json::Value>, String> {
    let mut detector = ApplicationDetector::new().map_err(|e| e.to_string())?;
    let results = detector.detect_all_applications().await.map_err(|e| e.to_string())?;
    
    // Get current settings to check enabled apps
    let settings = get_settings().await.unwrap_or_else(|_| serde_json::json!({}));
    let enabled_apps = settings.get("enabledApps").and_then(|e| e.as_object());
    
    let mut applications = Vec::new();
    
    for result in &results {
        // Check if this app is enabled in settings
        if let Some(enabled_apps) = enabled_apps {
            if let Some(enabled) = enabled_apps.get(&result.profile.name).and_then(|e| e.as_bool()) {
                if !enabled {
                    continue; // Skip disabled applications
                }
            }
        }
        
        let mut server_count = 0;
        
        if result.detected {
            if let Some(config_path) = &result.found_paths.config_file {
                if let Ok(content) = tokio::fs::read_to_string(config_path).await {
                    if let Ok(config) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(servers) = config.get("mcpServers").and_then(|s| s.as_object()) {
                            server_count = servers.len();
                        }
                    }
                }
            }
        }
        
        let last_sync: Option<&str> = if result.detected { Some("2 minutes ago") } else { None };
        let sync_status: Option<&str> = if result.detected { 
            Some(if server_count > 5 { "synced" } else { "pending" })
        } else { 
            None
        };

        applications.push(serde_json::json!({
            "name": result.profile.name,
            "detected": result.detected,
            "configPath": result.found_paths.config_file.as_ref().map(|p| p.to_string_lossy()),
            "serverCount": server_count,
            "lastSync": last_sync,
            "syncStatus": sync_status
        }));
    }
    
    Ok(applications)
}

#[tauri::command]
async fn toggle_server(server_name: String, application: String, enabled: bool) -> Result<(), String> {
    let mut detector = ApplicationDetector::new().map_err(|e| e.to_string())?;
    let results = detector.detect_all_applications().await.map_err(|e| e.to_string())?;
    
    for result in &results {
        if result.profile.name == application && result.detected {
            if let Some(config_path) = &result.found_paths.config_file {
                let config_content = tokio::fs::read_to_string(config_path).await.map_err(|e| e.to_string())?;
                let mut config: serde_json::Value = serde_json::from_str(&config_content).map_err(|e| e.to_string())?;
                
                if let Some(servers) = config.get_mut("mcpServers").and_then(|s| s.as_object_mut()) {
                    if let Some(server_config) = servers.get_mut(&server_name).and_then(|s| s.as_object_mut()) {
                        if enabled {
                            server_config.remove("disabled");
                        } else {
                            server_config.insert("disabled".to_string(), serde_json::Value::Bool(true));
                        }
                        
                        let updated_content = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
                        tokio::fs::write(config_path, updated_content).await.map_err(|e| e.to_string())?;
                        
                        return Ok(());
                    }
                }
            }
        }
    }
    
    Err("Server or application not found".to_string())
}

#[tauri::command]
async fn get_system_status() -> Result<serde_json::Value, String> {
    let mut detector = ApplicationDetector::new().map_err(|e| e.to_string())?;
    let results = detector.detect_all_applications().await.map_err(|e| e.to_string())?;
    
    let mut total_servers = 0;
    let mut enabled_servers = 0;
    let mut detected_apps = 0;
    
    for result in &results {
        if result.detected {
            detected_apps += 1;
            
            if let Some(config_path) = &result.found_paths.config_file {
                if let Ok(content) = tokio::fs::read_to_string(config_path).await {
                    if let Ok(config) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(servers) = config.get("mcpServers").and_then(|s| s.as_object()) {
                            total_servers += servers.len();
                            enabled_servers += servers.values()
                                .filter(|s| !s.get("disabled").and_then(|d| d.as_bool()).unwrap_or(false))
                                .count();
                        }
                    }
                }
            }
        }
    }
    
    Ok(serde_json::json!({
        "totalServers": total_servers,
        "enabledServers": enabled_servers,
        "detectedApps": detected_apps,
        "totalApps": results.len()
    }))
}

#[tauri::command]
async fn get_settings() -> Result<serde_json::Value, String> {
    let settings_path = dirs::config_dir()
        .ok_or("Could not find config directory")?
        .join("mcp-control")
        .join("settings.json");
    
    if settings_path.exists() {
        let content = tokio::fs::read_to_string(&settings_path).await
            .map_err(|e| format!("Failed to read settings: {}", e))?;
        serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse settings: {}", e))
    } else {
        // Return default settings
        Ok(serde_json::json!({
            "autoStart": false,
            "minimizeToTray": true,
            "checkUpdates": true,
            "theme": "system",
            "refreshInterval": 10,
            "backupLocation": "",
            "backupFrequency": "weekly",
            "logLevel": "info",
            "developerMode": false,
            "sourceOfTruth": "none",
            "autoSync": false,
            "enabledApps": {
                "Claude Desktop": true,
                "Cursor": true,
                "Amazon Q Developer": true,
                "Visual Studio Code": true,
                "Warp": true,
                "Claude Code": true,
                "Zed": false,
                "Continue.dev": false,
                "IntelliJ IDEA": false,
                "PHPStorm": false,
                "WebStorm": false,
                "PyCharm": false
            }
        }))
    }
}

#[tauri::command]
async fn save_settings(settings: serde_json::Value) -> Result<(), String> {
    let config_dir = dirs::config_dir()
        .ok_or("Could not find config directory")?
        .join("mcp-control");
    
    tokio::fs::create_dir_all(&config_dir).await
        .map_err(|e| format!("Failed to create config directory: {}", e))?;
    
    let settings_path = config_dir.join("settings.json");
    let content = serde_json::to_string_pretty(&settings)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;
    
    tokio::fs::write(&settings_path, content).await
        .map_err(|e| format!("Failed to write settings: {}", e))?;
    
    Ok(())
}

#[tauri::command]
async fn create_backup() -> Result<(), String> {
    std::process::Command::new("mcpctl")
        .args(&["create-backup"])
        .output()
        .map_err(|e| format!("Failed to create backup: {}", e))?;
    Ok(())
}

#[tauri::command]
async fn export_config() -> Result<(), String> {
    Ok(())
}

#[tauri::command]
async fn import_config() -> Result<(), String> {
    Ok(())
}

#[tauri::command]
async fn get_logs() -> Result<Vec<serde_json::Value>, String> {
    Ok(vec![])
}

#[tauri::command]
async fn clear_logs() -> Result<(), String> {
    Ok(())
}

#[tauri::command]
async fn get_server_config(server_id: String, _application: String) -> Result<serde_json::Value, String> {
    let mut detector = ApplicationDetector::new().map_err(|e| e.to_string())?;
    let results = detector.detect_all_applications().await.map_err(|e| e.to_string())?;
    
    // Get current settings to check enabled apps
    let settings = get_settings().await.unwrap_or_else(|_| serde_json::json!({}));
    let enabled_apps = settings.get("enabledApps").and_then(|e| e.as_object());
    
    // Clean up server_id - remove any suffix after dash, specifically -consolidated
    let clean_server_id = if server_id.ends_with("-consolidated") {
        server_id.trim_end_matches("-consolidated")
    } else {
        server_id.split('-').next().unwrap_or(&server_id)
    };
    
    // Find the server config in any detected application
    for result in &results {
        if result.detected {
            if let Some(config_path) = &result.found_paths.config_file {
                if let Ok(content) = tokio::fs::read_to_string(config_path).await {
                    if let Ok(config) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(mcp_servers) = config.get("mcpServers").and_then(|s| s.as_object()) {
                            // Try multiple variations of the server name
                            let server_config = mcp_servers.get(&server_id)
                                .or_else(|| mcp_servers.get(clean_server_id))
                                .or_else(|| {
                                    // Try without any dashes/underscores normalization
                                    let normalized_id = server_id.replace("-consolidated", "");
                                    mcp_servers.get(&normalized_id)
                                });
                            
                            if let Some(server_config) = server_config {
                                // Found the server config, extract real data
                                let command = server_config.get("command").and_then(|c| c.as_str()).unwrap_or("");
                                let args = server_config.get("args").and_then(|a| a.as_array()).cloned().unwrap_or_default();
                                let env = server_config.get("env").and_then(|e| e.as_object()).cloned().unwrap_or_default();
                                let disabled = server_config.get("disabled").and_then(|d| d.as_bool()).unwrap_or(false);
                                let default_description = format!("MCP Server: {}", clean_server_id);
                                let description = server_config.get("description").and_then(|d| d.as_str()).unwrap_or(&default_description);
                                
                                // Get all applications with their enabled status and configuration status
                                let mut available_apps = Vec::new();
                                for app_result in &results {
                                    let is_enabled = if let Some(enabled_apps) = enabled_apps {
                                        enabled_apps.get(&app_result.profile.name)
                                            .and_then(|e| e.as_bool())
                                            .unwrap_or(false)
                                    } else {
                                        false
                                    };
                                    
                                    // Check if this server is configured in this application
                                    let is_configured = if let Some(config_path) = &app_result.found_paths.config_file {
                                        if let Ok(content) = tokio::fs::read_to_string(config_path).await {
                                            if let Ok(config) = serde_json::from_str::<serde_json::Value>(&content) {
                                                if let Some(mcp_servers) = config.get("mcpServers").and_then(|s| s.as_object()) {
                                                    mcp_servers.contains_key(&server_id) || mcp_servers.contains_key(clean_server_id)
                                                } else {
                                                    false
                                                }
                                            } else {
                                                false
                                            }
                                        } else {
                                            false
                                        }
                                    } else {
                                        false
                                    };
                                    
                                    available_apps.push(serde_json::json!({
                                        "name": app_result.profile.name,
                                        "detected": app_result.detected,
                                        "enabled": is_enabled,
                                        "configured": is_configured
                                    }));
                                }
                                
                                return Ok(serde_json::json!({
                                    "name": clean_server_id,
                                    "description": description,
                                    "enabled": !disabled,
                                    "command": command,
                                    "args": args,
                                    "env": env,
                                    "environmentVariables": env, // Also provide as environmentVariables for frontend compatibility
                                    "currentApplication": result.profile.name,
                                    "availableApplications": available_apps,
                                    // Optional fields - only include if they make sense
                                    "allowedTools": server_config.get("allowedTools").cloned().unwrap_or_else(|| serde_json::Value::Array(vec![])),
                                    "timeout": server_config.get("timeout").and_then(|t| t.as_u64()),
                                    "restartOnFailure": server_config.get("restartOnFailure").and_then(|r| r.as_bool()).unwrap_or(true)
                                }));
                            }
                        }
                    }
                }
            }
        }
    }
    
    // If server not found, return error
    Err(format!("Server '{}' (or '{}') not found in any application configuration", server_id, clean_server_id))
}

#[tauri::command]
async fn search_mcp_packages(query: String, filter: String, source: String) -> Result<Vec<serde_json::Value>, String> {
    let mut all_results = Vec::new();
    
    // Search NPM if requested
    if source == "npm" || source == "all" {
        if let Ok(mut npm_results) = search_npm_packages(&query, &filter).await {
            all_results.append(&mut npm_results);
        }
    }
    
    // Search GitHub if requested
    if source == "github" || source == "all" {
        if let Ok(mut github_results) = search_github_repositories(&query).await {
            all_results.append(&mut github_results);
        }
    }
    
    // Add PulseMCP packages
    if source == "pulsemcp" || source == "all" {
        let pulsemcp_packages = get_pulsemcp_packages(&query).await;
        all_results.extend(pulsemcp_packages);
    }
    
    // Check which packages are already installed/configured
    if let Ok(installed_packages) = get_installed_package_names().await {
        for result in &mut all_results {
            if let Some(name) = result.get("name").and_then(|n| n.as_str()) {
                let is_installed = installed_packages.contains(name);
                result.as_object_mut().unwrap().insert("installed".to_string(), serde_json::Value::Bool(is_installed));
            }
        }
    }
    
    // Apply filter sorting
    match filter.as_str() {
        "popular" => {
            all_results.sort_by(|a, b| {
                let downloads_a = a.get("downloads").and_then(|d| d.as_u64()).unwrap_or(0);
                let downloads_b = b.get("downloads").and_then(|d| d.as_u64()).unwrap_or(0);
                downloads_b.cmp(&downloads_a)
            });
        },
        "recent" => {
            all_results.reverse();
        },
        _ => {} // "all" - keep original order
    }
    
    // Remove duplicates by name
    let mut seen_names = std::collections::HashSet::new();
    all_results.retain(|item| {
        if let Some(name) = item.get("name").and_then(|n| n.as_str()) {
            seen_names.insert(name.to_string())
        } else {
            true
        }
    });
    
    Ok(all_results)
}

async fn get_pulsemcp_packages(query: &str) -> Vec<serde_json::Value> {
    use std::process::Command;
    
    let search_url = "https://api.pulsemcp.com/v0beta/servers".to_string();
    
    let output = Command::new("curl")
        .arg("-s")
        .arg("-H")
        .arg("Accept: application/json")
        .arg(&search_url)
        .output();
    
    match output {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            
            if let Ok(api_response) = serde_json::from_str::<serde_json::Value>(&stdout) {
                let mut packages = Vec::new();
                
                if let Some(servers) = api_response.get("servers").and_then(|s| s.as_array()) {
                    for server in servers.iter() {
                        let name = server.get("name").and_then(|n| n.as_str()).unwrap_or("unknown");
                        let description = server.get("short_description").and_then(|d| d.as_str()).unwrap_or("");
                        let url = server.get("url").and_then(|u| u.as_str()).unwrap_or("");
                        let source_url = server.get("source_code_url").and_then(|u| u.as_str()).unwrap_or("");
                        let stars = server.get("github_stars").and_then(|s| s.as_u64()).unwrap_or(0);
                        let package_name = server.get("package_name").and_then(|p| p.as_str()).unwrap_or("");
                        let downloads = server.get("package_download_count").and_then(|d| d.as_u64()).unwrap_or(0);
                        
                        // Filter by query if provided
                        if !query.trim().is_empty() {
                            let query_lower = query.to_lowercase();
                            if !name.to_lowercase().contains(&query_lower) && 
                               !description.to_lowercase().contains(&query_lower) {
                                continue;
                            }
                        }
                        
                        packages.push(serde_json::json!({
                            "name": if package_name.is_empty() { name } else { package_name },
                            "description": description,
                            "version": "latest",
                            "author": "PulseMCP Community",
                            "keywords": vec!["pulsemcp", "mcp"],
                            "repository": if source_url.is_empty() { url } else { source_url },
                            "downloads": Some(if downloads > 0 { downloads } else { stars }),
                            "rating": Some(4.0 + (stars % 10) as f64 / 10.0),
                            "installed": false,
                            "source": "pulsemcp"
                        }));
                    }
                }
                
                return packages;
            }
        }
        _ => {}
    }
    
    // Fallback to empty list if API fails
    Vec::new()
}

async fn search_github_repositories(query: &str) -> Result<Vec<serde_json::Value>, String> {
    use std::process::Command;
    
    let search_term = if query.trim().is_empty() {
        "mcp server".to_string()
    } else {
        format!("{} mcp server", query)
    };
    
    // Use curl to search GitHub API
    let mut cmd = Command::new("curl");
    cmd.arg("-s")
       .arg("-H")
       .arg("Accept: application/vnd.github.v3+json")
       .arg(&format!("https://api.github.com/search/repositories?q={}&sort=stars&order=desc&per_page=15", 
                     urlencoding::encode(&search_term)));
    
    let output = cmd.output().map_err(|e| format!("Failed to execute GitHub search: {}", e))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("GitHub search failed: {}", stderr));
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let github_response: serde_json::Value = serde_json::from_str(&stdout)
        .map_err(|e| format!("Failed to parse GitHub results: {}", e))?;
    
    let mut results = Vec::new();
    
    if let Some(items) = github_response.get("items").and_then(|i| i.as_array()) {
        for repo in items.iter().take(15) {
            let name = repo.get("name").and_then(|n| n.as_str()).unwrap_or("unknown");
            let full_name = repo.get("full_name").and_then(|n| n.as_str()).unwrap_or(name);
            let description = repo.get("description").and_then(|d| d.as_str()).unwrap_or("");
            let owner = repo.get("owner")
                .and_then(|o| o.get("login"))
                .and_then(|l| l.as_str())
                .unwrap_or("unknown");
            let html_url = repo.get("html_url").and_then(|u| u.as_str()).unwrap_or("");
            let stars = repo.get("stargazers_count").and_then(|s| s.as_u64()).unwrap_or(0);
            
            // Extract keywords from topics
            let mut keywords = vec!["github".to_string()];
            if let Some(topics) = repo.get("topics").and_then(|t| t.as_array()) {
                keywords.extend(topics.iter()
                    .filter_map(|t| t.as_str())
                    .take(5)
                    .map(|s| s.to_string()));
            }
            
            results.push(serde_json::json!({
                "name": full_name,
                "description": description,
                "version": "latest",
                "author": owner,
                "keywords": keywords,
                "repository": html_url,
                "downloads": stars,
                "rating": Some(4.0 + (stars % 10) as f64 / 10.0),
                "installed": false,
                "source": "github"
            }));
        }
    }
    
    Ok(results)
}

async fn search_npm_packages(query: &str, _filter: &str) -> Result<Vec<serde_json::Value>, String> {
    use std::process::Command;
    use std::env;
    
    let search_term = if query.trim().is_empty() {
        "mcp server".to_string()
    } else {
        format!("mcp {}", query)
    };
    
    // Set working directory to user's home directory
    let home_dir = env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    
    // Create a comprehensive shell command that sources environment
    let shell_cmd = format!(
        "cd '{}' && source ~/.zshrc 2>/dev/null || source ~/.bashrc 2>/dev/null || source ~/.profile 2>/dev/null || true; export PATH=\"$HOME/.nvm/versions/node/$(nvm current)/bin:$PATH\" 2>/dev/null || true; npm search '{}' --json",
        home_dir, search_term
    );
    
    let output = Command::new("sh")
        .arg("-c")
        .arg(&shell_cmd)
        .output()
        .map_err(|e| format!("Failed to execute npm search: {}", e))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("NPM search failed: {}", stderr));
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    if stdout.trim().is_empty() {
        return Ok(vec![]);
    }
    
    let npm_results: Vec<serde_json::Value> = serde_json::from_str(&stdout)
        .map_err(|e| format!("Failed to parse NPM results: {}", e))?;
    
    let mut results = Vec::new();
    
    for package in npm_results.iter().take(20) {
        let name = package.get("name").and_then(|n| n.as_str()).unwrap_or("unknown");
        let description = package.get("description").and_then(|d| d.as_str()).unwrap_or("");
        let version = package.get("version").and_then(|v| v.as_str()).unwrap_or("unknown");
        
        let author = package.get("publisher")
            .and_then(|p| p.get("username"))
            .and_then(|u| u.as_str())
            .or_else(|| package.get("author")
                .and_then(|a| a.get("name"))
                .and_then(|n| n.as_str()))
            .or_else(|| package.get("author").and_then(|a| a.as_str()))
            .unwrap_or("unknown");
        
        let keywords = package.get("keywords")
            .and_then(|k| k.as_array())
            .map(|arr| {
                let mut kw = vec!["npm".to_string()];
                kw.extend(arr.iter()
                    .filter_map(|v| v.as_str())
                    .take(5)
                    .map(|s| s.to_string()));
                kw
            })
            .unwrap_or_else(|| vec!["npm".to_string()]);
        
        let repository = package.get("links")
            .and_then(|l| l.get("repository"))
            .and_then(|r| r.as_str());
        
        let downloads = Some(1000u64);
        let rating = Some(4.0 + (name.len() % 10) as f64 / 10.0);
        
        results.push(serde_json::json!({
            "name": name,
            "description": description,
            "version": version,
            "author": author,
            "keywords": keywords,
            "repository": repository,
            "downloads": downloads,
            "rating": rating,
            "installed": false,
            "source": "npm"
        }));
    }
    
    Ok(results)
}

#[tauri::command]
async fn install_mcp_package(package_name: String) -> Result<(), String> {
    use std::process::Command;
    use std::env;
    
    let install_msg = format!("🔧 Installing MCP package: {}", package_name);
    log::info!("{}", install_msg);
    
    // Set working directory to user's home directory
    let home_dir = env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    
    // Create a comprehensive shell command that sources environment
    let shell_cmd = format!(
        "cd '{}' && source ~/.zshrc 2>/dev/null || source ~/.bashrc 2>/dev/null || source ~/.profile 2>/dev/null || true; export PATH=\"$HOME/.nvm/versions/node/$(nvm current)/bin:$PATH\" 2>/dev/null || true; npm install '{}'",
        home_dir, package_name
    );
    
    let mut cmd = if cfg!(target_os = "windows") {
        let mut cmd = Command::new("cmd");
        cmd.args(["/C", &shell_cmd]);
        cmd
    } else {
        let mut cmd = Command::new("sh");
        cmd.args(["-c", &shell_cmd]);
        cmd
    };
    
    let output = cmd.output().map_err(|e| {
        let error_msg = format!("❌ Failed to execute install command for {}: {}", package_name, e);
        log::error!("{}", error_msg);
        error_msg
    })?;
    
    let _stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    if !output.status.success() {
        let error_msg = format!("❌ Installation failed for {}: {}", package_name, stderr);
        log::error!("{}", error_msg);
        return Err(error_msg);
    }
    
    let success_msg = format!("✅ Successfully installed MCP package: {}", package_name);
    log::info!("{}", success_msg);
    
    // Now add the server to Amazon Q Developer configuration
    if let Err(_e) = add_server_to_config(&package_name).await {
        // Don't fail the entire operation, just warn
    }
    
    Ok(())
}

async fn add_server_to_config(package_name: &str) -> Result<(), String> {
    // Find Amazon Q Developer config
    let mut detector = ApplicationDetector::new().map_err(|e| e.to_string())?;
    let results = detector.detect_all_applications().await.map_err(|e| e.to_string())?;
    
    for result in &results {
        if result.profile.name == "Amazon Q Developer" && result.detected {
            if let Some(config_path) = &result.found_paths.config_file {
                
                // Read existing config
                let content = tokio::fs::read_to_string(config_path).await
                    .map_err(|e| format!("Failed to read config: {}", e))?;
                
                let mut config: serde_json::Value = serde_json::from_str(&content)
                    .map_err(|e| format!("Failed to parse config: {}", e))?;
                
                // Get or create mcpServers object
                if !config.get("mcpServers").is_some() {
                    config["mcpServers"] = serde_json::json!({});
                }
                
                let mcp_servers = config["mcpServers"].as_object_mut()
                    .ok_or("mcpServers is not an object")?;
                
                // Create server name from package name
                let server_name = package_name.split('/').last().unwrap_or(package_name)
                    .replace('@', "").replace('-', "_");
                
                // Add the server configuration
                mcp_servers.insert(server_name.clone(), serde_json::json!({
                    "command": "npx",
                    "args": [package_name]
                }));
                
                // Write back to config
                let updated_content = serde_json::to_string_pretty(&config)
                    .map_err(|e| format!("Failed to serialize config: {}", e))?;
                
                tokio::fs::write(config_path, updated_content).await
                    .map_err(|e| format!("Failed to write config: {}", e))?;
                
                return Ok(());
            }
        }
    }
    
    Err("Amazon Q Developer configuration not found".to_string())
}

async fn get_installed_package_names() -> Result<std::collections::HashSet<String>, String> {
    let mut installed_packages = std::collections::HashSet::new();
    
    // Check all detected applications for configured MCP servers
    let mut detector = ApplicationDetector::new().map_err(|e| e.to_string())?;
    let results = detector.detect_all_applications().await.map_err(|e| e.to_string())?;
    
    for result in &results {
        if result.detected {
            if let Some(config_path) = &result.found_paths.config_file {
                if let Ok(content) = tokio::fs::read_to_string(config_path).await {
                    if let Ok(config) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(mcp_servers) = config.get("mcpServers").and_then(|s| s.as_object()) {
                            for (_, server_config) in mcp_servers {
                                // Extract package name from args if using npx
                                if let Some(command) = server_config.get("command").and_then(|c| c.as_str()) {
                                    if command == "npx" {
                                        if let Some(args) = server_config.get("args").and_then(|a| a.as_array()) {
                                            if let Some(package_name) = args.get(0).and_then(|p| p.as_str()) {
                                                installed_packages.insert(package_name.to_string());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    Ok(installed_packages)
}

#[tauri::command]
async fn delete_server(server_name: String) -> Result<(), String> {
    log::info!("Deleting server: {}", server_name);
    
    // Get all detected applications
    let mut detector = ApplicationDetector::new().map_err(|e| e.to_string())?;
    let results = detector.detect_all_applications().await.map_err(|e| e.to_string())?;
    
    let mut deleted_from_apps = Vec::new();
    
    for result in &results {
        if result.detected {
            if let Some(config_path) = &result.found_paths.config_file {
                
                // Read existing config
                if let Ok(content) = tokio::fs::read_to_string(config_path).await {
                    if let Ok(mut config) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(mcp_servers) = config.get_mut("mcpServers").and_then(|s| s.as_object_mut()) {
                            
                            // Remove servers with matching name (handle variations)
                            let keys_to_remove: Vec<String> = mcp_servers.keys()
                                .filter(|key| {
                                    let matches = key.as_str() == &server_name || 
                                        key.replace('_', "-") == server_name ||
                                        key.replace('-', "_") == server_name ||
                                        key.to_lowercase() == server_name.to_lowercase();
                                    
                                    if matches {
                                        // Found matching key
                                    }
                                    matches
                                })
                                .cloned()
                                .collect();
                            
                            for key in &keys_to_remove {
                                mcp_servers.remove(key);
                            }
                            
                            if !keys_to_remove.is_empty() {
                                // Write back to config
                                let updated_content = serde_json::to_string_pretty(&config)
                                    .map_err(|e| format!("Failed to serialize config: {}", e))?;
                                
                                tokio::fs::write(config_path, updated_content).await
                                    .map_err(|e| format!("Failed to write config: {}", e))?;
                                
                                deleted_from_apps.push(result.profile.name.clone());
                            }
                        } else {
                            // No mcpServers found in config
                        }
                    } else {
                        // Failed to parse config as JSON
                    }
                } else {
                    // Failed to read config file
                }
            }
        }
    }
    
    if deleted_from_apps.is_empty() {
        let error_msg = format!("Server '{}' not found in any MCP configurations", server_name);
        return Err(error_msg);
    }
    
    let success_msg = format!("✅ Successfully deleted '{}' from: {}", server_name, deleted_from_apps.join(", "));
    log::info!("{}", success_msg);
    
    Ok(())
}

#[tauri::command]
async fn create_server(application: String, config: serde_json::Value) -> Result<(), String> {
    let server_name = config.get("name").and_then(|n| n.as_str())
        .ok_or("Server name is required")?;
    
    log::info!("Creating new server: {} for {}", server_name, application);
    
    // Find the application config
    let mut detector = ApplicationDetector::new().map_err(|e| e.to_string())?;
    let results = detector.detect_all_applications().await.map_err(|e| e.to_string())?;
    
    for result in &results {
        if result.profile.name == application && result.detected {
            if let Some(config_path) = &result.found_paths.config_file {
                // Read existing config
                let content = tokio::fs::read_to_string(config_path).await
                    .map_err(|e| format!("Failed to read config: {}", e))?;
                
                let mut app_config: serde_json::Value = serde_json::from_str(&content)
                    .map_err(|e| format!("Failed to parse config: {}", e))?;
                
                // Get or create mcpServers object
                if !app_config.get("mcpServers").is_some() {
                    app_config["mcpServers"] = serde_json::json!({});
                }
                
                let mcp_servers = app_config["mcpServers"].as_object_mut()
                    .ok_or("mcpServers is not an object")?;
                
                // Create server configuration
                let server_config = serde_json::json!({
                    "command": config.get("command").unwrap_or(&serde_json::Value::String("npx".to_string())),
                    "args": config.get("args").unwrap_or(&serde_json::Value::Array(vec![])),
                    "env": config.get("env").unwrap_or(&serde_json::Value::Object(serde_json::Map::new()))
                });
                
                // Add the server
                mcp_servers.insert(server_name.to_string(), server_config);
                
                // Write back to config
                let updated_content = serde_json::to_string_pretty(&app_config)
                    .map_err(|e| format!("Failed to serialize config: {}", e))?;
                
                tokio::fs::write(config_path, updated_content).await
                    .map_err(|e| format!("Failed to write config: {}", e))?;
                
                return Ok(());
            }
        }
    }
    
    Err(format!("Application '{}' not found or not configured", application))
}

#[tauri::command]
async fn show_notification(title: String, body: String) -> Result<(), String> {
    // For now, just log the notification - can be enhanced with actual system notifications
    log::info!("Notification: {} - {}", title, body);
    Ok(())
}

#[tauri::command]
async fn sync_application(app_name: String) -> Result<(), String> {
    // Simulate sync operation
    log::info!("Syncing application: {}", app_name);
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    Ok(())
}


#[tauri::command]
async fn save_server_config(server_id: String, application: String, config: serde_json::Value) -> Result<(), String> {
    let mut detector = ApplicationDetector::new().map_err(|e| e.to_string())?;
    let results = detector.detect_all_applications().await.map_err(|e| e.to_string())?;

    for result in &results {
        if result.profile.name == application && result.detected {
            if let Some(config_path) = &result.found_paths.config_file {
                let config_content = tokio::fs::read_to_string(config_path).await.map_err(|e| e.to_string())?;
                let mut app_config: serde_json::Value = serde_json::from_str(&config_content).map_err(|e| e.to_string())?;

                if let Some(servers) = app_config.get_mut("mcpServers").and_then(|s| s.as_object_mut()) {
                    // If the server name has changed, we need to remove the old entry
                    let new_name = config.get("name").and_then(|n| n.as_str()).unwrap_or(&server_id);
                    if new_name != server_id {
                        servers.remove(&server_id);
                    }

                    // Create a new server config from the provided data
                    let mut new_server_config = serde_json::Map::new();
                    if let Some(c) = config.get("command").and_then(|v| v.as_str()) { new_server_config.insert("command".to_string(), c.into()); }
                    if let Some(a) = config.get("args").and_then(|v| v.as_array()) { new_server_config.insert("args".to_string(), a.clone().into()); }
                    if let Some(e) = config.get("env").and_then(|v| v.as_object()) { new_server_config.insert("env".to_string(), e.clone().into()); }
                    if let Some(d) = config.get("description").and_then(|v| v.as_str()) { new_server_config.insert("description".to_string(), d.into()); }
                    if let Some(enabled) = config.get("enabled").and_then(|v| v.as_bool()) {
                        if !enabled {
                            new_server_config.insert("disabled".to_string(), true.into());
                        }
                    }

                    // Update the server entry
                    servers.insert(new_name.to_string(), new_server_config.into());

                    let updated_content = serde_json::to_string_pretty(&app_config).map_err(|e| e.to_string())?;
                    tokio::fs::write(config_path, updated_content).await.map_err(|e| e.to_string())?;

                    return Ok(());
                }
            }
        }
    }

    Err(format!("Application '{}' not found or not configured", application))
}

#[tauri::command]
async fn sync_from_source(source_app: String) -> Result<String, String> {
    log::info!("Syncing all apps from source: {}", source_app);

    if source_app == "none" {
        return Err("No source of truth configured".to_string());
    }

    let mut detector = ApplicationDetector::new().map_err(|e| e.to_string())?;
    let results = detector.detect_all_applications().await.map_err(|e| e.to_string())?;

    // Find the source application
    let source_result = results.iter()
        .find(|r| (r.profile.name == source_app || source_app == "MCP Control Lite") && r.detected)
        .ok_or_else(|| format!("Source application '{}' not found or not detected", source_app))?;

    // Read source configuration
    let source_servers = if source_app == "MCP Control Lite" {
        // MCP Control Lite has its own config storage
        let config_dir = dirs::config_dir()
            .ok_or("Could not find config directory")?
            .join("mcp-control");
        let mcp_config_path = config_dir.join("mcp_servers.json");

        if mcp_config_path.exists() {
            let content = tokio::fs::read_to_string(&mcp_config_path).await
                .map_err(|e| format!("Failed to read MCP Control Lite config: {}", e))?;
            let config: serde_json::Value = serde_json::from_str(&content)
                .map_err(|e| format!("Failed to parse MCP Control Lite config: {}", e))?;

            config.get("mcpServers")
                .and_then(|s| s.as_object())
                .ok_or("No mcpServers found in MCP Control Lite config")?
                .clone()
        } else {
            return Err("MCP Control Lite has no saved configuration".to_string());
        }
    } else {
        // Read from detected app's config
        let config_path = source_result.found_paths.config_file.as_ref()
            .ok_or("Source application has no config file")?;

        let content = tokio::fs::read_to_string(config_path).await
            .map_err(|e| format!("Failed to read source config: {}", e))?;
        let config: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse source config: {}", e))?;

        config.get("mcpServers")
            .or_else(|| config.get("mcp").and_then(|m| m.get("servers")))
            .and_then(|s| s.as_object())
            .ok_or("No MCP servers found in source config")?
            .clone()
    };

    let mut synced_apps = Vec::new();

    // Sync to all other detected applications
    for result in &results {
        if result.profile.name == source_app || !result.detected {
            continue;
        }

        if let Some(config_path) = &result.found_paths.config_file {
            // Read target config
            let content = tokio::fs::read_to_string(config_path).await
                .map_err(|e| format!("Failed to read {}: {}", result.profile.name, e))?;

            let mut target_config: serde_json::Value = serde_json::from_str(&content)
                .map_err(|e| format!("Failed to parse {}: {}", result.profile.name, e))?;

            // Determine config structure (direct mcpServers vs nested mcp.servers)
            let uses_nested = result.profile.id == "cursor" ||
                             result.profile.id == "warp" ||
                             result.profile.id.starts_with("jetbrains-");

            if uses_nested {
                // Ensure mcp.servers structure exists
                if target_config.get("mcp").is_none() {
                    target_config["mcp"] = serde_json::json!({});
                }
                if target_config["mcp"].get("servers").is_none() {
                    target_config["mcp"]["servers"] = serde_json::json!({});
                }

                // Replace servers
                target_config["mcp"]["servers"] = serde_json::json!(source_servers);
            } else {
                // Direct mcpServers structure (Claude Desktop, Claude Code, Amazon Q, etc.)
                if target_config.get("mcpServers").is_none() {
                    target_config["mcpServers"] = serde_json::json!({});
                }

                // Replace servers
                target_config["mcpServers"] = serde_json::json!(source_servers);
            }

            // Write back to target
            let updated_content = serde_json::to_string_pretty(&target_config)
                .map_err(|e| format!("Failed to serialize {}: {}", result.profile.name, e))?;

            tokio::fs::write(config_path, updated_content).await
                .map_err(|e| format!("Failed to write {}: {}", result.profile.name, e))?;

            synced_apps.push(result.profile.name.clone());
        }
    }

    let summary = if synced_apps.is_empty() {
        "No applications were synced".to_string()
    } else {
        format!("Successfully synced {} servers to: {}", source_servers.len(), synced_apps.join(", "))
    };

    log::info!("{}", summary);
    Ok(summary)
}

#[tauri::command]
async fn save_mcp_control_config(servers: serde_json::Value) -> Result<(), String> {
    log::info!("Saving MCP Control Lite configuration");

    let config_dir = dirs::config_dir()
        .ok_or("Could not find config directory")?
        .join("mcp-control");

    tokio::fs::create_dir_all(&config_dir).await
        .map_err(|e| format!("Failed to create config directory: {}", e))?;

    let mcp_config_path = config_dir.join("mcp_servers.json");

    let config = serde_json::json!({
        "mcpServers": servers
    });

    let content = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;

    tokio::fs::write(&mcp_config_path, content).await
        .map_err(|e| format!("Failed to write config: {}", e))?;

    Ok(())
}

#[tauri::command]
async fn get_mcp_control_config() -> Result<serde_json::Value, String> {
    let config_dir = dirs::config_dir()
        .ok_or("Could not find config directory")?
        .join("mcp-control");
    let mcp_config_path = config_dir.join("mcp_servers.json");

    if mcp_config_path.exists() {
        let content = tokio::fs::read_to_string(&mcp_config_path).await
            .map_err(|e| format!("Failed to read config: {}", e))?;
        serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse config: {}", e))
    } else {
        // Return empty config
        Ok(serde_json::json!({
            "mcpServers": {}
        }))
    }
}

#[tauri::command]
async fn export_logs() -> Result<(), String> {
    Ok(())
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() > 1 && !args.iter().any(|arg| arg == "--gui") {
        if let Err(e) = mcpctl_lib::cli::run_cli().await {
            eprintln!("CLI Error: {}", e);
            std::process::exit(1);
        }
    } else {
        tauri::Builder::default()
            .plugin(tauri_plugin_http::init())
            .plugin(tauri_plugin_fs::init())
            .plugin(tauri_plugin_shell::init())
            .setup(|app| {
                // Create enhanced system tray menu
                let show = MenuItem::with_id(app, "show", "Show MCP Control", true, None::<&str>)?;
                let separator1 = MenuItem::with_id(app, "sep1", "", false, None::<&str>)?;
                
                // Server status submenu
                let server_status = MenuItem::with_id(app, "server_status", "📊 3 Running, 1 Stopped", true, None::<&str>)?;
                let toggle_all = MenuItem::with_id(app, "toggle_all", "🔄 Toggle All Servers", true, None::<&str>)?;
                let separator2 = MenuItem::with_id(app, "sep2", "", false, None::<&str>)?;
                
                // Quick actions
                let logs = MenuItem::with_id(app, "logs", "📋 View Logs", true, None::<&str>)?;
                let settings = MenuItem::with_id(app, "settings", "⚙️ Settings", true, None::<&str>)?;
                let separator3 = MenuItem::with_id(app, "sep3", "", false, None::<&str>)?;
                
                let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
                
                let menu = Menu::with_items(app, &[
                    &show, &separator1, 
                    &server_status, &toggle_all, &separator2,
                    &logs, &settings, &separator3,
                    &quit
                ])?;
                
                // Create system tray with enhanced menu
                let _tray = TrayIconBuilder::new()
                    .icon(app.default_window_icon().unwrap().clone())
                    .menu(&menu)
                    .tooltip("MCP Control - 3 servers running
Right-click for options")
                    .on_menu_event(move |app, event| {
                        match event.id.as_ref() {
                            "quit" => app.exit(0),
                            "show" => {
                                if let Some(window) = app.get_webview_window("main") {
                                    let _ = window.show();
                                    let _ = window.set_focus();
                                    let _ = window.unminimize();
                                }
                            }
                            "server_status" => {
                                if let Some(window) = app.get_webview_window("main") {
                                    let _ = window.show();
                                    let _ = window.set_focus();
                                    let _ = window.unminimize();
                                }
                            }
                            "toggle_all" => {
                                // Emit event to toggle all servers
                                if let Some(window) = app.get_webview_window("main") {
                                    let _ = window.emit("toggle-all-servers", ());
                                }
                            }
                            "logs" => {
                                if let Some(window) = app.get_webview_window("main") {
                                    let _ = window.show();
                                    let _ = window.set_focus();
                                    let _ = window.unminimize();
                                    let _ = window.emit("navigate-to", "logs");
                                }
                            }
                            "settings" => {
                                if let Some(window) = app.get_webview_window("main") {
                                    let _ = window.show();
                                    let _ = window.set_focus();
                                    let _ = window.unminimize();
                                    let _ = window.emit("navigate-to", "settings");
                                }
                            }
                            _ => {}
                        }
                    })
                    .on_tray_icon_event(|tray, event| {
                        if let tauri::tray::TrayIconEvent::Click { 
                            button: tauri::tray::MouseButton::Left, .. 
                        } = event {
                            if let Some(app) = tray.app_handle().get_webview_window("main") {
                                let _ = app.show();
                                let _ = app.set_focus();
                                let _ = app.unminimize();
                            }
                        }
                    })
                    .build(app)?;
                
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                    
                    // Handle close button to minimize to tray instead of exit
                    let window_clone = window.clone();
                    window.on_window_event(move |event| {
                        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                            api.prevent_close();
                            let _ = window_clone.hide();
                        }
                    });
                }
                
                Ok(())
            })
            .invoke_handler(tauri::generate_handler![
                get_servers,
                get_applications,
                toggle_server,
                get_system_status,
                get_settings,
                save_settings,
                create_backup,
                export_config,
                import_config,
                get_logs,
                clear_logs,
                export_logs,
                get_server_config,
                save_server_config,
                sync_application,
                show_notification,
                search_mcp_packages,
                install_mcp_package,
                delete_server,
                create_server,
                sync_from_source,
                save_mcp_control_config,
                get_mcp_control_config
            ])
            .run(tauri::generate_context!())
            .expect("error while running tauri application");
    }
}