use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Per-source Paperless-ngx configuration stored in `Source.config`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperlessConfig {
    /// Base URL of the Paperless-ngx instance (e.g. `http://paperless:8000`).
    /// Must not have a trailing slash.
    pub url: String,
    /// Whether periodic sync is enabled.
    #[serde(default = "default_true")]
    pub sync_enabled: bool,
}

fn default_true() -> bool {
    true
}

impl PaperlessConfig {
    pub fn from_source_config(config: &serde_json::Value) -> Result<Self> {
        serde_json::from_value(config.clone()).context("Failed to parse Paperless-ngx config")
    }

    /// Returns the normalized base URL (without trailing slash).
    pub fn base_url(&self) -> &str {
        self.url.trim_end_matches('/')
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_config_from_json_defaults() {
        let cfg_json = json!({
            "url": "http://paperless:8000"
        });
        let cfg: PaperlessConfig = serde_json::from_value(cfg_json).unwrap();
        assert_eq!(cfg.url, "http://paperless:8000");
        assert!(cfg.sync_enabled);
    }

    #[test]
    fn test_base_url_strips_trailing_slash() {
        let cfg = PaperlessConfig {
            url: "http://paperless:8000/".to_string(),
            sync_enabled: true,
        };
        assert_eq!(cfg.base_url(), "http://paperless:8000");
    }

    #[test]
    fn test_base_url_no_slash() {
        let cfg = PaperlessConfig {
            url: "http://paperless:8000".to_string(),
            sync_enabled: true,
        };
        assert_eq!(cfg.base_url(), "http://paperless:8000");
    }
}
