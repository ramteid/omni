use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub query_id: String,
    pub query_text: String,
    pub retrieved_docs: Vec<RetrievedDocument>,
    pub relevant_docs: Vec<RelevantDocument>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievedDocument {
    pub doc_id: String,
    pub rank: usize,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelevantDocument {
    pub doc_id: String,
    pub relevance_score: f64, // 0.0 to 1.0, where 1.0 is most relevant
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationMetrics {
    pub query_id: String,
    pub ndcg_at_1: f64,
    pub ndcg_at_5: f64,
    pub ndcg_at_10: f64,
    pub ndcg_at_20: f64,
    pub mrr: f64,
    pub map_at_5: f64,
    pub map_at_10: f64,
    pub map_at_20: f64,
    pub precision_at_1: f64,
    pub precision_at_5: f64,
    pub precision_at_10: f64,
    pub precision_at_20: f64,
    pub recall_at_5: f64,
    pub recall_at_10: f64,
    pub recall_at_20: f64,
    pub num_relevant: usize,
    pub num_retrieved: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedMetrics {
    pub dataset_name: String,
    pub search_mode: String,
    pub total_queries: usize,
    pub mean_ndcg_at_1: f64,
    pub mean_ndcg_at_5: f64,
    pub mean_ndcg_at_10: f64,
    pub mean_ndcg_at_20: f64,
    pub mean_mrr: f64,
    pub mean_map_at_5: f64,
    pub mean_map_at_10: f64,
    pub mean_map_at_20: f64,
    pub mean_precision_at_1: f64,
    pub mean_precision_at_5: f64,
    pub mean_precision_at_10: f64,
    pub mean_precision_at_20: f64,
    pub mean_recall_at_5: f64,
    pub mean_recall_at_10: f64,
    pub mean_recall_at_20: f64,
    pub query_metrics: Vec<EvaluationMetrics>,
}

pub struct MetricsCalculator;

impl MetricsCalculator {
    pub fn calculate_metrics(query_result: &QueryResult) -> EvaluationMetrics {
        let relevant_docs_map: HashMap<String, f64> = query_result
            .relevant_docs
            .iter()
            .map(|doc| (doc.doc_id.clone(), doc.relevance_score))
            .collect();

        let retrieved_docs = &query_result.retrieved_docs;
        let num_relevant = query_result.relevant_docs.len();
        let num_retrieved = retrieved_docs.len();

        EvaluationMetrics {
            query_id: query_result.query_id.clone(),
            ndcg_at_1: Self::calculate_ndcg(&retrieved_docs, &relevant_docs_map, 1),
            ndcg_at_5: Self::calculate_ndcg(&retrieved_docs, &relevant_docs_map, 5),
            ndcg_at_10: Self::calculate_ndcg(&retrieved_docs, &relevant_docs_map, 10),
            ndcg_at_20: Self::calculate_ndcg(&retrieved_docs, &relevant_docs_map, 20),
            mrr: Self::calculate_mrr(&retrieved_docs, &relevant_docs_map),
            map_at_5: Self::calculate_map(&retrieved_docs, &relevant_docs_map, 5),
            map_at_10: Self::calculate_map(&retrieved_docs, &relevant_docs_map, 10),
            map_at_20: Self::calculate_map(&retrieved_docs, &relevant_docs_map, 20),
            precision_at_1: Self::calculate_precision(&retrieved_docs, &relevant_docs_map, 1),
            precision_at_5: Self::calculate_precision(&retrieved_docs, &relevant_docs_map, 5),
            precision_at_10: Self::calculate_precision(&retrieved_docs, &relevant_docs_map, 10),
            precision_at_20: Self::calculate_precision(&retrieved_docs, &relevant_docs_map, 20),
            recall_at_5: Self::calculate_recall(
                &retrieved_docs,
                &relevant_docs_map,
                5,
                num_relevant,
            ),
            recall_at_10: Self::calculate_recall(
                &retrieved_docs,
                &relevant_docs_map,
                10,
                num_relevant,
            ),
            recall_at_20: Self::calculate_recall(
                &retrieved_docs,
                &relevant_docs_map,
                20,
                num_relevant,
            ),
            num_relevant,
            num_retrieved,
        }
    }

    pub fn aggregate_metrics(
        query_metrics: Vec<EvaluationMetrics>,
        dataset_name: String,
        search_mode: String,
    ) -> AggregatedMetrics {
        let total_queries = query_metrics.len();

        if total_queries == 0 {
            return AggregatedMetrics {
                dataset_name,
                search_mode,
                total_queries: 0,
                mean_ndcg_at_1: 0.0,
                mean_ndcg_at_5: 0.0,
                mean_ndcg_at_10: 0.0,
                mean_ndcg_at_20: 0.0,
                mean_mrr: 0.0,
                mean_map_at_5: 0.0,
                mean_map_at_10: 0.0,
                mean_map_at_20: 0.0,
                mean_precision_at_1: 0.0,
                mean_precision_at_5: 0.0,
                mean_precision_at_10: 0.0,
                mean_precision_at_20: 0.0,
                mean_recall_at_5: 0.0,
                mean_recall_at_10: 0.0,
                mean_recall_at_20: 0.0,
                query_metrics,
            };
        }

        let total_queries_f = total_queries as f64;

        AggregatedMetrics {
            dataset_name,
            search_mode,
            total_queries,
            mean_ndcg_at_1: query_metrics.iter().map(|m| m.ndcg_at_1).sum::<f64>()
                / total_queries_f,
            mean_ndcg_at_5: query_metrics.iter().map(|m| m.ndcg_at_5).sum::<f64>()
                / total_queries_f,
            mean_ndcg_at_10: query_metrics.iter().map(|m| m.ndcg_at_10).sum::<f64>()
                / total_queries_f,
            mean_ndcg_at_20: query_metrics.iter().map(|m| m.ndcg_at_20).sum::<f64>()
                / total_queries_f,
            mean_mrr: query_metrics.iter().map(|m| m.mrr).sum::<f64>() / total_queries_f,
            mean_map_at_5: query_metrics.iter().map(|m| m.map_at_5).sum::<f64>() / total_queries_f,
            mean_map_at_10: query_metrics.iter().map(|m| m.map_at_10).sum::<f64>()
                / total_queries_f,
            mean_map_at_20: query_metrics.iter().map(|m| m.map_at_20).sum::<f64>()
                / total_queries_f,
            mean_precision_at_1: query_metrics.iter().map(|m| m.precision_at_1).sum::<f64>()
                / total_queries_f,
            mean_precision_at_5: query_metrics.iter().map(|m| m.precision_at_5).sum::<f64>()
                / total_queries_f,
            mean_precision_at_10: query_metrics.iter().map(|m| m.precision_at_10).sum::<f64>()
                / total_queries_f,
            mean_precision_at_20: query_metrics.iter().map(|m| m.precision_at_20).sum::<f64>()
                / total_queries_f,
            mean_recall_at_5: query_metrics.iter().map(|m| m.recall_at_5).sum::<f64>()
                / total_queries_f,
            mean_recall_at_10: query_metrics.iter().map(|m| m.recall_at_10).sum::<f64>()
                / total_queries_f,
            mean_recall_at_20: query_metrics.iter().map(|m| m.recall_at_20).sum::<f64>()
                / total_queries_f,
            query_metrics,
        }
    }

    fn calculate_ndcg(
        retrieved_docs: &[RetrievedDocument],
        relevant_docs: &HashMap<String, f64>,
        k: usize,
    ) -> f64 {
        let k = k.min(retrieved_docs.len());
        if k == 0 {
            return 0.0;
        }

        // Calculate DCG@k
        let mut dcg = 0.0;
        for (i, doc) in retrieved_docs.iter().take(k).enumerate() {
            if let Some(&relevance) = relevant_docs.get(&doc.doc_id) {
                let gain = relevance;
                let discount = if i == 0 { 1.0 } else { (1.0 + i as f64).log2() };
                dcg += gain / discount;
            }
        }

        // Calculate IDCG@k (Ideal DCG)
        let mut ideal_relevances: Vec<f64> = relevant_docs.values().cloned().collect();
        ideal_relevances.sort_by(|a, b| b.partial_cmp(a).unwrap());

        let mut idcg = 0.0;
        for (i, &relevance) in ideal_relevances.iter().take(k).enumerate() {
            let gain = relevance;
            let discount = if i == 0 { 1.0 } else { (1.0 + i as f64).log2() };
            idcg += gain / discount;
        }

        if idcg == 0.0 {
            0.0
        } else {
            dcg / idcg
        }
    }

    fn calculate_mrr(
        retrieved_docs: &[RetrievedDocument],
        relevant_docs: &HashMap<String, f64>,
    ) -> f64 {
        for (i, doc) in retrieved_docs.iter().enumerate() {
            if relevant_docs.contains_key(&doc.doc_id) {
                return 1.0 / (i + 1) as f64;
            }
        }
        0.0
    }

    fn calculate_map(
        retrieved_docs: &[RetrievedDocument],
        relevant_docs: &HashMap<String, f64>,
        k: usize,
    ) -> f64 {
        let k = k.min(retrieved_docs.len());
        if k == 0 {
            return 0.0;
        }

        let mut sum_precision = 0.0;
        let mut num_relevant_found = 0;

        for (i, doc) in retrieved_docs.iter().take(k).enumerate() {
            if relevant_docs.contains_key(&doc.doc_id) {
                num_relevant_found += 1;
                let precision_at_i = num_relevant_found as f64 / (i + 1) as f64;
                sum_precision += precision_at_i;
            }
        }

        if num_relevant_found == 0 {
            0.0
        } else {
            sum_precision / num_relevant_found as f64
        }
    }

    fn calculate_precision(
        retrieved_docs: &[RetrievedDocument],
        relevant_docs: &HashMap<String, f64>,
        k: usize,
    ) -> f64 {
        let k = k.min(retrieved_docs.len());
        if k == 0 {
            return 0.0;
        }

        let relevant_in_top_k = retrieved_docs
            .iter()
            .take(k)
            .filter(|doc| relevant_docs.contains_key(&doc.doc_id))
            .count();

        relevant_in_top_k as f64 / k as f64
    }

    fn calculate_recall(
        retrieved_docs: &[RetrievedDocument],
        relevant_docs: &HashMap<String, f64>,
        k: usize,
        total_relevant: usize,
    ) -> f64 {
        if total_relevant == 0 {
            return 0.0;
        }

        let k = k.min(retrieved_docs.len());
        let relevant_in_top_k = retrieved_docs
            .iter()
            .take(k)
            .filter(|doc| relevant_docs.contains_key(&doc.doc_id))
            .count();

        relevant_in_top_k as f64 / total_relevant as f64
    }
}

impl AggregatedMetrics {
    pub fn save_to_file(&self, path: &str) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn print_summary(&self) {
        println!("\n=== Benchmark Results Summary ===");
        println!("Dataset: {}", self.dataset_name);
        println!("Search Mode: {}", self.search_mode);
        println!("Total Queries: {}", self.total_queries);
        println!();
        println!("nDCG Metrics:");
        println!("  nDCG@1:  {:.4}", self.mean_ndcg_at_1);
        println!("  nDCG@5:  {:.4}", self.mean_ndcg_at_5);
        println!("  nDCG@10: {:.4}", self.mean_ndcg_at_10);
        println!("  nDCG@20: {:.4}", self.mean_ndcg_at_20);
        println!();
        println!("Other Metrics:");
        println!("  MRR:     {:.4}", self.mean_mrr);
        println!("  MAP@10:  {:.4}", self.mean_map_at_10);
        println!("  P@1:     {:.4}", self.mean_precision_at_1);
        println!("  P@5:     {:.4}", self.mean_precision_at_5);
        println!("  P@10:    {:.4}", self.mean_precision_at_10);
        println!("  R@10:    {:.4}", self.mean_recall_at_10);
        println!("================================\n");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_ndcg() {
        let retrieved_docs = vec![
            RetrievedDocument {
                doc_id: "doc1".to_string(),
                rank: 1,
                score: 0.9,
            },
            RetrievedDocument {
                doc_id: "doc2".to_string(),
                rank: 2,
                score: 0.8,
            },
            RetrievedDocument {
                doc_id: "doc3".to_string(),
                rank: 3,
                score: 0.7,
            },
        ];

        let mut relevant_docs = HashMap::new();
        relevant_docs.insert("doc1".to_string(), 1.0);
        relevant_docs.insert("doc3".to_string(), 0.5);

        let ndcg = MetricsCalculator::calculate_ndcg(&retrieved_docs, &relevant_docs, 3);
        assert!(ndcg > 0.0 && ndcg <= 1.0);
    }

    #[test]
    fn test_calculate_mrr() {
        let retrieved_docs = vec![
            RetrievedDocument {
                doc_id: "doc1".to_string(),
                rank: 1,
                score: 0.9,
            },
            RetrievedDocument {
                doc_id: "doc2".to_string(),
                rank: 2,
                score: 0.8,
            },
            RetrievedDocument {
                doc_id: "doc3".to_string(),
                rank: 3,
                score: 0.7,
            },
        ];

        let mut relevant_docs = HashMap::new();
        relevant_docs.insert("doc2".to_string(), 1.0);

        let mrr = MetricsCalculator::calculate_mrr(&retrieved_docs, &relevant_docs);
        assert_eq!(mrr, 0.5); // First relevant doc at rank 2, so MRR = 1/2 = 0.5
    }

    #[test]
    fn test_calculate_precision() {
        let retrieved_docs = vec![
            RetrievedDocument {
                doc_id: "doc1".to_string(),
                rank: 1,
                score: 0.9,
            },
            RetrievedDocument {
                doc_id: "doc2".to_string(),
                rank: 2,
                score: 0.8,
            },
            RetrievedDocument {
                doc_id: "doc3".to_string(),
                rank: 3,
                score: 0.7,
            },
        ];

        let mut relevant_docs = HashMap::new();
        relevant_docs.insert("doc1".to_string(), 1.0);
        relevant_docs.insert("doc3".to_string(), 1.0);

        let precision = MetricsCalculator::calculate_precision(&retrieved_docs, &relevant_docs, 3);
        assert_eq!(precision, 2.0 / 3.0); // 2 relevant docs out of 3 retrieved
    }
}
