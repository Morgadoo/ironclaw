//! SelfModelBuilder — computes the five health dimensions that drive
//! deliberation engine operating mode selection.

use project_e_core::types::{CycleMetrics, OperatingMode};

/// System self-assessment built at the start of each Deliberation phase.
#[derive(Debug, Clone)]
pub struct SystemSelfModel {
    /// Fraction of identified gaps with active modules.
    pub coverage_score: f32,
    /// Module success rate over sliding window.
    pub stability_score: f32,
    /// Forge success rate: successful / total attempts.
    pub efficiency_score: f32,
    /// Fraction of active modules receiving events.
    pub utilization_score: f32,
    /// Recency of data feeding active modules (0=stale, 1=fresh).
    pub freshness_signal: f32,
    /// Current operating mode derived from scores.
    pub operating_mode: OperatingMode,
    /// Last N cycle metrics for trend analysis.
    pub reflection_window: Vec<CycleMetrics>,
}

/// Builds a `SystemSelfModel` from database metrics and cycle history.
pub struct SelfModelBuilder {
    recovery_threshold: f32,
    cautious_threshold: f32,
}

impl SelfModelBuilder {
    pub fn new(recovery_threshold: f32, cautious_threshold: f32) -> Self {
        Self {
            recovery_threshold,
            cautious_threshold,
        }
    }

    /// Build a self-model from the provided health scores and history.
    pub fn build(
        &self,
        coverage: f32,
        stability: f32,
        efficiency: f32,
        utilization: f32,
        freshness: f32,
        history: Vec<CycleMetrics>,
    ) -> SystemSelfModel {
        let operating_mode = self.determine_mode(efficiency, stability);

        SystemSelfModel {
            coverage_score: coverage,
            stability_score: stability,
            efficiency_score: efficiency,
            utilization_score: utilization,
            freshness_signal: freshness,
            operating_mode,
            reflection_window: history,
        }
    }

    fn determine_mode(&self, efficiency: f32, stability: f32) -> OperatingMode {
        let composite = (efficiency + stability) / 2.0;
        if composite < self.recovery_threshold {
            OperatingMode::Recovery
        } else if composite < self.cautious_threshold {
            OperatingMode::Cautious
        } else {
            OperatingMode::Nominal
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_transitions() {
        let builder = SelfModelBuilder::new(0.3, 0.6);

        let model = builder.build(0.5, 0.1, 0.2, 0.3, 0.5, vec![]);
        assert_eq!(model.operating_mode, OperatingMode::Recovery);

        let model = builder.build(0.5, 0.5, 0.5, 0.3, 0.5, vec![]);
        assert_eq!(model.operating_mode, OperatingMode::Cautious);

        let model = builder.build(0.8, 0.9, 0.8, 0.7, 0.9, vec![]);
        assert_eq!(model.operating_mode, OperatingMode::Nominal);
    }
}
