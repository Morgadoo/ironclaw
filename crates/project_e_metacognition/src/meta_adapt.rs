//! Meta-adaptation engine — reads evaluation history and writes adjusted
//! thresholds to the adaptive TOML overlay.

use std::path::Path;

use serde::{Deserialize, Serialize};

use project_e_core::config::MetaConfig;
use project_e_core::error::{ProjectEError, Result};

use crate::evaluation::CycleEvaluation;

/// Adjusted configuration parameters produced by meta-adaptation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AdaptiveConfig {
    pub min_decision_confidence: Option<f32>,
    pub min_relevance_score: Option<f32>,
    pub max_actions_per_cycle: Option<usize>,
    pub novelty_weight: Option<f32>,
    pub coverage_weight: Option<f32>,
    pub magnitude_weight: Option<f32>,
}

/// Engine that analyzes evaluation history and produces adaptive config changes.
pub struct MetaAdaptationEngine {
    config: MetaConfig,
}

impl MetaAdaptationEngine {
    pub fn new(config: MetaConfig) -> Self {
        Self { config }
    }

    /// Run adaptation analysis on the evaluation history window.
    /// Returns adjusted parameters (only fields that changed are Some).
    pub fn adapt(&self, evaluations: &[CycleEvaluation]) -> AdaptiveConfig {
        if evaluations.len() < self.config.adaptation_frequency_cycles {
            return AdaptiveConfig::default();
        }

        let window: Vec<&CycleEvaluation> = evaluations
            .iter()
            .rev()
            .take(self.config.adaptation_frequency_cycles)
            .collect();

        let avg_efficiency =
            window.iter().map(|e| e.forge_efficiency).sum::<f32>() / window.len() as f32;

        let mut adjusted = AdaptiveConfig::default();

        // If efficiency is declining, tighten confidence threshold
        if avg_efficiency < 0.4 {
            adjusted.min_decision_confidence =
                Some((0.7 + self.config.adjustment_max_step).min(0.95));
        } else if avg_efficiency > 0.7 {
            // If efficiency is high, loosen slightly
            adjusted.min_decision_confidence =
                Some((0.7 - self.config.adjustment_max_step).max(0.5));
        }

        adjusted
    }

    /// Write the adaptive config to the runtime TOML overlay file.
    pub fn write_overlay(&self, path: &Path, config: &AdaptiveConfig) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| ProjectEError::Io {
                reason: format!("failed to create runtime config dir: {e}"),
            })?;
        }
        let toml_str = toml::to_string_pretty(config).map_err(|e| ProjectEError::Config {
            reason: format!("failed to serialize adaptive config: {e}"),
        })?;
        std::fs::write(path, toml_str).map_err(|e| ProjectEError::Io {
            reason: format!("failed to write adaptive config: {e}"),
        })?;
        Ok(())
    }

    /// Check if a failure pattern has repeated enough to warrant a lesson.
    pub fn should_extract_lesson(&self, evaluations: &[CycleEvaluation]) -> bool {
        if evaluations.len() < self.config.lesson_repeat_threshold {
            return false;
        }

        let recent: Vec<&CycleEvaluation> = evaluations
            .iter()
            .rev()
            .take(self.config.lesson_repeat_threshold)
            .collect();

        // Pattern: forge efficiency below 0.3 for N consecutive cycles
        recent.iter().all(|e| e.forge_efficiency < 0.3)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::health::HomeostaticIndicators;
    use chrono::Utc;
    use uuid::Uuid;

    fn make_evaluation(efficiency: f32) -> CycleEvaluation {
        CycleEvaluation {
            cycle_id: Uuid::new_v4(),
            modules_published: vec![],
            modules_rejected: 0,
            forge_efficiency: efficiency,
            coverage_delta: 0.0,
            skill_effectiveness: vec![],
            health_indicators: HomeostaticIndicators {
                coverage: 0.5,
                stability: 0.5,
                efficiency,
                utilization: 0.5,
                freshness: 0.5,
            },
            homeostatic_actions: vec![],
            evaluated_at: Utc::now(),
        }
    }

    #[test]
    fn adapt_with_insufficient_history() {
        let engine = MetaAdaptationEngine::new(MetaConfig::default());
        let evals = vec![make_evaluation(0.5)]; // only 1, need 5
        let result = engine.adapt(&evals);
        assert!(result.min_decision_confidence.is_none());
    }

    #[test]
    fn adapt_tightens_on_low_efficiency() {
        let engine = MetaAdaptationEngine::new(MetaConfig {
            adaptation_frequency_cycles: 3,
            ..MetaConfig::default()
        });
        let evals: Vec<CycleEvaluation> = (0..3).map(|_| make_evaluation(0.2)).collect();
        let result = engine.adapt(&evals);
        assert!(result.min_decision_confidence.is_some());
        assert!(result.min_decision_confidence.unwrap() > 0.7);
    }

    #[test]
    fn lesson_extraction_threshold() {
        let engine = MetaAdaptationEngine::new(MetaConfig {
            lesson_repeat_threshold: 3,
            ..MetaConfig::default()
        });

        let low_evals: Vec<CycleEvaluation> = (0..3).map(|_| make_evaluation(0.1)).collect();
        assert!(engine.should_extract_lesson(&low_evals));

        let mixed_evals: Vec<CycleEvaluation> = vec![
            make_evaluation(0.1),
            make_evaluation(0.8),
            make_evaluation(0.1),
        ];
        assert!(!engine.should_extract_lesson(&mixed_evals));
    }
}
