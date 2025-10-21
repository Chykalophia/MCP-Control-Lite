use crate::detection::profiles::{ApplicationProfile, ApplicationRegistry, DetectionMethod};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

/// Result of application detection with detailed status information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DetectionResult {
    /// Application profile that was detected
    pub profile: ApplicationProfile,
    /// Whether the application was successfully detected
    pub detected: bool,
    /// Detection method that succeeded (if any)
    pub detection_method: Option<DetectionMethod>,
    /// Actual paths found during detection
    pub found_paths: DetectionPaths,
    /// Detection confidence score (0.0 to 1.0)
    pub confidence: f64,
    /// Any errors or warnings encountered during detection
    pub messages: Vec<DetectionMessage>,
    /// Timestamp of detection
    pub detected_at: chrono::DateTime<chrono::Utc>,
}

/// Paths found during application detection
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DetectionPaths {
    /// Executable path (if found)
    pub executable: Option<PathBuf>,
    /// Configuration file path (if found)
    pub config_file: Option<PathBuf>,
    /// Additional paths discovered
    pub additional_paths: Vec<PathBuf>,
}

/// Detection messages for logging and debugging
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DetectionMessage {
    /// Message level
    pub level: MessageLevel,
    /// Message content
    pub message: String,
    /// Detection method that generated this message
    pub method: Option<DetectionMethod>,
}

/// Message severity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessageLevel {
    Info,
    Warning,
    Error,
}

impl std::fmt::Display for MessageLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageLevel::Info => write!(f, "INFO"),
            MessageLevel::Warning => write!(f, "WARN"),
            MessageLevel::Error => write!(f, "ERROR"),
        }
    }
}

/// Multi-strategy application detector
pub struct ApplicationDetector {
    /// Application registry with known applications
    registry: ApplicationRegistry,
    /// Detection cache to avoid repeated checks
    detection_cache: HashMap<String, DetectionResult>,
}

impl ApplicationDetector {
    /// Create a new application detector
    pub fn new() -> Result<Self> {
        Ok(Self {
            registry: ApplicationRegistry::with_auto_load(),
            detection_cache: HashMap::new(),
        })
    }

    /// Create detector with custom registry
    pub fn with_registry(registry: ApplicationRegistry) -> Result<Self> {
        Ok(Self {
            registry,
            detection_cache: HashMap::new(),
        })
    }

    /// Expand path with ~ to home directory
    fn expand_path(&self, path: &str) -> Result<PathBuf> {
        if path.starts_with('~') {
            if let Some(home) = dirs::home_dir() {
                Ok(home.join(&path[2..]))
            } else {
                Err(anyhow::anyhow!("Could not find home directory"))
            }
        } else {
            Ok(PathBuf::from(path))
        }
    }

    /// Detect all known applications
    pub async fn detect_all_applications(&mut self) -> Result<Vec<DetectionResult>> {
        let mut results = Vec::new();
        
        // Collect application IDs first to avoid borrow checker issues
        let app_ids: Vec<String> = self.registry.get_all_applications()
            .iter()
            .map(|profile| profile.id.clone())
            .collect();
        
        for app_id in app_ids {
            let result = self.detect_application(&app_id).await?;
            results.push(result);
        }
        
        Ok(results)
    }

    /// Detect a specific application by ID
    pub async fn detect_application(&mut self, app_id: &str) -> Result<DetectionResult> {
        // Check cache first
        if let Some(cached_result) = self.detection_cache.get(app_id) {
            // Return cached result if it's recent (within 5 minutes)
            let cache_age = chrono::Utc::now() - cached_result.detected_at;
            if cache_age.num_minutes() < 5 {
                return Ok(cached_result.clone());
            }
        }

        let profile = self.registry.get_application(app_id)
            .ok_or_else(|| anyhow::anyhow!("Application profile not found: {}", app_id))?;

        let result = self.perform_detection(profile).await?;
        
        // Cache the result
        self.detection_cache.insert(app_id.to_string(), result.clone());
        
        Ok(result)
    }

