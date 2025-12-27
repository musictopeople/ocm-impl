use crate::config::app::OcmConfig;
use crate::core::error::{OcmError, Result};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub fn init_logging(config: &OcmConfig) -> Result<()> {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .or_else(|_| tracing_subscriber::EnvFilter::try_new(&config.logging.level))
        .map_err(|e| OcmError::Config(format!("Invalid log level: {}", e)))?;

    let subscriber = tracing_subscriber::registry().with(filter);

    match config.logging.format.as_str() {
        "json" => {
            let json_layer = tracing_subscriber::fmt::layer().json();
            subscriber.with(json_layer).init();
        }
        "pretty" => {
            let pretty_layer = tracing_subscriber::fmt::layer().pretty();
            subscriber.with(pretty_layer).init();
        }
        _ => {
            return Err(OcmError::Config(format!(
                "Unsupported log format: {}",
                config.logging.format
            )));
        }
    }

    info!("Logging initialized with level: {}", config.logging.level);
    Ok(())
}
