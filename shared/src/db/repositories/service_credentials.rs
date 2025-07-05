use anyhow::Result;
use serde_json::Value as JsonValue;
use sqlx::PgPool;

use crate::encryption::{EncryptedData, EncryptionService};
use crate::models::ServiceCredentials;

/// Service credentials repository with encryption support
pub struct ServiceCredentialsRepo {
    pool: PgPool,
    encryption_service: EncryptionService,
}

impl ServiceCredentialsRepo {
    pub fn new(pool: PgPool) -> Result<Self> {
        let encryption_service = EncryptionService::new()?;
        Ok(Self {
            pool,
            encryption_service,
        })
    }

    pub async fn get_by_source_id(&self, source_id: &str) -> Result<Option<ServiceCredentials>> {
        let mut creds = sqlx::query_as::<_, ServiceCredentials>(
            "SELECT * FROM service_credentials WHERE source_id = $1",
        )
        .bind(source_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(ref mut creds) = creds {
            self.decrypt_credentials_in_place(creds)?;
        }

        Ok(creds)
    }

    /// Decrypt credentials in place if they are encrypted
    fn decrypt_credentials_in_place(&self, creds: &mut ServiceCredentials) -> Result<()> {
        // Check if credentials are already encrypted (new format)
        if let Some(encrypted_data) = creds.credentials.get("encrypted_data") {
            let encrypted_data: EncryptedData = serde_json::from_value(encrypted_data.clone())?;
            let decrypted_credentials: JsonValue =
                self.encryption_service.decrypt_json(&encrypted_data)?;
            creds.credentials = decrypted_credentials;
        }
        // If no encrypted_data field, credentials are in legacy unencrypted format - leave as is
        Ok(())
    }

    /// Encrypt credentials from application format to database format
    fn encrypt_credentials(&self, creds: &ServiceCredentials) -> Result<JsonValue> {
        let encrypted_data = self.encryption_service.encrypt_json(&creds.credentials)?;
        Ok(serde_json::json!({
            "encrypted_data": encrypted_data,
            "version": 1
        }))
    }

    pub async fn create(&self, creds: ServiceCredentials) -> Result<ServiceCredentials> {
        let encrypted_credentials = self.encrypt_credentials(&creds)?;

        let mut created_creds = sqlx::query_as::<_, ServiceCredentials>(
            r#"
            INSERT INTO service_credentials 
            (id, source_id, provider, auth_type, principal_email, credentials, config, expires_at, last_validated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING *
            "#
        )
        .bind(&creds.id)
        .bind(&creds.source_id)
        .bind(&creds.provider)
        .bind(&creds.auth_type)
        .bind(&creds.principal_email)
        .bind(&encrypted_credentials)
        .bind(&creds.config)
        .bind(&creds.expires_at)
        .bind(&creds.last_validated_at)
        .fetch_one(&self.pool)
        .await?;

        // Decrypt the credentials for return (they come back encrypted from the database)
        self.decrypt_credentials_in_place(&mut created_creds)?;
        Ok(created_creds)
    }

    pub async fn update_last_validated(&self, id: &str) -> Result<()> {
        sqlx::query(
            "UPDATE service_credentials SET last_validated_at = CURRENT_TIMESTAMP WHERE id = $1",
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn delete_by_source_id(&self, source_id: &str) -> Result<()> {
        sqlx::query("DELETE FROM service_credentials WHERE source_id = $1")
            .bind(source_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Update credentials (encrypts the new credentials)
    pub async fn update_credentials(&self, creds: &ServiceCredentials) -> Result<()> {
        let encrypted_credentials = self.encrypt_credentials(creds)?;

        sqlx::query(
            r#"
            UPDATE service_credentials 
            SET credentials = $2, config = $3, expires_at = $4, updated_at = CURRENT_TIMESTAMP
            WHERE id = $1
            "#,
        )
        .bind(&creds.id)
        .bind(&encrypted_credentials)
        .bind(&creds.config)
        .bind(&creds.expires_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Encrypt all existing unencrypted credentials in the database
    pub async fn encrypt_existing_credentials(&self) -> Result<usize> {
        let mut count = 0;

        // Get all credentials that are not encrypted (don't have encrypted_data field)
        let unencrypted_creds = sqlx::query_as::<_, ServiceCredentials>(
            "SELECT * FROM service_credentials WHERE NOT (credentials ? 'encrypted_data')",
        )
        .fetch_all(&self.pool)
        .await?;

        for creds in unencrypted_creds {
            // These credentials are in unencrypted format, encrypt and update them
            self.update_credentials(&creds).await?;
            count += 1;
        }

        Ok(count)
    }
}
