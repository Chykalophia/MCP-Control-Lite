use crate::detection::profiles::{ApplicationProfile, ApplicationRegistry, ConfigFormat, DetectionStrategy, DetectionMethod, ApplicationCategory, ApplicationMetadata};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Manual application registration request
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ManualRegistrationRequest {
    /// Application identifier (must be unique)
    pub id: String,
    /// Human-readable application name
    pub name: String,
    /// macOS bundle identifier (optional)
    pub bundle_id: Option<String>,
    /// Primary configuration file path
    pub config_path: String,
    /// Alternative configuration paths
    pub alt_config_paths: Vec<String>,
    /// Configuration file format
    pub config_format: ConfigFormat,
    /// Primary executable paths
    pub executable_paths: Vec<String>,
    /// Alternative executable paths
    pub alt_executable_paths: Vec<String>,
    /// Developer/publisher name
    pub developer: String,
    /// Application category
    pub category: ApplicationCategory,
    /// MCP protocol version supported
    pub mcp_version: String,
    /// Additional notes
    pub notes: Option<String>,
    /// Whether application requires special permissions
    pub requires_permissions: bool,
    /// Custom detection strategy (optional)
    pub detection_strategy: Option<DetectionStrategy>,
}

/// Validation result for manual registration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ValidationResult {
    /// Whether the registration request is valid
    pub is_valid: bool,
    /// Validation errors (if any)
    pub errors: Vec<ValidationError>,
    /// Validation warnings (non-blocking)
    pub warnings: Vec<ValidationWarning>,
    /// Suggested improvements
    pub suggestions: Vec<String>,
}

/// Validation error types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ValidationError {
    /// Field that caused the error
    pub field: String,
    /// Error message
    pub message: String,
    /// Error severity
    pub severity: ErrorSeverity,
}

/// Validation warning types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ValidationWarning {
    /// Field that caused the warning
    pub field: String,
    /// Warning message
    pub message: String,
    /// Suggested action
    pub suggestion: Option<String>,
}

/// Error severity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ErrorSeverity {
    Critical,
    High,
    Medium,
    Low,
}

/// Registration conflict types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RegistrationConflict {
    /// ID already exists
    DuplicateId(String),
    /// Bundle ID already exists
    DuplicateBundleId(String),
    /// Similar application already exists
    SimilarApplication(String),
}

/// Manual application registry manager
pub struct ManualRegistryManager {
    /// Base registry with known applications
    base_registry: ApplicationRegistry,
    /// Custom applications added manually
    custom_applications: HashMap<String, ApplicationProfile>,
    /// Registry file path for persistence
    registry_file_path: Option<PathBuf>,
}

impl ManualRegistryManager {
    /// Create a new manual registry manager
    pub fn new() -> Self {
        Self {
            base_registry: ApplicationRegistry::with_auto_load(),
            custom_applications: HashMap::new(),
            registry_file_path: None,
        }
    }

    /// Create manager with custom registry file path
    pub fn with_registry_file<P: AsRef<Path>>(registry_path: P) -> Result<Self> {
        let mut manager = Self::new();
        manager.registry_file_path = Some(registry_path.as_ref().to_path_buf());
        
        // Try to load existing custom applications
        if registry_path.as_ref().exists() {
            manager.load_custom_applications()?;
        }
        
        Ok(manager)
    }

    /// Validate a manual registration request
    pub fn validate_registration(&self, request: &ManualRegistrationRequest) -> ValidationResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        let mut suggestions = Vec::new();

        // Validate required fields
        if request.id.is_empty() {
            errors.push(ValidationError {
                field: "id".to_string(),
                message: "Application ID cannot be empty".to_string(),
                severity: ErrorSeverity::Critical,
            });
        } else if !self.is_valid_id(&request.id) {
            errors.push(ValidationError {
                field: "id".to_string(),
                message: "Application ID must contain only alphanumeric characters, hyphens, and underscores".to_string(),
                severity: ErrorSeverity::High,
            });
        }

