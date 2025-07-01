use crate::datasets::{Dataset, DatasetLoader, Document, Query, RelevantDoc};
use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tracing::info;

pub struct CustomDataset {
    data_dir: String,
    generate_synthetic: bool,
    num_synthetic_queries: usize,
    enterprise_domains: Vec<String>,
}

impl CustomDataset {
    pub fn new(data_dir: String) -> Self {
        Self {
            data_dir,
            generate_synthetic: false,
            num_synthetic_queries: 100,
            enterprise_domains: vec![
                "google_drive".to_string(),
                "slack".to_string(),
                "confluence".to_string(),
                "github".to_string(),
            ],
        }
    }

    pub fn with_synthetic_generation(mut self, enable: bool, num_queries: usize) -> Self {
        self.generate_synthetic = enable;
        self.num_synthetic_queries = num_queries;
        self
    }

    pub fn with_enterprise_domains(mut self, domains: Vec<String>) -> Self {
        self.enterprise_domains = domains;
        self
    }

    fn generate_synthetic_enterprise_queries(&self) -> Result<Vec<Query>> {
        let mut queries = Vec::new();

        // Enterprise search query templates
        let query_templates = vec![
            (
                "Find documents about {}",
                vec![
                    "project planning",
                    "budget review",
                    "quarterly results",
                    "team meeting notes",
                    "product roadmap",
                ],
            ),
            (
                "Search for {} from last month",
                vec![
                    "reports",
                    "presentations",
                    "emails",
                    "documents",
                    "meetings",
                ],
            ),
            (
                "Show me {} related to {}",
                vec![
                    "files",
                    "docs",
                    "slides",
                    "notes",
                    "sales",
                    "marketing",
                    "product",
                    "engineering",
                ],
            ),
            (
                "What is the status of {}",
                vec![
                    "project alpha",
                    "Q4 goals",
                    "customer feedback",
                    "product launch",
                    "hiring process",
                ],
            ),
            (
                "Find the latest version of {}",
                vec![
                    "user manual",
                    "API documentation",
                    "design specs",
                    "test results",
                    "financial report",
                ],
            ),
        ];

        for i in 0..self.num_synthetic_queries {
            let query_id = format!("synthetic_{}", i);

            // Generate a realistic enterprise search query
            let query_text = self.generate_enterprise_query(&query_templates, i);

            // Generate some synthetic relevant documents
            let relevant_docs = self.generate_synthetic_relevant_docs(&query_text, i);

            queries.push(Query {
                id: query_id,
                text: query_text,
                relevant_docs,
            });
        }

        info!("Generated {} synthetic enterprise queries", queries.len());
        Ok(queries)
    }

    fn generate_enterprise_query(&self, templates: &[(&str, Vec<&str>)], index: usize) -> String {
        let template_index = index % templates.len();
        let (template, terms) = &templates[template_index];

        if template.contains("{}") {
            let term_index = index % terms.len();
            if let Some(term) = terms.get(term_index) {
                if template.matches("{}").count() == 1 {
                    template.replace("{}", term)
                } else {
                    // Handle templates with multiple placeholders
                    let term1 = terms.get(term_index).unwrap_or(&"documents");
                    let term2 = terms
                        .get((term_index + 1) % terms.len())
                        .unwrap_or(&"project");
                    template.replace("{}", term1).replacen("{}", term2, 1)
                }
            } else {
                template.to_string()
            }
        } else {
            template.to_string()
        }
    }

    fn generate_synthetic_relevant_docs(&self, _query: &str, index: usize) -> Vec<RelevantDoc> {
        // Generate 1-3 relevant documents per query
        let num_docs = (index % 3) + 1;
        let mut relevant_docs = Vec::new();

        for i in 0..num_docs {
            relevant_docs.push(RelevantDoc {
                doc_id: format!("synthetic_doc_{}_{}", index, i),
                relevance_score: match i {
                    0 => 1.0, // Highly relevant
                    1 => 0.7, // Moderately relevant
                    _ => 0.3, // Somewhat relevant
                },
            });
        }

        relevant_docs
    }