    /// Perform detection using the application's configured strategy
    async fn perform_detection(&self, profile: &ApplicationProfile) -> Result<DetectionResult> {
        let mut messages = Vec::new();
        let mut found_paths = DetectionPaths {
            executable: None,
            config_file: None,
            additional_paths: Vec::new(),
        };
        let mut confidence = 0.0;
        let mut detection_method = None;

        // Try detection methods in priority order
        for method in &profile.detection_strategy.priority_order {
            match method {
                DetectionMethod::BundleLookup if profile.detection_strategy.use_bundle_lookup => {
                    if let Ok(result) = self.detect_via_bundle_lookup(profile).await {
                        if result.0 {
                            detection_method = Some(DetectionMethod::BundleLookup);
                            confidence = f64::max(confidence, 0.9);
                            messages.push(DetectionMessage {
                                level: MessageLevel::Info,
                                message: format!("Found via bundle lookup: {}", profile.bundle_id),
                                method: Some(DetectionMethod::BundleLookup),
                            });
                            if let Some(path) = result.1 {
                                found_paths.executable = Some(path);
                            }
                        }
                    }
                }
                DetectionMethod::ExecutableCheck if profile.detection_strategy.use_executable_check => {
                    if let Ok(Some(path)) = self.detect_via_executable_check(profile).await {
                        detection_method = Some(DetectionMethod::ExecutableCheck);
                        confidence = f64::max(confidence, 0.8);
                        found_paths.executable = Some(path.clone());
                        messages.push(DetectionMessage {
                            level: MessageLevel::Info,
                            message: format!("Found executable at: {}", path.display()),
                            method: Some(DetectionMethod::ExecutableCheck),
                        });
                    }
                }
                DetectionMethod::ConfigCheck if profile.detection_strategy.use_config_check => {
                    if let Ok(Some(path)) = self.detect_via_config_check(profile).await {
                        detection_method = Some(DetectionMethod::ConfigCheck);
                        confidence = f64::max(confidence, 0.7);
                        found_paths.config_file = Some(path.clone());
                        messages.push(DetectionMessage {
                            level: MessageLevel::Info,
                            message: format!("Found config file at: {}", path.display()),
                            method: Some(DetectionMethod::ConfigCheck),
                        });
                    }
                }
                DetectionMethod::SpotlightSearch if profile.detection_strategy.use_spotlight => {
                    if let Ok(Some(path)) = self.detect_via_spotlight(profile).await {
                        detection_method = Some(DetectionMethod::SpotlightSearch);
                        confidence = f64::max(confidence, 0.6);
                        found_paths.additional_paths.push(path.clone());
                        messages.push(DetectionMessage {
                            level: MessageLevel::Info,
                            message: format!("Found via Spotlight: {}", path.display()),
                            method: Some(DetectionMethod::SpotlightSearch),
                        });
                    }
                }
                _ => {
                    messages.push(DetectionMessage {
                        level: MessageLevel::Warning,
                        message: format!("Detection method {:?} is disabled or not supported", method),
                        method: Some(method.clone()),
                    });
                }
            }
        }

        let detected = confidence > 0.0;
        
        if !detected {
            messages.push(DetectionMessage {
                level: MessageLevel::Info,
                message: format!("Application {} not detected on this system", profile.name),
                method: None,
            });
        }

        Ok(DetectionResult {
            profile: profile.clone(),
            detected,
            detection_method,
            found_paths,
            confidence,
            messages,
            detected_at: chrono::Utc::now(),
        })
    }

    /// Detect application via macOS bundle lookup
    async fn detect_via_bundle_lookup(&self, profile: &ApplicationProfile) -> Result<(bool, Option<PathBuf>)> {
        // Use mdfind to search for the bundle ID
        let output = Command::new("mdfind")
            .arg(format!("kMDItemCFBundleIdentifier == '{}'", profile.bundle_id))
            .output()
            .context("Failed to execute mdfind command")?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let paths: Vec<&str> = stdout.lines().collect();
            
            if !paths.is_empty() {
                // Return the first valid path
                for path_str in paths {
                    let path = PathBuf::from(path_str.trim());
                    if path.exists() {
                        return Ok((true, Some(path)));
                    }
                }
            }
        }

