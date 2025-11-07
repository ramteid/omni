use shared::{ConnectorConfig, DatabaseConfig};

#[derive(Debug, Clone)]
pub struct AtlassianConnectorConfig {
    pub base: ConnectorConfig,
    pub database: DatabaseConfig,
}

impl AtlassianConnectorConfig {
    pub fn from_env() -> Self {
        let base = ConnectorConfig::from_env();
        let database = DatabaseConfig::from_env();

        Self { base, database }
    }
}
