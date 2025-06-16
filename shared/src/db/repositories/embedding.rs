use crate::{
    db::error::DatabaseError,
    models::{Document, Embedding},
};
use pgvector::Vector;
use sqlx::{PgPool, Row};

pub struct EmbeddingRepository {
    pool: PgPool,
}

impl EmbeddingRepository {
    pub fn new(pool: &PgPool) -> Self {
        Self { pool: pool.clone() }
    }

    pub async fn find_by_document_id(
        &self,
        document_id: &str,
    ) -> Result<Vec<Embedding>, DatabaseError> {
        let embeddings = sqlx::query_as::<_, Embedding>(
            r#"
            SELECT id, document_id, chunk_index, chunk_text, embedding, model_name, created_at
            FROM embeddings
            WHERE document_id = $1
            ORDER BY chunk_index
            "#,
        )
        .bind(document_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(embeddings)
    }

    pub async fn create(&self, embedding: Embedding) -> Result<Embedding, DatabaseError> {
        let created_embedding = sqlx::query_as::<_, Embedding>(
            r#"
            INSERT INTO embeddings (id, document_id, chunk_index, chunk_text, embedding, model_name)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, document_id, chunk_index, chunk_text, embedding, model_name, created_at
            "#,
        )
        .bind(&embedding.id)
        .bind(&embedding.document_id)
        .bind(&embedding.chunk_index)
        .bind(&embedding.chunk_text)
        .bind(&embedding.embedding)
        .bind(&embedding.model_name)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::Database(db_err) if db_err.is_unique_violation() => {
                DatabaseError::ConstraintViolation(
                    "Embedding already exists for this document and chunk".to_string(),
                )
            }
            _ => DatabaseError::from(e),
        })?;

        Ok(created_embedding)
    }

    pub async fn bulk_create(&self, embeddings: Vec<Embedding>) -> Result<(), DatabaseError> {
        if embeddings.is_empty() {
            return Ok(());
        }

        let mut tx = self.pool.begin().await?;

        for embedding in embeddings {
            sqlx::query(
                r#"
                INSERT INTO embeddings (id, document_id, chunk_index, chunk_text, embedding, model_name)
                VALUES ($1, $2, $3, $4, $5, $6)
                ON CONFLICT (document_id, chunk_index, model_name) DO UPDATE
                SET chunk_text = EXCLUDED.chunk_text,
                    embedding = EXCLUDED.embedding
                "#,
            )
            .bind(&embedding.id)
            .bind(&embedding.document_id)
            .bind(&embedding.chunk_index)
            .bind(&embedding.chunk_text)
            .bind(&embedding.embedding)
            .bind(&embedding.model_name)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    pub async fn find_similar(
        &self,
        embedding: Vec<f32>,
        limit: i64,
    ) -> Result<Vec<(Document, f32)>, DatabaseError> {
        let vector = Vector::from(embedding);

        // Find the best matching chunk for each document to avoid duplicates
        let results = sqlx::query(
            r#"
            WITH ranked_embeddings AS (
                SELECT 
                    e.document_id,
                    e.embedding <=> $1 as distance,
                    e.chunk_text,
                    ROW_NUMBER() OVER (PARTITION BY e.document_id ORDER BY e.embedding <=> $1) as rn
                FROM embeddings e
            )
            SELECT 
                d.id, d.source_id, d.external_id, d.title, d.content,
                d.content_type, d.file_size, d.file_extension, d.url, d.parent_id,
                d.metadata, d.permissions, d.created_at, d.updated_at, d.last_indexed_at,
                re.distance,
                re.chunk_text
            FROM ranked_embeddings re
            JOIN documents d ON re.document_id = d.id
            WHERE re.rn = 1
            ORDER BY re.distance
            LIMIT $2
            "#,
        )
        .bind(&vector)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        let documents_with_scores = results
            .into_iter()
            .map(|row| {
                let doc = Document {
                    id: row.get("id"),
                    source_id: row.get("source_id"),
                    external_id: row.get("external_id"),
                    title: row.get("title"),
                    content: row.get("content"),
                    content_type: row.get("content_type"),
                    file_size: row.get("file_size"),
                    file_extension: row.get("file_extension"),
                    url: row.get("url"),
                    parent_id: row.get("parent_id"),
                    metadata: row.get("metadata"),
                    permissions: row.get("permissions"),
                    created_at: row.get("created_at"),
                    updated_at: row.get("updated_at"),
                    last_indexed_at: row.get("last_indexed_at"),
                };
                let distance: Option<f32> = row.get("distance");
                let similarity = 1.0 - distance.unwrap_or(1.0);
                (doc, similarity)
            })
            .collect();

        Ok(documents_with_scores)
    }

    pub async fn find_similar_chunks(
        &self,
        embedding: Vec<f32>,
        limit: i64,
    ) -> Result<Vec<(Document, f32, String)>, DatabaseError> {
        let vector = Vector::from(embedding);

        let results = sqlx::query(
            r#"
            SELECT 
                d.id, d.source_id, d.external_id, d.title, d.content,
                d.content_type, d.file_size, d.file_extension, d.url, d.parent_id,
                d.metadata, d.permissions, d.created_at, d.updated_at, d.last_indexed_at,
                e.embedding <=> $1 as distance,
                e.chunk_text
            FROM embeddings e
            JOIN documents d ON e.document_id = d.id
            ORDER BY e.embedding <=> $1
            LIMIT $2
            "#,
        )
        .bind(&vector)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        let chunks_with_scores = results
            .into_iter()
            .map(|row| {
                let doc = Document {
                    id: row.get("id"),
                    source_id: row.get("source_id"),
                    external_id: row.get("external_id"),
                    title: row.get("title"),
                    content: row.get("content"),
                    content_type: row.get("content_type"),
                    file_size: row.get("file_size"),
                    file_extension: row.get("file_extension"),
                    url: row.get("url"),
                    parent_id: row.get("parent_id"),
                    metadata: row.get("metadata"),
                    permissions: row.get("permissions"),
                    created_at: row.get("created_at"),
                    updated_at: row.get("updated_at"),
                    last_indexed_at: row.get("last_indexed_at"),
                };
                let distance: Option<f32> = row.get("distance");
                let similarity = 1.0 - distance.unwrap_or(1.0);
                let chunk_text: String = row.get("chunk_text");
                (doc, similarity, chunk_text)
            })
            .collect();

        Ok(chunks_with_scores)
    }

    pub async fn delete_by_document_id(&self, document_id: &str) -> Result<bool, DatabaseError> {
        let result = sqlx::query("DELETE FROM embeddings WHERE document_id = $1")
            .bind(document_id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }
}
