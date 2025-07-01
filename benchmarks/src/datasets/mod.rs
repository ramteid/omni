pub mod beir;
pub mod custom;
pub mod msmarco;

pub use beir::*;
pub use custom::*;
pub use msmarco::*;

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dataset {
    pub name: String,
    pub queries: Vec<Query>,
    pub documents: Vec<Document>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Query {
    pub id: String,
    pub text: String,
    pub relevant_docs: Vec<RelevantDoc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub title: String,
    pub content: String,
    pub metadata: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelevantDoc {
    pub doc_id: String,
    pub relevance_score: f64,
}

#[async_trait]
pub trait DatasetLoader: Send + Sync {
    async fn download(&self) -> Result<()>;
    async fn load_dataset(&self) -> Result<Dataset>;
    fn get_name(&self) -> String;
    fn get_cache_dir(&self) -> String;
}
