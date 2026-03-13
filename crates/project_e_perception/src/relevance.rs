//! Tri-dimensional relevance scoring: novelty × coverage × magnitude.
//!
//! - novelty:   how new/different the content is vs. existing knowledge
//! - coverage:  how far from existing module capabilities (low sim = important)
//! - magnitude: keyword density weighted by configured urgency

use std::collections::{HashMap, HashSet};

use project_e_core::config::KeywordEntry;

use crate::collector::RawDataItem;

/// Tri-dimensional relevance scorer.
pub struct RelevanceScorer {
    novelty_weight: f32,
    coverage_weight: f32,
    magnitude_weight: f32,
    keyword_weights: HashMap<String, f32>,
    existing_topics: HashSet<String>,
}

impl RelevanceScorer {
    pub fn new(
        novelty_weight: f32,
        coverage_weight: f32,
        magnitude_weight: f32,
        keywords: &[KeywordEntry],
    ) -> Self {
        let keyword_weights = keywords
            .iter()
            .map(|k| (k.term.to_ascii_lowercase(), k.weight))
            .collect();

        Self {
            novelty_weight,
            coverage_weight,
            magnitude_weight,
            keyword_weights,
            existing_topics: HashSet::new(),
        }
    }

    /// Update the set of existing module topics for coverage scoring.
    pub fn set_existing_topics(&mut self, topics: HashSet<String>) {
        self.existing_topics = topics;
    }

    /// Score a raw data item. Returns (total_score, novelty, coverage, magnitude).
    pub fn score(&self, item: &RawDataItem) -> (f32, f32, f32, f32) {
        let novelty = self.compute_novelty(item);
        let coverage = self.compute_coverage(item);
        let magnitude = self.compute_magnitude(item);

        let total = self.novelty_weight * novelty
            + self.coverage_weight * coverage
            + self.magnitude_weight * magnitude;

        (total.clamp(0.0, 1.0), novelty, coverage, magnitude)
    }

    /// Novelty: simple content length and uniqueness heuristic.
    /// A production implementation would use TF-IDF against the knowledge base.
    fn compute_novelty(&self, item: &RawDataItem) -> f32 {
        let content = &item.content;
        if content.is_empty() {
            return 0.0;
        }

        // Heuristic: content with more unique tokens is more novel
        let tokens: HashSet<&str> = content.split_whitespace().collect();
        let uniqueness = tokens.len() as f32 / content.split_whitespace().count().max(1) as f32;

        // Content length contribution (longer = more substance, diminishing returns)
        let length_score = (content.len() as f32 / 1000.0).min(1.0);

        (uniqueness * 0.6 + length_score * 0.4).clamp(0.0, 1.0)
    }

    /// Coverage: how far the content is from existing module topics.
    /// A production implementation would use pgvector cosine similarity.
    fn compute_coverage(&self, item: &RawDataItem) -> f32 {
        if self.existing_topics.is_empty() {
            // No existing modules = everything is a coverage gap
            return 1.0;
        }

        let content_lower = item.content.to_ascii_lowercase();
        let title_lower = item.title.as_deref().unwrap_or("").to_ascii_lowercase();

        // Check if content overlaps with existing topics
        let overlap_count = self
            .existing_topics
            .iter()
            .filter(|topic| {
                content_lower.contains(topic.as_str()) || title_lower.contains(topic.as_str())
            })
            .count();

        // Low overlap with existing topics = high coverage gap = high score
        let coverage_ratio = overlap_count as f32 / self.existing_topics.len() as f32;
        (1.0 - coverage_ratio).clamp(0.0, 1.0)
    }

    /// Magnitude: keyword density weighted by configured urgency.
    fn compute_magnitude(&self, item: &RawDataItem) -> f32 {
        if self.keyword_weights.is_empty() || item.content.is_empty() {
            return 0.5; // neutral when no keywords configured
        }

        let content_lower = item.content.to_ascii_lowercase();
        let word_count = content_lower.split_whitespace().count().max(1) as f32;

        let mut weighted_hits = 0.0;
        let mut max_possible = 0.0;

        for (keyword, weight) in &self.keyword_weights {
            max_possible += weight;
            let keyword_count = content_lower.matches(keyword.as_str()).count() as f32;
            let density = (keyword_count / word_count).min(0.1); // cap density contribution
            weighted_hits += weight * (density * 10.0).min(1.0);
        }

        if max_possible > 0.0 {
            (weighted_hits / max_possible).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_scorer() -> RelevanceScorer {
        RelevanceScorer::new(
            0.4,
            0.35,
            0.25,
            &[
                KeywordEntry {
                    term: "tariff".to_string(),
                    weight: 2.0,
                },
                KeywordEntry {
                    term: "regulation".to_string(),
                    weight: 1.5,
                },
            ],
        )
    }

    #[test]
    fn empty_content_scores_low() {
        let scorer = test_scorer();
        let item = RawDataItem {
            id: uuid::Uuid::new_v4(),
            source_type: "test".to_string(),
            source_url: None,
            title: None,
            content: "".to_string(),
            collected_at: chrono::Utc::now(),
            metadata: serde_json::json!({}),
        };
        let (total, novelty, _, _) = scorer.score(&item);
        assert_eq!(novelty, 0.0);
        assert!(total < 0.5);
    }

    #[test]
    fn keyword_rich_content_scores_higher() {
        let scorer = test_scorer();
        let plain = RawDataItem {
            id: uuid::Uuid::new_v4(),
            source_type: "test".to_string(),
            source_url: None,
            title: None,
            content: "The weather is nice today and the sky is blue".to_string(),
            collected_at: chrono::Utc::now(),
            metadata: serde_json::json!({}),
        };
        let rich = RawDataItem {
            id: uuid::Uuid::new_v4(),
            source_type: "test".to_string(),
            source_url: None,
            title: None,
            content:
                "New tariff regulation affects energy tariff pricing in the regulation framework"
                    .to_string(),
            collected_at: chrono::Utc::now(),
            metadata: serde_json::json!({}),
        };

        let (_, _, _, magnitude_plain) = scorer.score(&plain);
        let (_, _, _, magnitude_rich) = scorer.score(&rich);
        assert!(magnitude_rich > magnitude_plain);
    }

    #[test]
    fn coverage_gap_scores_high_when_no_existing_topics() {
        let scorer = test_scorer();
        let item = RawDataItem {
            id: uuid::Uuid::new_v4(),
            source_type: "test".to_string(),
            source_url: None,
            title: None,
            content: "Some content about energy markets".to_string(),
            collected_at: chrono::Utc::now(),
            metadata: serde_json::json!({}),
        };
        let (_, _, coverage, _) = scorer.score(&item);
        assert_eq!(coverage, 1.0);
    }

    #[test]
    fn score_components_in_valid_range() {
        let scorer = test_scorer();
        let item = RawDataItem {
            id: uuid::Uuid::new_v4(),
            source_type: "test".to_string(),
            source_url: None,
            title: Some("Test title".to_string()),
            content: "Some test content for scoring".to_string(),
            collected_at: chrono::Utc::now(),
            metadata: serde_json::json!({}),
        };
        let (total, novelty, coverage, magnitude) = scorer.score(&item);
        assert!((0.0..=1.0).contains(&total));
        assert!((0.0..=1.0).contains(&novelty));
        assert!((0.0..=1.0).contains(&coverage));
        assert!((0.0..=1.0).contains(&magnitude));
    }
}
