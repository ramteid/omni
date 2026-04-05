use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Configuration for a paperless-ngx source, stored in `Source.config`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperlessConfig {
    /// Base URL of the paperless-ngx instance, e.g. `http://localhost:8000`.
    /// Trailing slashes are stripped automatically.
    pub base_url: String,
    /// Whether periodic sync is enabled.
    #[serde(default = "default_true")]
    pub sync_enabled: bool,
}

fn default_true() -> bool {
    true
}

impl PaperlessConfig {
    pub fn from_source_config(config: &serde_json::Value) -> Result<Self> {
        let mut cfg: Self =
            serde_json::from_value(config.clone()).context("Failed to parse paperless config")?;
        // Normalize trailing slash so URL construction is consistent.
        cfg.base_url = cfg.base_url.trim_end_matches('/').to_string();
        Ok(cfg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_config_parses_base_url() {
        let cfg = PaperlessConfig::from_source_config(&json!({
            "base_url": "http://paperless.local:8000/"
        }))
        .unwrap();
        assert_eq!(cfg.base_url, "http://paperless.local:8000");
        assert!(cfg.sync_enabled);
    }

    #[test]
    fn test_config_sync_enabled_default() {
        let cfg = PaperlessConfig::from_source_config(&json!({
            "base_url": "https://paperless.example.com"
        }))
        .unwrap();
        assert!(cfg.sync_enabled);
    }

    #[test]
    fn test_config_sync_enabled_false() {
        let cfg = PaperlessConfig::from_source_config(&json!({
            "base_url": "https://paperless.example.com",
            "sync_enabled": false
        }))
        .unwrap();
        assert!(!cfg.sync_enabled);
    }

    #[test]
    fn test_config_missing_base_url_fails() {
        let result = PaperlessConfig::from_source_config(&json!({}));
        assert!(result.is_err());
    }
}
