use crate::core::error::{OcmError, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcmConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub networking: NetworkingConfig,
    pub plc: PlcConfig,
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub p2p_port: u16,
    pub discovery_port: u16,
    pub shutdown_timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub path: PathBuf,
    pub connection_pool_size: u32,
    pub backup_interval_hours: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkingConfig {
    pub max_peers: usize,
    pub heartbeat_interval_seconds: u64,
    pub connection_timeout_seconds: u64,
    pub discovery_interval_seconds: u64,
    pub seed_peers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlcConfig {
    pub directory_url: String,
    pub enable_network_calls: bool,
    pub cache_ttl_hours: u64,
    pub handle: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub format: String, // "json" or "pretty"
    pub log_to_file: bool,
    pub file_path: Option<PathBuf>,
    pub max_file_size_mb: u64,
}

impl Default for OcmConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                p2p_port: 8080,
                discovery_port: 8081,
                shutdown_timeout_seconds: 30,
            },
            database: DatabaseConfig {
                path: PathBuf::from("data/ocm-impl.db"),
                connection_pool_size: 10,
                backup_interval_hours: Some(24),
            },
            networking: NetworkingConfig {
                max_peers: 50,
                heartbeat_interval_seconds: 30,
                connection_timeout_seconds: 10,
                discovery_interval_seconds: 60,
                seed_peers: vec![],
            },
            plc: PlcConfig {
                directory_url: "https://plc.directory".to_string(),
                enable_network_calls: false, // Safe default for development
                cache_ttl_hours: 24,
                handle: None,
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                format: "pretty".to_string(),
                log_to_file: false,
                file_path: None,
                max_file_size_mb: 100,
            },
        }
    }
}

impl OcmConfig {
    pub fn from_file(path: &str) -> Result<Self> {
        let config = config::Config::builder()
            .add_source(config::File::with_name(path))
            .add_source(config::Environment::with_prefix("OCM"))
            .build()
            .map_err(|e| OcmError::Config(format!("Failed to load config: {}", e)))?;

        config
            .try_deserialize()
            .map_err(|e| OcmError::Config(format!("Failed to parse config: {}", e)))
    }

    pub fn from_env() -> Result<Self> {
        let config = config::Config::builder()
            .add_source(config::Environment::with_prefix("OCM"))
            .build()
            .map_err(|e| OcmError::Config(format!("Failed to load environment config: {}", e)))?;

        // Start with defaults and overlay environment variables
        let mut base_config = Self::default();

        // Try to deserialize environment overrides
        if let Ok(env_config) = config.try_deserialize::<OcmConfig>() {
            // Merge with base config (environment takes precedence)
            base_config = env_config;
        }

        Ok(base_config)
    }

    pub fn validate(&self) -> Result<()> {
        // Validate ports
        if self.server.p2p_port == self.server.discovery_port {
            return Err(OcmError::Config(
                "P2P port and discovery port cannot be the same".to_string(),
            ));
        }

        // Validate database path
        if let Some(parent) = self.database.path.parent() {
            if !parent.exists() {
                return Err(OcmError::Config(format!(
                    "Database directory does not exist: {:?}",
                    parent
                )));
            }
        }

        // Validate logging level
        match self.logging.level.to_lowercase().as_str() {
            "trace" | "debug" | "info" | "warn" | "error" => {}
            _ => {
                return Err(OcmError::Config(format!(
                    "Invalid log level: {}",
                    self.logging.level
                )));
            }
        }

        // Validate PLC directory URL
        if self.plc.enable_network_calls {
            url::Url::parse(&self.plc.directory_url)
                .map_err(|e| OcmError::Config(format!("Invalid PLC directory URL: {}", e)))?;
        }

        tracing::info!("Configuration validation passed");
        Ok(())
    }

    pub fn server_address(&self) -> String {
        format!("{}:{}", self.server.host, self.server.p2p_port)
    }

    pub fn discovery_address(&self) -> String {
        format!("{}:{}", self.server.host, self.server.discovery_port)
    }
}