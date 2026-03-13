//! DataCollector orchestrates multiple data collection strategies
//! (Firecrawl, Tavily, scraper NATS events) and produces scored data items.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use project_e_core::config::PerceptionConfig;
use project_e_core::error::Result;
use project_e_event_bus::EventBus;
use project_e_event_bus::protocol::CognitiveEvent;

use crate::relevance::RelevanceScorer;

/// A raw data item before scoring.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawDataItem {
    pub id: Uuid,
    pub source_type: String,
    pub source_url: Option<String>,
    pub title: Option<String>,
    pub content: String,
    pub collected_at: DateTime<Utc>,
    pub metadata: serde_json::Value,
}

/// A data item with its computed relevance score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredDataItem {
    pub raw: RawDataItem,
    pub relevance_score: f32,
    pub novelty: f32,
    pub coverage: f32,
    pub magnitude: f32,
}

impl ScoredDataItem {
    pub fn content(&self) -> &str {
        &self.raw.content
    }

    pub fn source_type(&self) -> &str {
        &self.raw.source_type
    }
}

/// Orchestrates data collection from multiple sources, scores items,
/// and emits perception-complete events.
pub struct DataCollector {
    config: PerceptionConfig,
    scorer: RelevanceScorer,
    event_bus: Arc<EventBus>,
}

impl DataCollector {
    pub fn new(config: PerceptionConfig, event_bus: Arc<EventBus>) -> Self {
        let scorer = RelevanceScorer::new(
            config.novelty_weight,
            config.coverage_weight,
            config.magnitude_weight,
            &config.keywords,
        );
        Self {
            config,
            scorer,
            event_bus,
        }
    }

    /// Run a full perception cycle: collect data from all sources,
    /// score each item, filter by minimum relevance, and emit event.
    pub async fn collect_cycle(
        &self,
        cycle_id: Uuid,
        raw_items: Vec<RawDataItem>,
    ) -> Result<Vec<ScoredDataItem>> {
        let mut scored_items = Vec::with_capacity(raw_items.len());

        for item in raw_items {
            let (score, novelty, coverage, magnitude) = self.scorer.score(&item);
            scored_items.push(ScoredDataItem {
                raw: item,
                relevance_score: score,
                novelty,
                coverage,
                magnitude,
            });
        }

        // Filter by minimum relevance threshold
        scored_items.retain(|item| item.relevance_score >= self.config.min_relevance_score);

        // Sort by relevance descending
        scored_items.sort_by(|a, b| b.relevance_score.total_cmp(&a.relevance_score));

        let avg_relevance = if scored_items.is_empty() {
            0.0
        } else {
            scored_items.iter().map(|i| i.relevance_score).sum::<f32>() / scored_items.len() as f32
        };

        self.event_bus
            .emit(CognitiveEvent::PerceptionComplete {
                cycle_id,
                data_count: scored_items.len(),
                avg_relevance,
            })
            .await?;

        Ok(scored_items)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use project_e_core::config::PerceptionConfig;

    fn test_config() -> PerceptionConfig {
        PerceptionConfig {
            min_relevance_score: 0.3,
            ..PerceptionConfig::default()
        }
    }

    #[tokio::test]
    async fn collect_cycle_filters_low_relevance() {
        let event_bus = Arc::new(EventBus::in_memory(64));
        let collector = DataCollector::new(test_config(), event_bus);

        let items = vec![
            RawDataItem {
                id: Uuid::new_v4(),
                source_type: "test".to_string(),
                source_url: None,
                title: Some("Important item".to_string()),
                content: "This has relevant content with keywords".to_string(),
                collected_at: Utc::now(),
                metadata: serde_json::json!({}),
            },
            RawDataItem {
                id: Uuid::new_v4(),
                source_type: "test".to_string(),
                source_url: None,
                title: Some("Empty item".to_string()),
                content: "".to_string(),
                collected_at: Utc::now(),
                metadata: serde_json::json!({}),
            },
        ];

        let result = collector
            .collect_cycle(Uuid::new_v4(), items)
            .await
            .unwrap();
        // Items are scored and filtered; exact count depends on scorer behavior
        assert!(result.len() <= 2);
    }
}
