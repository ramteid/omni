use crate::evaluator::metrics::AggregatedMetrics;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkReport {
    pub timestamp: String,
    pub summary: ReportSummary,
    pub detailed_results: Vec<AggregatedMetrics>,
    pub comparative_analysis: Option<ComparativeAnalysis>,
    pub statistical_tests: Option<StatisticalTestResults>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSummary {
    pub total_datasets: usize,
    pub total_queries: usize,
    pub search_modes_tested: Vec<String>,
    pub best_performing_mode: String,
    pub best_ndcg_10: f64,
    pub best_mrr: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparativeAnalysis {
    pub pairwise_comparisons: Vec<PairwiseComparison>,
    pub ranking_by_metric: HashMap<String, Vec<(String, f64)>>,
    pub improvement_analysis: Vec<ImprovementAnalysis>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairwiseComparison {
    pub method_a: String,
    pub method_b: String,
    pub ndcg_10_diff: f64,
    pub mrr_diff: f64,
    pub better_method: String,
    pub significance: String, // "significant", "not_significant", "inconclusive"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImprovementAnalysis {
    pub baseline_method: String,
    pub comparison_method: String,
    pub relative_improvement_ndcg: f64,
    pub relative_improvement_mrr: f64,
    pub absolute_improvement_ndcg: f64,
    pub absolute_improvement_mrr: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatisticalTestResults {
    pub t_test_results: Vec<TTestResult>,
    pub wilcoxon_results: Vec<WilcoxonResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TTestResult {
    pub method_a: String,
    pub method_b: String,
    pub metric: String,
    pub t_statistic: f64,
    pub p_value: f64,
    pub significant: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WilcoxonResult {
    pub method_a: String,
    pub method_b: String,
    pub metric: String,
    pub statistic: f64,
    pub p_value: f64,
    pub significant: bool,
}

pub struct BenchmarkReporter {
    results_dir: String,
}

impl BenchmarkReporter {
    pub fn new(results_dir: String) -> Self {
        Self { results_dir }
    }

    pub async fn generate_json_report(&self) -> Result<BenchmarkReport> {
        let results = self.load_all_results().await?;
        let report = self.create_comprehensive_report(results)?;
        Ok(report)
    }

    pub async fn generate_html_report(&self) -> Result<String> {
        let report = self.generate_json_report().await?;
        let html_content = self.create_html_report(&report)?;

        let html_path = format!("{}/benchmark_report.html", self.results_dir);
        fs::write(&html_path, html_content)?;

        info!("HTML report generated: {}", html_path);
        Ok(html_path)
    }

    pub async fn generate_csv_report(&self) -> Result<String> {
        let results = self.load_all_results().await?;
        let csv_content = self.create_csv_report(&results)?;

        let csv_path = format!("{}/benchmark_results.csv", self.results_dir);
        fs::write(&csv_path, csv_content)?;

        info!("CSV report generated: {}", csv_path);
        Ok(csv_path)
    }

    async fn load_all_results(&self) -> Result<Vec<AggregatedMetrics>> {
        let mut results = Vec::new();

        if !Path::new(&self.results_dir).exists() {
            return Err(anyhow::anyhow!(
                "Results directory not found: {}",
                self.results_dir
            ));
        }

        for entry in fs::read_dir(&self.results_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(metrics) = serde_json::from_str::<AggregatedMetrics>(&content) {
                        results.push(metrics);
                    }
                }
            }
        }

        info!("Loaded {} result files", results.len());
        Ok(results)
    }

    fn create_comprehensive_report(
        &self,
        results: Vec<AggregatedMetrics>,
    ) -> Result<BenchmarkReport> {
        let summary = self.create_summary(&results);
        let comparative_analysis = if results.len() > 1 {
            Some(self.create_comparative_analysis(&results)?)
        } else {
            None
        };

        let statistical_tests = if results.len() > 1 {
            Some(self.perform_statistical_tests(&results)?)
        } else {
            None
        };

        Ok(BenchmarkReport {
            timestamp: chrono::Utc::now().to_rfc3339(),
            summary,
            detailed_results: results,
            comparative_analysis,
            statistical_tests,
        })
    }

    fn create_summary(&self, results: &[AggregatedMetrics]) -> ReportSummary {
        let total_queries = results.iter().map(|r| r.total_queries).sum();
        let search_modes: Vec<String> = results.iter().map(|r| r.search_mode.clone()).collect();

        let best_result = results.iter().max_by(|a, b| {
            a.mean_ndcg_at_10
                .partial_cmp(&b.mean_ndcg_at_10)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let (best_mode, best_ndcg, best_mrr) = if let Some(best) = best_result {
            (
                best.search_mode.clone(),
                best.mean_ndcg_at_10,
                best.mean_mrr,
            )
        } else {
            ("None".to_string(), 0.0, 0.0)
        };

        ReportSummary {
            total_datasets: results.len(),
            total_queries,
            search_modes_tested: search_modes,
            best_performing_mode: best_mode,
            best_ndcg_10: best_ndcg,
            best_mrr: best_mrr,
        }
    }

    fn create_comparative_analysis(
        &self,
        results: &[AggregatedMetrics],
    ) -> Result<ComparativeAnalysis> {
        let mut pairwise_comparisons = Vec::new();
        let mut ranking_by_metric = HashMap::new();
        let mut improvement_analysis = Vec::new();

        // Generate pairwise comparisons
        for i in 0..results.len() {
            for j in i + 1..results.len() {
                let result_a = &results[i];
                let result_b = &results[j];

                let ndcg_diff = result_b.mean_ndcg_at_10 - result_a.mean_ndcg_at_10;
                let mrr_diff = result_b.mean_mrr - result_a.mean_mrr;

                let better_method = if ndcg_diff > 0.0 {
                    result_b.search_mode.clone()
                } else {
                    result_a.search_mode.clone()
                };

                pairwise_comparisons.push(PairwiseComparison {
                    method_a: result_a.search_mode.clone(),
                    method_b: result_b.search_mode.clone(),
                    ndcg_10_diff: ndcg_diff,
                    mrr_diff,
                    better_method,
                    significance: "not_tested".to_string(), // Will be updated with statistical tests
                });
            }
        }

        // Create rankings by metric
        let mut ndcg_ranking: Vec<(String, f64)> = results
            .iter()
            .map(|r| (r.search_mode.clone(), r.mean_ndcg_at_10))
            .collect();
        ndcg_ranking.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let mut mrr_ranking: Vec<(String, f64)> = results
            .iter()
            .map(|r| (r.search_mode.clone(), r.mean_mrr))
            .collect();
        mrr_ranking.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        ranking_by_metric.insert("ndcg@10".to_string(), ndcg_ranking);
        ranking_by_metric.insert("mrr".to_string(), mrr_ranking);

        // Generate improvement analysis (using first result as baseline)
        if !results.is_empty() {
            let baseline = &results[0];
            for result in results.iter().skip(1) {
                let rel_imp_ndcg = if baseline.mean_ndcg_at_10 > 0.0 {
                    ((result.mean_ndcg_at_10 - baseline.mean_ndcg_at_10) / baseline.mean_ndcg_at_10)
                        * 100.0
                } else {
                    0.0
                };

                let rel_imp_mrr = if baseline.mean_mrr > 0.0 {
                    ((result.mean_mrr - baseline.mean_mrr) / baseline.mean_mrr) * 100.0
                } else {
                    0.0
                };

                improvement_analysis.push(ImprovementAnalysis {
                    baseline_method: baseline.search_mode.clone(),
                    comparison_method: result.search_mode.clone(),
                    relative_improvement_ndcg: rel_imp_ndcg,
                    relative_improvement_mrr: rel_imp_mrr,
                    absolute_improvement_ndcg: result.mean_ndcg_at_10 - baseline.mean_ndcg_at_10,
                    absolute_improvement_mrr: result.mean_mrr - baseline.mean_mrr,
                });
            }
        }

        Ok(ComparativeAnalysis {
            pairwise_comparisons,
            ranking_by_metric,
            improvement_analysis,
        })
    }

    fn perform_statistical_tests(
        &self,
        _results: &[AggregatedMetrics],
    ) -> Result<StatisticalTestResults> {
        // For a complete implementation, we would perform actual statistical tests
        // This would require access to individual query results for each method
        // For now, we'll return empty results as a placeholder

        Ok(StatisticalTestResults {
            t_test_results: Vec::new(),
            wilcoxon_results: Vec::new(),
        })
    }

    fn create_html_report(&self, report: &BenchmarkReport) -> Result<String> {
        let html = format!(
            r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Clio Search Relevance Benchmark Report</title>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 40px; line-height: 1.6; }}
        .header {{ background: #f4f4f4; padding: 20px; border-radius: 5px; margin-bottom: 30px; }}
        .metric-table {{ width: 100%; border-collapse: collapse; margin-bottom: 30px; }}
        .metric-table th, .metric-table td {{ border: 1px solid #ddd; padding: 12px; text-align: left; }}
        .metric-table th {{ background-color: #f2f2f2; }}
        .best-score {{ background-color: #e8f5e8; font-weight: bold; }}
        .improvement-positive {{ color: #28a745; }}
        .improvement-negative {{ color: #dc3545; }}
        .section {{ margin-bottom: 40px; }}
        h1, h2, h3 {{ color: #333; }}
        .timestamp {{ color: #666; font-size: 0.9em; }}
    </style>
</head>
<body>
    <div class="header">
        <h1>Clio Search Relevance Benchmark Report</h1>
        <p class="timestamp">Generated: {}</p>
    </div>

    <div class="section">
        <h2>Summary</h2>
        <p><strong>Total Datasets:</strong> {}</p>
        <p><strong>Total Queries:</strong> {}</p>
        <p><strong>Search Modes Tested:</strong> {}</p>
        <p><strong>Best Performing Mode:</strong> {} (nDCG@10: {:.4})</p>
    </div>

    <div class="section">
        <h2>Detailed Results</h2>
        <table class="metric-table">
            <thead>
                <tr>
                    <th>Search Mode</th>
                    <th>Dataset</th>
                    <th>Queries</th>
                    <th>nDCG@1</th>
                    <th>nDCG@5</th>
                    <th>nDCG@10</th>
                    <th>MRR</th>
                    <th>MAP@10</th>
                    <th>P@10</th>
                    <th>R@10</th>
                </tr>
            </thead>
            <tbody>
                {}
            </tbody>
        </table>
    </div>

    {}

    <div class="section">
        <h2>Raw Data</h2>
        <pre><code>{}</code></pre>
    </div>
</body>
</html>
            "#,
            report.timestamp,
            report.summary.total_datasets,
            report.summary.total_queries,
            report.summary.search_modes_tested.join(", "),
            report.summary.best_performing_mode,
            report.summary.best_ndcg_10,
            self.create_results_table_rows(&report.detailed_results),
            self.create_comparative_analysis_html(&report.comparative_analysis),
            serde_json::to_string_pretty(report)?
        );

        Ok(html)
    }

    fn create_results_table_rows(&self, results: &[AggregatedMetrics]) -> String {
        results
            .iter()
            .map(|r| {
                format!(
                    "<tr><td>{}</td><td>{}</td><td>{}</td><td>{:.4}</td><td>{:.4}</td><td>{:.4}</td><td>{:.4}</td><td>{:.4}</td><td>{:.4}</td><td>{:.4}</td></tr>",
                    r.search_mode,
                    r.dataset_name,
                    r.total_queries,
                    r.mean_ndcg_at_1,
                    r.mean_ndcg_at_5,
                    r.mean_ndcg_at_10,
                    r.mean_mrr,
                    r.mean_map_at_10,
                    r.mean_precision_at_10,
                    r.mean_recall_at_10
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn create_comparative_analysis_html(&self, analysis: &Option<ComparativeAnalysis>) -> String {
        if let Some(analysis) = analysis {
            format!(
                r#"
    <div class="section">
        <h2>Comparative Analysis</h2>
        <h3>Improvement Analysis</h3>
        <table class="metric-table">
            <thead>
                <tr>
                    <th>Baseline</th>
                    <th>Comparison</th>
                    <th>nDCG@10 Improvement</th>
                    <th>MRR Improvement</th>
                </tr>
            </thead>
            <tbody>
                {}
            </tbody>
        </table>
    </div>
                "#,
                analysis
                    .improvement_analysis
                    .iter()
                    .map(|imp| {
                        let ndcg_class = if imp.relative_improvement_ndcg > 0.0 {
                            "improvement-positive"
                        } else {
                            "improvement-negative"
                        };
                        let mrr_class = if imp.relative_improvement_mrr > 0.0 {
                            "improvement-positive"
                        } else {
                            "improvement-negative"
                        };

                        format!(
                            r#"<tr><td>{}</td><td>{}</td><td class="{}">{:+.2}%</td><td class="{}">{:+.2}%</td></tr>"#,
                            imp.baseline_method,
                            imp.comparison_method,
                            ndcg_class,
                            imp.relative_improvement_ndcg,
                            mrr_class,
                            imp.relative_improvement_mrr
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        } else {
            String::new()
        }
    }

    fn create_csv_report(&self, results: &[AggregatedMetrics]) -> Result<String> {
        let mut csv = String::new();

        // Header
        csv.push_str("search_mode,dataset,total_queries,ndcg_1,ndcg_5,ndcg_10,ndcg_20,mrr,map_5,map_10,map_20,precision_1,precision_5,precision_10,precision_20,recall_5,recall_10,recall_20\n");

        // Data rows
        for result in results {
            csv.push_str(&format!(
                "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
                result.search_mode,
                result.dataset_name,
                result.total_queries,
                result.mean_ndcg_at_1,
                result.mean_ndcg_at_5,
                result.mean_ndcg_at_10,
                result.mean_ndcg_at_20,
                result.mean_mrr,
                result.mean_map_at_5,
                result.mean_map_at_10,
                result.mean_map_at_20,
                result.mean_precision_at_1,
                result.mean_precision_at_5,
                result.mean_precision_at_10,
                result.mean_precision_at_20,
                result.mean_recall_at_5,
                result.mean_recall_at_10,
                result.mean_recall_at_20,
            ));
        }

        Ok(csv)
    }
}