        if request.name.is_empty() {
            errors.push(ValidationError {
                field: "name".to_string(),
                message: "Application name cannot be empty".to_string(),
                severity: ErrorSeverity::Critical,
            });
        }

        if request.config_path.is_empty() {
            errors.push(ValidationError {
                field: "config_path".to_string(),
                message: "Configuration path cannot be empty".to_string(),
                severity: ErrorSeverity::Critical,
            });
        }

        if request.executable_paths.is_empty() {
            warnings.push(ValidationWarning {
                field: "executable_paths".to_string(),
                message: "No executable paths provided - detection may be limited".to_string(),
                suggestion: Some("Add at least one executable path for better detection".to_string()),
            });
        }

        if request.developer.is_empty() {
            warnings.push(ValidationWarning {
                field: "developer".to_string(),
                message: "Developer name not provided".to_string(),
                suggestion: Some("Add developer name for better organization".to_string()),
            });
        }

        // Check for conflicts
        if let Some(conflict) = self.check_conflicts(request) {
            match conflict {
                RegistrationConflict::DuplicateId(existing_id) => {
                    errors.push(ValidationError {
                        field: "id".to_string(),
                        message: format!("Application ID '{}' already exists", existing_id),
                        severity: ErrorSeverity::Critical,
                    });
                }
                RegistrationConflict::DuplicateBundleId(bundle_id) => {
                    warnings.push(ValidationWarning {
                        field: "bundle_id".to_string(),
                        message: format!("Bundle ID '{}' already exists", bundle_id),
                        suggestion: Some("Consider using a different bundle ID or updating the existing application".to_string()),
                    });
                }
                RegistrationConflict::SimilarApplication(similar_name) => {
                    warnings.push(ValidationWarning {
                        field: "name".to_string(),
                        message: format!("Similar application '{}' already exists", similar_name),
                        suggestion: Some("Verify this is not a duplicate registration".to_string()),
                    });
                }
            }
        }

        // Validate paths
        self.validate_paths(request, &mut errors, &mut warnings, &mut suggestions);

        // Validate detection strategy
        if let Some(strategy) = &request.detection_strategy {
            self.validate_detection_strategy(strategy, &mut errors, &mut warnings);
        }

