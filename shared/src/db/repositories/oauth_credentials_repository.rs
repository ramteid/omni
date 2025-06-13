use crate::{
    db::error::DatabaseError,
    models::{OAuthCredentials, OAuthProvider},
    traits::Repository,
};
use async_trait::async_trait;
use sqlx::PgPool;

pub struct OAuthCredentialsRepository {
    pool: PgPool,
}

impl OAuthCredentialsRepository {
    pub fn new(pool: &PgPool) -> Self {
        Self { pool: pool.clone() }
    }

    pub async fn find_by_source_id(&self, source_id: &str) -> Result<Vec<OAuthCredentials>, DatabaseError> {
        let credentials = sqlx::query_as::<_, OAuthCredentials>(
            "SELECT * FROM oauth_credentials WHERE source_id = $1 ORDER BY created_at DESC"
        )
        .bind(source_id)
        .fetch_all(&self.pool)
        .await?;
        
        Ok(credentials)
    }

    pub async fn find_by_source_and_provider(
        &self,
        source_id: &str,
        provider: OAuthProvider,
    ) -> Result<Option<OAuthCredentials>, DatabaseError> {
        let credential = sqlx::query_as::<_, OAuthCredentials>(
            "SELECT * FROM oauth_credentials WHERE source_id = $1 AND provider = $2"
        )
        .bind(source_id)
        .bind(provider)
        .fetch_optional(&self.pool)
        .await?;
        
        Ok(credential)
    }

    pub async fn find_expiring_tokens(&self, hours_ahead: i32) -> Result<Vec<OAuthCredentials>, DatabaseError> {
        let credentials = sqlx::query_as::<_, OAuthCredentials>(
            "SELECT * FROM oauth_credentials 
             WHERE expires_at IS NOT NULL 
             AND expires_at <= NOW() + INTERVAL '1 hours' * $1"
        )
        .bind(hours_ahead)
        .fetch_all(&self.pool)
        .await?;
        
        Ok(credentials)
    }

    pub async fn update_tokens(
        &self,
        id: &str,
        access_token: Option<&str>,
        refresh_token: Option<&str>,
        expires_at: Option<sqlx::types::time::OffsetDateTime>,
    ) -> Result<OAuthCredentials, DatabaseError> {
        let credential = sqlx::query_as::<_, OAuthCredentials>(
            "UPDATE oauth_credentials 
             SET access_token = COALESCE($2, access_token),
                 refresh_token = COALESCE($3, refresh_token),
                 expires_at = COALESCE($4, expires_at),
                 updated_at = NOW()
             WHERE id = $1
             RETURNING *"
        )
        .bind(id)
        .bind(access_token)
        .bind(refresh_token)
        .bind(expires_at)
        .fetch_one(&self.pool)
        .await?;
        
        Ok(credential)
    }
}

#[async_trait]
impl Repository<OAuthCredentials, String> for OAuthCredentialsRepository {
    async fn find_by_id(&self, id: String) -> Result<Option<OAuthCredentials>, DatabaseError> {
        let credential = sqlx::query_as::<_, OAuthCredentials>("SELECT * FROM oauth_credentials WHERE id = $1")
            .bind(&id)
            .fetch_optional(&self.pool)
            .await?;
        
        Ok(credential)
    }

    async fn find_all(&self, limit: i64, offset: i64) -> Result<Vec<OAuthCredentials>, DatabaseError> {
        let credentials = sqlx::query_as::<_, OAuthCredentials>(
            "SELECT * FROM oauth_credentials ORDER BY created_at DESC LIMIT $1 OFFSET $2"
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        
        Ok(credentials)
    }

    async fn create(&self, entity: OAuthCredentials) -> Result<OAuthCredentials, DatabaseError> {
        let created_credential = sqlx::query_as::<_, OAuthCredentials>(
            "INSERT INTO oauth_credentials (
                id, source_id, provider, client_id, client_secret, 
                access_token, refresh_token, token_type, expires_at, 
                scopes, metadata
             ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
             RETURNING *"
        )
        .bind(&entity.id)
        .bind(&entity.source_id)
        .bind(&entity.provider)
        .bind(&entity.client_id)
        .bind(&entity.client_secret)
        .bind(&entity.access_token)
        .bind(&entity.refresh_token)
        .bind(&entity.token_type)
        .bind(&entity.expires_at)
        .bind(&entity.scopes)
        .bind(&entity.metadata)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::Database(db_err) if db_err.is_unique_violation() => {
                DatabaseError::ConstraintViolation("OAuth credentials already exist for this source and provider".to_string())
            }
            _ => DatabaseError::from(e),
        })?;

        Ok(created_credential)
    }

    async fn update(&self, id: String, entity: OAuthCredentials) -> Result<Option<OAuthCredentials>, DatabaseError> {
        let updated_credential = sqlx::query_as::<_, OAuthCredentials>(
            "UPDATE oauth_credentials SET 
                source_id = $2,
                provider = $3,
                client_id = $4,
                client_secret = $5,
                access_token = $6,
                refresh_token = $7,
                token_type = $8,
                expires_at = $9,
                scopes = $10,
                metadata = $11,
                updated_at = NOW()
             WHERE id = $1
             RETURNING *"
        )
        .bind(&id)
        .bind(&entity.source_id)
        .bind(&entity.provider)
        .bind(&entity.client_id)
        .bind(&entity.client_secret)
        .bind(&entity.access_token)
        .bind(&entity.refresh_token)
        .bind(&entity.token_type)
        .bind(&entity.expires_at)
        .bind(&entity.scopes)
        .bind(&entity.metadata)
        .fetch_optional(&self.pool)
        .await?;

        Ok(updated_credential)
    }

    async fn delete(&self, id: String) -> Result<bool, DatabaseError> {
        let result = sqlx::query("DELETE FROM oauth_credentials WHERE id = $1")
            .bind(&id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }
}