        Ok((false, None))
    }

    /// Detect application via executable file checks
    async fn detect_via_executable_check(&self, profile: &ApplicationProfile) -> Result<Option<PathBuf>> {
        // Check primary executable paths
        for path_str in &profile.executable_paths {
            let resolved_path = self.expand_path(path_str)?;
            if resolved_path.exists() {
                return Ok(Some(resolved_path));
            }
        }

        // Check alternative executable paths
        for path_str in &profile.alt_executable_paths {
            let resolved_path = self.expand_path(path_str)?;
            if resolved_path.exists() {
                return Ok(Some(resolved_path));
            }
        }

        Ok(None)
    }

    /// Detect application via configuration file checks
    async fn detect_via_config_check(&self, profile: &ApplicationProfile) -> Result<Option<PathBuf>> {
        // Check primary config path
        let resolved_path = self.expand_path(&profile.config_path)?;
        if resolved_path.exists() {
            return Ok(Some(resolved_path));
        }

        // Check alternative config paths
        for path_str in &profile.alt_config_paths {
            let resolved_path = self.expand_path(path_str)?;
            if resolved_path.exists() {
                return Ok(Some(resolved_path));
            }
        }

        Ok(None)
    }

    /// Detect application via macOS Spotlight search
    async fn detect_via_spotlight(&self, profile: &ApplicationProfile) -> Result<Option<PathBuf>> {
        // Search for the application name
        let output = Command::new("mdfind")
            .arg(format!("kMDItemDisplayName == '{}'", profile.name))
            .output()
            .context("Failed to execute mdfind command for Spotlight search")?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let paths: Vec<&str> = stdout.lines().collect();
            
            for path_str in paths {
                let path = PathBuf::from(path_str.trim());
                if path.exists() && path.extension().is_some_and(|ext| ext == "app") {
                    return Ok(Some(path));
                }
            }
        }

        Ok(None)
    }

    /// Get detection results for detected applications only
    pub async fn get_detected_applications(&mut self) -> Result<Vec<DetectionResult>> {
        let all_results = self.detect_all_applications().await?;
        Ok(all_results.into_iter().filter(|r| r.detected).collect())
    }

    /// Clear detection cache
    pub fn clear_cache(&mut self) {
        self.detection_cache.clear();
    }

    /// Get cache statistics
    pub fn get_cache_stats(&self) -> (usize, usize) {
        let total_entries = self.detection_cache.len();
        let fresh_entries = self.detection_cache.values()
            .filter(|result| {
                let age = chrono::Utc::now() - result.detected_at;
                age.num_minutes() < 5
            })
            .count();
        
        (total_entries, fresh_entries)
    }

    /// Add custom application to registry
    pub fn add_custom_application(&mut self, profile: ApplicationProfile) {
        self.registry.add_application(profile);
    }

    /// Get the current registry
    pub fn get_registry(&self) -> &ApplicationRegistry {
        &self.registry
    }
}

