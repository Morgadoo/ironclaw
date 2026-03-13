//! Analysis engine that orchestrates gap detection using BM25 module
//! discovery and LLM-as-Judge structured output.

use std::sync::Arc;

use tokio::sync::RwLock;
use uuid::Uuid;

use project_e_core::config::AnalysisConfig;
use project_e_core::error::Result;
use project_e_core::types::ModuleMetadata;
use project_e_event_bus::EventBus;
use project_e_event_bus::protocol::CognitiveEvent;

use crate::bm25::ModuleBm25Index;
use crate::gap_report::GapReport;

/// Orchestrates gap detection combining BM25 module discovery,
/// pgvector cosine reranking, and LLM-as-Judge structured output.
pub struct AnalysisEngine {
    bm25_index: RwLock<ModuleBm25Index>,
    config: AnalysisConfig,
    event_bus: Arc<EventBus>,
}

impl AnalysisEngine {
    pub fn new(config: AnalysisConfig, event_bus: Arc<EventBus>) -> Self {
        Self {
            bm25_index: RwLock::new(ModuleBm25Index::build(&[])),
            config,
            event_bus,
        }
    }

    /// Rebuild the BM25 index from the current module registry.
    pub async fn rebuild_index(&self, modules: &[ModuleMetadata]) {
        let index = ModuleBm25Index::build(modules);
        *self.bm25_index.write().await = index;
    }

    /// Find modules relevant to a query using BM25 pre-filtering.
    pub async fn find_relevant_modules(&self, query: &str) -> Vec<(Uuid, f32)> {
        let index = self.bm25_index.read().await;
        index.query(query, self.config.bm25_top_k)
    }

    /// Run the full analysis phase for a cognitive cycle.
    ///
    /// In a complete implementation this would:
    /// 1. Rebuild BM25 index from current registry
    /// 2. For each high-relevance data item, find relevant modules via BM25
    /// 3. Rerank with pgvector cosine similarity
    /// 4. Run LLM-as-Judge for gap detection
    /// 5. Filter by confidence threshold
    /// 6. Emit GapReportReady event
    pub async fn analyze(
        &self,
        cycle_id: Uuid,
        gap_reports: Vec<GapReport>,
    ) -> Result<Vec<GapReport>> {
        // Filter by confidence threshold
        let filtered: Vec<GapReport> = gap_reports
            .into_iter()
            .filter(|r| r.confidence >= self.config.confidence_threshold)
            .take(self.config.max_gap_reports_per_cycle)
            .collect();

        let gap_count = filtered
            .iter()
            .filter(|r| r.classification == project_e_core::types::GapClassification::Gap)
            .count();
        let conflict_count = filtered
            .iter()
            .filter(|r| r.classification == project_e_core::types::GapClassification::Conflict)
            .count();
        let confirmation_count = filtered
            .iter()
            .filter(|r| r.classification == project_e_core::types::GapClassification::Confirmation)
            .count();

        self.event_bus
            .emit(CognitiveEvent::GapReportReady {
                cycle_id,
                report_id: Uuid::new_v4(),
                gap_count,
                conflict_count,
                confirmation_count,
            })
            .await?;

        Ok(filtered)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use project_e_core::types::*;

    #[tokio::test]
    async fn analysis_filters_by_confidence() {
        let event_bus = Arc::new(EventBus::in_memory(64));
        let config = AnalysisConfig {
            confidence_threshold: 0.7,
            ..AnalysisConfig::default()
        };
        let engine = AnalysisEngine::new(config, event_bus);

        let cycle_id = Uuid::new_v4();
        let reports = vec![
            crate::gap_report::GapReport::new(
                cycle_id,
                GapClassification::Gap,
                0.9,
                CynefinDomain::Simple,
                WardleyStage::Product,
                Priority::High,
                LifecycleAction::Create,
                "High confidence gap".to_string(),
            ),
            crate::gap_report::GapReport::new(
                cycle_id,
                GapClassification::Gap,
                0.3,
                CynefinDomain::Chaotic,
                WardleyStage::Genesis,
                Priority::Low,
                LifecycleAction::Create,
                "Low confidence gap".to_string(),
            ),
        ];

        let result = engine.analyze(cycle_id, reports).await.unwrap();
        assert_eq!(result.len(), 1);
        assert!(result[0].confidence >= 0.7);
    }
}