    fn generate_synthetic_documents(&self) -> Result<Vec<Document>> {
        let mut documents = Vec::new();

        // Enterprise document templates by domain
        let document_templates = vec![
            ("Meeting Notes", "google_drive", "Weekly team meeting discussing project progress, action items, and next steps."),
            ("Project Specification", "confluence", "Detailed technical specification for the new product feature including requirements and timeline."),
            ("Code Review", "github", "Pull request review comments and discussion about implementation changes."),
            ("Sales Report", "slack", "Monthly sales performance review with charts and analysis."),
            ("User Manual", "google_drive", "Complete user guide for the product including setup and troubleshooting."),
            ("Budget Planning", "confluence", "Quarterly budget allocation and expense planning document."),
            ("API Documentation", "github", "REST API documentation with endpoints, parameters, and examples."),
            ("Team Announcement", "slack", "Important company-wide announcement about organizational changes."),
        ];

        for i in 0..self.num_synthetic_queries * 3 {
            let template_index = i % document_templates.len();
            let (title_template, source, content_template) = &document_templates[template_index];

            let mut metadata = HashMap::new();
            metadata.insert("source".to_string(), source.to_string());
            metadata.insert("generated".to_string(), "true".to_string());

            documents.push(Document {
                id: format!("synthetic_doc_{}", i),
                title: format!("{} {}", title_template, i / document_templates.len() + 1),
                content: format!("{} Document ID: {}", content_template, i),
                metadata,
            });
        }

        info!("Generated {} synthetic documents", documents.len());
        Ok(documents)
    }

    fn load_custom_dataset_from_files(&self) -> Result<Dataset> {
        let queries_file = format!("{}/queries.json", self.data_dir);
        let documents_file = format!("{}/documents.json", self.data_dir);

        if !Path::new(&queries_file).exists() || !Path::new(&documents_file).exists() {
            return Err(anyhow::anyhow!(
                "Custom dataset files not found. Expected: {} and {}",
                queries_file,
                documents_file
            ));
        }

        let queries_content = fs::read_to_string(&queries_file)?;
        let documents_content = fs::read_to_string(&documents_file)?;

        let queries: Vec<Query> = serde_json::from_str(&queries_content)?;
        let documents: Vec<Document> = serde_json::from_str(&documents_content)?;

        info!(
            "Loaded custom dataset with {} queries and {} documents",
            queries.len(),
            documents.len()
        );

        Ok(Dataset {
            name: "Custom".to_string(),
            queries,
            documents,
        })
    }

    fn save_synthetic_dataset(&self, dataset: &Dataset) -> Result<()> {
        fs::create_dir_all(&self.data_dir)?;

        let queries_file = format!("{}/synthetic_queries.json", self.data_dir);
        let documents_file = format!("{}/synthetic_documents.json", self.data_dir);

        let queries_json = serde_json::to_string_pretty(&dataset.queries)?;
        let documents_json = serde_json::to_string_pretty(&dataset.documents)?;

        fs::write(&queries_file, queries_json)?;
        fs::write(&documents_file, documents_json)?;

        info!(
            "Saved synthetic dataset to {} and {}",
            queries_file, documents_file
        );
        Ok(())
    }
}

#[async_trait]
impl DatasetLoader for CustomDataset {
    async fn download(&self) -> Result<()> {
        if self.generate_synthetic {
            info!("Generating synthetic enterprise dataset");
            let queries = self.generate_synthetic_enterprise_queries()?;
            let documents = self.generate_synthetic_documents()?;

            let dataset = Dataset {
                name: "Synthetic Enterprise".to_string(),
                queries,
                documents,
            };

            self.save_synthetic_dataset(&dataset)?;
            info!("Synthetic dataset generation completed");
        } else {
            info!("Custom dataset download not required - using existing files");
        }

        Ok(())
    }

    async fn load_dataset(&self) -> Result<Dataset> {
        if self.generate_synthetic {
            let queries = self.generate_synthetic_enterprise_queries()?;
            let documents = self.generate_synthetic_documents()?;

            Ok(Dataset {
                name: "Synthetic Enterprise".to_string(),
                queries,
                documents,
            })
        } else {
            self.load_custom_dataset_from_files()
        }
    }

    fn get_name(&self) -> String {
        if self.generate_synthetic {
            "Synthetic Enterprise".to_string()
        } else {
            "Custom".to_string()
        }
    }

    fn get_cache_dir(&self) -> String {
        self.data_dir.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_synthetic_dataset_generation() {
        let temp_dir = TempDir::new().unwrap();
        let data_dir = temp_dir.path().to_str().unwrap().to_string();

        let dataset_loader = CustomDataset::new(data_dir).with_synthetic_generation(true, 10);

        let dataset = dataset_loader.load_dataset().await.unwrap();

        assert_eq!(dataset.queries.len(), 10);
        assert!(!dataset.documents.is_empty());
        assert_eq!(dataset.name, "Synthetic Enterprise");
    }
}