impl Default for ApplicationDetector {
    fn default() -> Self {
        Self::new().expect("Failed to create default ApplicationDetector")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::detection::profiles::{ApplicationCategory, ApplicationMetadata, ConfigFormat, DetectionStrategy};

    fn create_test_profile() -> ApplicationProfile {
        ApplicationProfile {
            id: "test-app".to_string(),
            name: "Test Application".to_string(),
            bundle_id: "com.test.app".to_string(),
            config_path: "~/test/config.json".to_string(),
            alt_config_paths: vec!["~/.config/test/config.json".to_string()],
            config_format: ConfigFormat::Json,
            executable_paths: vec!["/Applications/Test.app".to_string()],
            alt_executable_paths: vec!["~/Applications/Test.app".to_string()],
            detection_strategy: DetectionStrategy {
                use_bundle_lookup: true,
                use_executable_check: true,
                use_config_check: true,
                use_spotlight: false,
                priority_order: vec![
                    DetectionMethod::BundleLookup,
                    DetectionMethod::ExecutableCheck,
                    DetectionMethod::ConfigCheck,
                ],
            },
            metadata: ApplicationMetadata {
                version: None,
                developer: "Test Developer".to_string(),
                category: ApplicationCategory::Other("Test".to_string()),
                mcp_version: "1.0".to_string(),
                notes: None,
                requires_permissions: false,
            },
        }
    }

    #[tokio::test]
    async fn test_detector_creation() {
        let detector = ApplicationDetector::new();
        assert!(detector.is_ok());
    }

    #[tokio::test]
    async fn test_detect_nonexistent_application() {
        let mut detector = ApplicationDetector::new().unwrap();
        let result = detector.detect_application("nonexistent-app").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_detect_application_with_custom_profile() {
        let mut registry = ApplicationRegistry::new();
        let test_profile = create_test_profile();
        registry.add_application(test_profile.clone());
        
        let mut detector = ApplicationDetector::with_registry(registry).unwrap();
        let result = detector.detect_application("test-app").await;
        
        assert!(result.is_ok());
        let detection_result = result.unwrap();
        assert_eq!(detection_result.profile.id, "test-app");
        assert!(!detection_result.detected); // Won't be detected since paths don't exist
    }

    #[tokio::test]
    async fn test_detection_result_serialization() {
        let profile = create_test_profile();
        let result = DetectionResult {
            profile: profile.clone(),
            detected: true,
            detection_method: Some(DetectionMethod::ExecutableCheck),
            found_paths: DetectionPaths {
                executable: Some(PathBuf::from("/Applications/Test.app")),
                config_file: Some(PathBuf::from("/Users/test/.config/test.json")),
                additional_paths: vec![],
            },
            confidence: 0.8,
            messages: vec![
                DetectionMessage {
                    level: MessageLevel::Info,
                    message: "Application detected successfully".to_string(),
                    method: Some(DetectionMethod::ExecutableCheck),
                }
            ],
            detected_at: chrono::Utc::now(),
        };

        let serialized = serde_json::to_string(&result).unwrap();
        let deserialized: DetectionResult = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(result.profile.id, deserialized.profile.id);
        assert_eq!(result.detected, deserialized.detected);
        assert_eq!(result.confidence, deserialized.confidence);
    }

    #[tokio::test]
    async fn test_cache_functionality() {
        let mut detector = ApplicationDetector::new().unwrap();
        
        // Initially empty cache
        let (total, fresh) = detector.get_cache_stats();
        assert_eq!(total, 0);
        assert_eq!(fresh, 0);
        
        // Clear cache (should not panic)
        detector.clear_cache();
        
        let (total, fresh) = detector.get_cache_stats();
        assert_eq!(total, 0);
        assert_eq!(fresh, 0);
    }

    #[tokio::test]
    async fn test_add_custom_application() {
        let mut detector = ApplicationDetector::new().unwrap();
        let initial_count = detector.get_registry().metadata.application_count;
        
        let test_profile = create_test_profile();
        detector.add_custom_application(test_profile);
        
        let new_count = detector.get_registry().metadata.application_count;
        assert_eq!(new_count, initial_count + 1);
        
        assert!(detector.get_registry().get_application("test-app").is_some());
    }

    #[tokio::test]
    async fn test_detection_message_levels() {
        let info_msg = DetectionMessage {
            level: MessageLevel::Info,
            message: "Info message".to_string(),
            method: None,
        };
        
        let warning_msg = DetectionMessage {
            level: MessageLevel::Warning,
            message: "Warning message".to_string(),
            method: Some(DetectionMethod::BundleLookup),
        };
        
        let error_msg = DetectionMessage {
            level: MessageLevel::Error,
            message: "Error message".to_string(),
            method: Some(DetectionMethod::ExecutableCheck),
        };
        
        // Test serialization
        let serialized = serde_json::to_string(&vec![info_msg, warning_msg, error_msg]).unwrap();
        let deserialized: Vec<DetectionMessage> = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(deserialized.len(), 3);
        assert!(matches!(deserialized[0].level, MessageLevel::Info));
        assert!(matches!(deserialized[1].level, MessageLevel::Warning));
        assert!(matches!(deserialized[2].level, MessageLevel::Error));
    }
}
