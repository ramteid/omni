use crate::DatabasePool;
use sqlx::{PgPool, Row};
use std::io::Read;
use tokio::io::AsyncReadExt;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ContentStorage {
    pool: PgPool,
}

#[derive(Debug, thiserror::Error)]
pub enum ContentStorageError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Content not found")]
    NotFound,
}

impl ContentStorage {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Store content in PostgreSQL Large Objects and return the OID
    pub async fn store_content(&self, content: &[u8]) -> Result<u32, ContentStorageError> {
        let mut tx = self.pool.begin().await?;

        // Create a new large object
        let oid: u32 = sqlx::query_scalar("SELECT lo_create(0)")
            .fetch_one(&mut *tx)
            .await?;

        // Open the large object for writing
        let fd: i32 = sqlx::query_scalar("SELECT lo_open($1, 131072)") // 131072 = INV_WRITE
            .bind(oid as i32)
            .fetch_one(&mut *tx)
            .await?;

        // Write content in chunks
        const CHUNK_SIZE: usize = 8192;
        let mut offset = 0;

        for chunk in content.chunks(CHUNK_SIZE) {
            sqlx::query("SELECT lo_write($1, $2)")
                .bind(fd)
                .bind(chunk)
                .execute(&mut *tx)
                .await?;

            offset += chunk.len();
        }

        // Close the large object
        sqlx::query("SELECT lo_close($1)")
            .bind(fd)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;

        Ok(oid)
    }