        ValidationResult {
            is_valid: errors.is_empty(),
            errors,
            warnings,
            suggestions,
        }
    }

    /// Register a new application manually
    pub async fn register_application(&mut self, request: ManualRegistrationRequest) -> Result<ApplicationProfile> {
        // Validate the request first
        let validation = self.validate_registration(&request);
        if !validation.is_valid {
            return Err(anyhow::anyhow!(
                "Registration validation failed: {}",
                validation.errors.iter()
                    .map(|e| format!("{}: {}", e.field, e.message))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }

        // Create application profile from request
        let profile = self.create_profile_from_request(request)?;
        
        // Add to custom applications
        self.custom_applications.insert(profile.id.clone(), profile.clone());
        
        // Save to file if path is configured
        if self.registry_file_path.is_some() {
            self.save_custom_applications()?;
        }

        Ok(profile)
    }

    /// Update an existing manually registered application
    pub async fn update_application(&mut self, id: &str, request: ManualRegistrationRequest) -> Result<ApplicationProfile> {
        // Check if application exists in custom registry
        if !self.custom_applications.contains_key(id) {
            return Err(anyhow::anyhow!("Application '{}' not found in custom registry", id));
        }

        // For updates, we need to temporarily remove the existing application to avoid duplicate ID validation
        let existing_app = self.custom_applications.remove(id);
        
        // Validate the updated request
        let validation = self.validate_registration(&request);
        
        // Restore the existing application if validation fails
        if !validation.is_valid {
            if let Some(app) = existing_app {
                self.custom_applications.insert(id.to_string(), app);
            }
            return Err(anyhow::anyhow!(
                "Update validation failed: {}",
                validation.errors.iter()
                    .map(|e| format!("{}: {}", e.field, e.message))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }

        // Create updated profile
        let profile = self.create_profile_from_request(request)?;
        
        // Update in custom applications
        self.custom_applications.insert(id.to_string(), profile.clone());
        
        // Save to file if path is configured
        if self.registry_file_path.is_some() {
            self.save_custom_applications()?;
        }

        Ok(profile)
    }

    /// Remove a manually registered application
    pub fn remove_application(&mut self, id: &str) -> Result<ApplicationProfile> {
        let profile = self.custom_applications.remove(id)
            .ok_or_else(|| anyhow::anyhow!("Application '{}' not found in custom registry", id))?;

        // Save to file if path is configured
        if self.registry_file_path.is_some() {
            self.save_custom_applications()?;
        }

        Ok(profile)
    }

    /// Get all applications (base + custom)
    pub fn get_all_applications(&self) -> Vec<&ApplicationProfile> {
        let mut applications = self.base_registry.get_all_applications();
        applications.extend(self.custom_applications.values());
        applications
    }

    /// Get only custom applications
    pub fn get_custom_applications(&self) -> Vec<&ApplicationProfile> {
        self.custom_applications.values().collect()
    }

    /// Get application by ID (checks both base and custom)
    pub fn get_application(&self, id: &str) -> Option<&ApplicationProfile> {
        self.custom_applications.get(id)
            .or_else(|| self.base_registry.get_application(id))
    }

    /// Check if an application ID is available
    pub fn is_id_available(&self, id: &str) -> bool {
        self.base_registry.get_application(id).is_none() && 
        !self.custom_applications.contains_key(id)
    }

    /// Get registration statistics
    pub fn get_registration_stats(&self) -> RegistrationStats {
        RegistrationStats {
            total_applications: self.base_registry.metadata.application_count + self.custom_applications.len(),
            base_applications: self.base_registry.metadata.application_count,
            custom_applications: self.custom_applications.len(),
            categories: self.get_category_breakdown(),
        }
    }

    // Private helper methods

    fn is_valid_id(&self, id: &str) -> bool {
        id.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    }

    fn check_conflicts(&self, request: &ManualRegistrationRequest) -> Option<RegistrationConflict> {
        // Check for duplicate ID
        if !self.is_id_available(&request.id) {
            return Some(RegistrationConflict::DuplicateId(request.id.clone()));
        }

        // Check for duplicate bundle ID
        if let Some(bundle_id) = &request.bundle_id {
            for app in self.get_all_applications() {
                if app.bundle_id == *bundle_id {
                    return Some(RegistrationConflict::DuplicateBundleId(bundle_id.clone()));
                }
            }
        }

        // Check for similar application names
        for app in self.get_all_applications() {
            if self.are_names_similar(&request.name, &app.name) {
                return Some(RegistrationConflict::SimilarApplication(app.name.clone()));
            }
        }

        None
    }

    fn are_names_similar(&self, name1: &str, name2: &str) -> bool {
        let name1_lower = name1.to_lowercase();
        let name2_lower = name2.to_lowercase();
        
        // Simple similarity check - can be enhanced with more sophisticated algorithms
        name1_lower == name2_lower || 
        name1_lower.contains(&name2_lower) || 
        name2_lower.contains(&name1_lower)
    }

    fn validate_paths(&self, request: &ManualRegistrationRequest, errors: &mut Vec<ValidationError>, warnings: &mut Vec<ValidationWarning>, suggestions: &mut Vec<String>) {
        // Validate config path format
        if !request.config_path.starts_with('/') && !request.config_path.starts_with('~') {
            warnings.push(ValidationWarning {
                field: "config_path".to_string(),
                message: "Configuration path should be absolute or start with ~".to_string(),
                suggestion: Some("Use absolute paths or ~ for home directory".to_string()),
            });
        }

        // Check if paths exist (non-blocking)
        let expanded_config_path = self.expand_path(&request.config_path);
        if let Ok(path) = expanded_config_path {
            if !path.exists() {
                warnings.push(ValidationWarning {
                    field: "config_path".to_string(),
                    message: "Configuration file does not exist".to_string(),
                    suggestion: Some("Verify the path is correct or create the configuration file".to_string()),
                });
            }
        }

        // Validate executable paths
        for (i, exec_path) in request.executable_paths.iter().enumerate() {
            if exec_path.is_empty() {
                errors.push(ValidationError {
                    field: format!("executable_paths[{}]", i),
                    message: "Executable path cannot be empty".to_string(),
                    severity: ErrorSeverity::Medium,
                });
            }
        }

        // Suggest adding alternative paths if none provided
        if request.alt_config_paths.is_empty() && request.alt_executable_paths.is_empty() {
            suggestions.push("Consider adding alternative paths for better detection reliability".to_string());
        }
    }

    fn validate_detection_strategy(&self, strategy: &DetectionStrategy, errors: &mut Vec<ValidationError>, warnings: &mut Vec<ValidationWarning>) {
        if strategy.priority_order.is_empty() {
            errors.push(ValidationError {
                field: "detection_strategy.priority_order".to_string(),
                message: "Detection strategy must have at least one method".to_string(),
                severity: ErrorSeverity::High,
            });
        }

        // Check for inconsistencies
        if strategy.use_bundle_lookup && !strategy.priority_order.contains(&DetectionMethod::BundleLookup) {
            warnings.push(ValidationWarning {
                field: "detection_strategy".to_string(),
                message: "Bundle lookup is enabled but not in priority order".to_string(),
                suggestion: Some("Add BundleLookup to priority_order or disable use_bundle_lookup".to_string()),
            });
        }
    }

    fn create_profile_from_request(&self, request: ManualRegistrationRequest) -> Result<ApplicationProfile> {
        let detection_strategy = request.detection_strategy.unwrap_or_else(|| {
            DetectionStrategy {
                use_bundle_lookup: request.bundle_id.is_some(),
                use_executable_check: !request.executable_paths.is_empty(),
                use_config_check: true,
                use_spotlight: true,
                priority_order: vec![
                    DetectionMethod::ConfigCheck,
                    DetectionMethod::ExecutableCheck,
                    DetectionMethod::BundleLookup,
                    DetectionMethod::SpotlightSearch,
                ],
            }
        });

        let bundle_id = request.bundle_id.unwrap_or_else(|| format!("com.custom.{}", request.id));

        Ok(ApplicationProfile {
            id: request.id,
            name: request.name,
            bundle_id,
            config_path: request.config_path,
            alt_config_paths: request.alt_config_paths,
            config_format: request.config_format,
            executable_paths: request.executable_paths,
            alt_executable_paths: request.alt_executable_paths,
            detection_strategy,
            metadata: ApplicationMetadata {
                version: None,
                developer: request.developer,
                category: request.category,
                mcp_version: request.mcp_version,
                notes: request.notes,
                requires_permissions: request.requires_permissions,
            },
        })
    }

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

    fn get_category_breakdown(&self) -> HashMap<String, usize> {
        let mut categories = HashMap::new();
        
        for app in self.get_all_applications() {
            let category_name = match &app.metadata.category {
                ApplicationCategory::CodeEditor => "Code Editor".to_string(),
                ApplicationCategory::IDE => "IDE".to_string(),
                ApplicationCategory::ChatClient => "Chat Client".to_string(),
                ApplicationCategory::ProductivityTool => "Productivity Tool".to_string(),
                ApplicationCategory::Other(name) => name.clone(),
            };
            
            *categories.entry(category_name).or_insert(0) += 1;
        }
        
        categories
    }

    fn save_custom_applications(&self) -> Result<()> {
        if let Some(path) = &self.registry_file_path {
            let json = serde_json::to_string_pretty(&self.custom_applications)
                .context("Failed to serialize custom applications")?;
            
            std::fs::write(path, json)
                .with_context(|| format!("Failed to write custom applications to {}", path.display()))?;
        }
        Ok(())
    }

    fn load_custom_applications(&mut self) -> Result<()> {
        if let Some(path) = &self.registry_file_path {
            let content = std::fs::read_to_string(path)
                .with_context(|| format!("Failed to read custom applications from {}", path.display()))?;
            
            self.custom_applications = serde_json::from_str(&content)
                .context("Failed to deserialize custom applications")?;
        }
        Ok(())
    }
}

/// Registration statistics
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RegistrationStats {
    /// Total number of applications (base + custom)
    pub total_applications: usize,
    /// Number of base applications
    pub base_applications: usize,
    /// Number of custom applications
    pub custom_applications: usize,
    /// Breakdown by category
    pub categories: HashMap<String, usize>,
}

impl Default for ManualRegistryManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn create_test_request() -> ManualRegistrationRequest {
        ManualRegistrationRequest {
            id: "test-app".to_string(),
            name: "Test Application".to_string(),
            bundle_id: Some("com.test.app".to_string()),
            config_path: "~/test/config.json".to_string(),
            alt_config_paths: vec!["~/.config/test/config.json".to_string()],
            config_format: ConfigFormat::Json,
            executable_paths: vec!["/Applications/Test.app".to_string()],
            alt_executable_paths: vec!["~/Applications/Test.app".to_string()],
            developer: "Test Developer".to_string(),
            category: ApplicationCategory::Other("Test".to_string()),
            mcp_version: "1.0".to_string(),
            notes: Some("Test application for manual registration".to_string()),
            requires_permissions: false,
            detection_strategy: None,
        }
    }

    #[test]
    fn test_manual_registry_manager_creation() {
        let manager = ManualRegistryManager::new();
        assert_eq!(manager.custom_applications.len(), 0);
        assert!(manager.registry_file_path.is_none());
    }

    #[test]
    fn test_validation_valid_request() {
        let manager = ManualRegistryManager::new();
        let request = create_test_request();
        
        let validation = manager.validate_registration(&request);
        assert!(validation.is_valid);
        assert!(validation.errors.is_empty());
    }

    #[test]
    fn test_validation_empty_id() {
        let manager = ManualRegistryManager::new();
        let mut request = create_test_request();
        request.id = String::new();
        
        let validation = manager.validate_registration(&request);
        assert!(!validation.is_valid);
        assert!(!validation.errors.is_empty());
        assert!(validation.errors.iter().any(|e| e.field == "id"));
    }

    #[test]
    fn test_validation_invalid_id() {
        let manager = ManualRegistryManager::new();
        let mut request = create_test_request();
        request.id = "invalid id with spaces".to_string();
        
        let validation = manager.validate_registration(&request);
        assert!(!validation.is_valid);
        assert!(validation.errors.iter().any(|e| e.field == "id"));
    }

    #[tokio::test]
    async fn test_register_application() {
        let mut manager = ManualRegistryManager::new();
        let request = create_test_request();
        
        let result = manager.register_application(request.clone()).await;
        assert!(result.is_ok());
        
        let profile = result.unwrap();
        assert_eq!(profile.id, request.id);
        assert_eq!(profile.name, request.name);
        
        // Check if application was added to custom registry
        assert!(manager.custom_applications.contains_key(&request.id));
    }

    #[tokio::test]
    async fn test_register_duplicate_id() {
        let mut manager = ManualRegistryManager::new();
        let request = create_test_request();
        
        // Register first time
        let result1 = manager.register_application(request.clone()).await;
        assert!(result1.is_ok());
        
        // Try to register again with same ID
        let result2 = manager.register_application(request).await;
        assert!(result2.is_err());
    }

    #[test]
    fn test_is_id_available() {
        let manager = ManualRegistryManager::new();
        
        // Should be available for new ID
        assert!(manager.is_id_available("new-app"));
        
        // Should not be available for existing base registry apps
        assert!(!manager.is_id_available("claude-desktop"));
    }

    #[tokio::test]
    async fn test_update_application() {
        let mut manager = ManualRegistryManager::new();
        let request = create_test_request();
        
        // Register application first
        manager.register_application(request.clone()).await.unwrap();
        
        // Update the application
        let mut updated_request = request.clone();
        updated_request.name = "Updated Test Application".to_string();
        
        let result = manager.update_application(&request.id, updated_request.clone()).await;
        assert!(result.is_ok());
        
        let updated_profile = result.unwrap();
        assert_eq!(updated_profile.name, "Updated Test Application");
    }

    #[test]
    fn test_remove_application() {
        let mut manager = ManualRegistryManager::new();
        let profile = ApplicationProfile {
            id: "test-app".to_string(),
            name: "Test App".to_string(),
            bundle_id: "com.test.app".to_string(),
            config_path: "~/test/config.json".to_string(),
            alt_config_paths: vec![],
            config_format: ConfigFormat::Json,
            executable_paths: vec![],
            alt_executable_paths: vec![],
            detection_strategy: DetectionStrategy {
                use_bundle_lookup: false,
                use_executable_check: false,
                use_config_check: true,
                use_spotlight: false,
                priority_order: vec![DetectionMethod::ConfigCheck],
            },
            metadata: ApplicationMetadata {
                version: None,
                developer: "Test".to_string(),
                category: ApplicationCategory::Other("Test".to_string()),
                mcp_version: "1.0".to_string(),
                notes: None,
                requires_permissions: false,
            },
        };
        
        manager.custom_applications.insert("test-app".to_string(), profile);
        
        let result = manager.remove_application("test-app");
        assert!(result.is_ok());
        assert!(!manager.custom_applications.contains_key("test-app"));
    }

    #[test]
    fn test_get_registration_stats() {
        let manager = ManualRegistryManager::new();
        let stats = manager.get_registration_stats();
        
        assert!(stats.total_applications > 0);
        assert_eq!(stats.custom_applications, 0);
        assert!(stats.base_applications > 0);
        assert!(!stats.categories.is_empty());
    }

    #[test]
    fn test_persistence() {
        let temp_dir = tempdir().unwrap();
        let registry_path = temp_dir.path().join("custom_registry.json");
        
        {
            let mut manager = ManualRegistryManager::with_registry_file(&registry_path).unwrap();
            let profile = ApplicationProfile {
                id: "test-app".to_string(),
                name: "Test App".to_string(),
                bundle_id: "com.test.app".to_string(),
                config_path: "~/test/config.json".to_string(),
                alt_config_paths: vec![],
                config_format: ConfigFormat::Json,
                executable_paths: vec![],
                alt_executable_paths: vec![],
                detection_strategy: DetectionStrategy {
                    use_bundle_lookup: false,
                    use_executable_check: false,
                    use_config_check: true,
                    use_spotlight: false,
                    priority_order: vec![DetectionMethod::ConfigCheck],
                },
                metadata: ApplicationMetadata {
                    version: None,
                    developer: "Test".to_string(),
                    category: ApplicationCategory::Other("Test".to_string()),
                    mcp_version: "1.0".to_string(),
                    notes: None,
                    requires_permissions: false,
                },
            };
            
            manager.custom_applications.insert("test-app".to_string(), profile);
            manager.save_custom_applications().unwrap();
        }
        
        // Load in new manager instance
        let manager2 = ManualRegistryManager::with_registry_file(&registry_path).unwrap();
        assert!(manager2.custom_applications.contains_key("test-app"));
    }

    #[test]
    fn test_validation_result_serialization() {
        let validation = ValidationResult {
            is_valid: false,
            errors: vec![
                ValidationError {
                    field: "id".to_string(),
                    message: "ID cannot be empty".to_string(),
                    severity: ErrorSeverity::Critical,
                }
            ],
            warnings: vec![
                ValidationWarning {
                    field: "developer".to_string(),
                    message: "Developer not specified".to_string(),
                    suggestion: Some("Add developer name".to_string()),
                }
            ],
            suggestions: vec!["Consider adding more paths".to_string()],
        };
        
        let serialized = serde_json::to_string(&validation).unwrap();
        let deserialized: ValidationResult = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(validation, deserialized);
    }
}