    /// Retrieve content from PostgreSQL Large Objects by OID
    pub async fn get_content(&self, oid: u32) -> Result<Vec<u8>, ContentStorageError> {
        let mut tx = self.pool.begin().await?;

        // Check if the large object exists
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM pg_largeobject_metadata WHERE oid = $1)",
        )
        .bind(oid as i32)
        .fetch_one(&mut *tx)
        .await?;

        if !exists {
            return Err(ContentStorageError::NotFound);
        }

        // Open the large object for reading
        let fd: i32 = sqlx::query_scalar("SELECT lo_open($1, 262144)") // 262144 = INV_READ
            .bind(oid as i32)
            .fetch_one(&mut *tx)
            .await?;

        // Read content in chunks
        let mut content = Vec::new();
        const CHUNK_SIZE: usize = 8192;

        loop {
            let chunk: Vec<u8> = sqlx::query_scalar("SELECT lo_read($1, $2)")
                .bind(fd)
                .bind(CHUNK_SIZE as i32)
                .fetch_one(&mut *tx)
                .await?;

            if chunk.is_empty() {
                break;
            }

            content.extend_from_slice(&chunk);
        }

        // Close the large object
        sqlx::query("SELECT lo_close($1)")
            .bind(fd)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;

        Ok(content)
    }

    /// Delete content from PostgreSQL Large Objects by OID
    pub async fn delete_content(&self, oid: u32) -> Result<(), ContentStorageError> {
        let mut tx = self.pool.begin().await?;

        // Delete the large object
        let deleted: i32 = sqlx::query_scalar("SELECT lo_unlink($1)")
            .bind(oid as i32)
            .fetch_one(&mut *tx)
            .await?;

        if deleted != 1 {
            return Err(ContentStorageError::NotFound);
        }

        tx.commit().await?;

        Ok(())
    }

    /// Store content as string (convenience method)
    pub async fn store_text(&self, content: &str) -> Result<u32, ContentStorageError> {
        self.store_content(content.as_bytes()).await
    }

    /// Retrieve content as string (convenience method)
    pub async fn get_text(&self, oid: u32) -> Result<String, ContentStorageError> {
        let bytes = self.get_content(oid).await?;
        Ok(String::from_utf8_lossy(&bytes).to_string())
    }

    /// Get content size without loading the full content
    pub async fn get_content_size(&self, oid: u32) -> Result<i64, ContentStorageError> {
        let size: i64 =
            sqlx::query_scalar("SELECT SUM(length(data)) FROM pg_largeobject WHERE loid = $1")
                .bind(oid as i32)
                .fetch_optional(&self.pool)
                .await?
                .unwrap_or(0);

        Ok(size)
    }

    /// Batch fetch content for multiple OIDs efficiently
    pub async fn batch_get_text(
        &self,
        oids: Vec<u32>,
    ) -> Result<std::collections::HashMap<u32, String>, ContentStorageError> {
        if oids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }

        let mut results = std::collections::HashMap::new();

        // Use a transaction for consistency
        let mut tx = self.pool.begin().await?;

        // Process in chunks to avoid overwhelming the database
        const CHUNK_SIZE: usize = 50;
        for chunk in oids.chunks(CHUNK_SIZE) {
            // Fetch all LOB data for this chunk in a single query
            let oid_params: Vec<i32> = chunk.iter().map(|&oid| oid as i32).collect();
            let placeholders = (1..=oid_params.len())
                .map(|i| format!("${}", i))
                .collect::<Vec<_>>()
                .join(",");

            let query = format!(
                "SELECT loid, data FROM pg_largeobject WHERE loid IN ({}) ORDER BY loid, pageno",
                placeholders
            );

            let mut query_builder = sqlx::query(&query);
            for oid in &oid_params {
                query_builder = query_builder.bind(oid);
            }

            let rows = query_builder.fetch_all(&mut *tx).await?;

            // Group data by OID and reconstruct content
            let mut current_oid: Option<i32> = None;
            let mut current_content = Vec::new();

            for row in rows {
                let oid: i32 = row.get("loid");
                let data: Vec<u8> = row.get("data");

                if current_oid != Some(oid) {
                    // Save previous content if any
                    if let Some(prev_oid) = current_oid {
                        let content_str = String::from_utf8_lossy(&current_content).to_string();
                        results.insert(prev_oid as u32, content_str);
                    }

                    // Start new content
                    current_oid = Some(oid);
                    current_content = data;
                } else {
                    // Append to current content
                    current_content.extend_from_slice(&data);
                }
            }

            // Save the last content
            if let Some(oid) = current_oid {
                let content_str = String::from_utf8_lossy(&current_content).to_string();
                results.insert(oid as u32, content_str);
            }
        }

        tx.commit().await?;
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_environment::TestEnvironment;

    #[tokio::test]
    async fn test_content_storage() {
        let env = TestEnvironment::new().await;
        let content_storage = ContentStorage::new(env.database_pool.clone());

        // Test storing and retrieving content
        let test_content = b"Hello, World! This is a test content.";
        let oid = content_storage.store_content(test_content).await.unwrap();

        let retrieved_content = content_storage.get_content(oid).await.unwrap();
        assert_eq!(test_content, retrieved_content.as_slice());

        // Test content size
        let size = content_storage.get_content_size(oid).await.unwrap();
        assert_eq!(size, test_content.len() as i64);

        // Test text convenience methods
        let text_content = "This is a text content";
        let text_oid = content_storage.store_text(text_content).await.unwrap();

        let retrieved_text = content_storage.get_text(text_oid).await.unwrap();
        assert_eq!(text_content, retrieved_text);

        // Test deletion
        content_storage.delete_content(oid).await.unwrap();

        // Verify content is deleted
        let result = content_storage.get_content(oid).await;
        assert!(matches!(result, Err(ContentStorageError::NotFound)));
    }

    #[tokio::test]
    async fn test_batch_get_text() {
        let env = TestEnvironment::new().await;
        let content_storage = ContentStorage::new(env.database_pool.clone());

        // Store multiple pieces of content
        let content1 = "First document content";
        let content2 = "Second document content";
        let content3 = "Third document content";

        let oid1 = content_storage.store_text(content1).await.unwrap();
        let oid2 = content_storage.store_text(content2).await.unwrap();
        let oid3 = content_storage.store_text(content3).await.unwrap();

        // Batch fetch all content
        let oids = vec![oid1 as u32, oid2 as u32, oid3 as u32];
        let results = content_storage.batch_get_text(oids).await.unwrap();

        // Verify all content is retrieved correctly
        assert_eq!(results.len(), 3);
        assert_eq!(results.get(&(oid1 as u32)).unwrap(), content1);
        assert_eq!(results.get(&(oid2 as u32)).unwrap(), content2);
        assert_eq!(results.get(&(oid3 as u32)).unwrap(), content3);

        // Test with empty OIDs list
        let empty_results = content_storage.batch_get_text(vec![]).await.unwrap();
        assert!(empty_results.is_empty());

        // Test with non-existent OID mixed in
        let mixed_oids = vec![oid1 as u32, 99999u32, oid2 as u32];
        let mixed_results = content_storage.batch_get_text(mixed_oids).await.unwrap();
        assert_eq!(mixed_results.len(), 2);
        assert_eq!(mixed_results.get(&(oid1 as u32)).unwrap(), content1);
        assert_eq!(mixed_results.get(&(oid2 as u32)).unwrap(), content2);
        assert!(!mixed_results.contains_key(&99999u32));
    }
}
