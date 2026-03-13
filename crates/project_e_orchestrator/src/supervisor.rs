//! Supervisor agent — monitors for anomaly patterns and applies interventions.

use std::collections::HashMap;

use project_e_core::types::{AnomalyPattern, InterventionType};

/// Tracks anomaly occurrences and applies interventions.
pub struct Supervisor {
    anomaly_counters: HashMap<AnomalyPattern, u32>,
    intervention_threshold: u32,
}

impl Supervisor {
    pub fn new(intervention_threshold: u32) -> Self {
        Self {
            anomaly_counters: HashMap::new(),
            intervention_threshold,
        }
    }

    /// Record an anomaly occurrence. Returns an intervention if the
    /// threshold has been reached.
    pub fn record_anomaly(&mut self, pattern: AnomalyPattern) -> Option<InterventionType> {
        let count = self.anomaly_counters.entry(pattern).or_insert(0);
        *count += 1;

        if *count >= self.intervention_threshold {
            *count = 0; // reset counter before borrowing self immutably
            let intervention = self.choose_intervention(pattern);
            Some(intervention)
        } else {
            None
        }
    }

    /// Choose an intervention based on the anomaly pattern.
    fn choose_intervention(&self, pattern: AnomalyPattern) -> InterventionType {
        match pattern {
            AnomalyPattern::RepeatedCompilationErrors => InterventionType::PromptInjection {
                guidance: "Focus on simpler module structure".to_string(),
            },
            AnomalyPattern::EfficiencyCollapse => InterventionType::ModelEscalation {
                to_model: "model_for_retry".to_string(),
            },
            AnomalyPattern::RecoveryLoop => {
                InterventionType::TightenConfidenceThreshold { delta: 0.05 }
            }
            AnomalyPattern::ErrorStorm => InterventionType::PauseCycle { duration_secs: 300 },
            AnomalyPattern::StaleData => InterventionType::ResetCounters,
            AnomalyPattern::RedundantModules => InterventionType::ForceRecoveryMode,
            AnomalyPattern::ContextOverflow => InterventionType::SkipPhase {
                phase: project_e_core::types::CognitivePhase::MetaAdaptation,
            },
        }
    }

    /// Get current anomaly counts.
    pub fn anomaly_counts(&self) -> &HashMap<AnomalyPattern, u32> {
        &self.anomaly_counters
    }

    /// Reset all counters.
    pub fn reset(&mut self) {
        self.anomaly_counters.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intervention_after_threshold() {
        let mut supervisor = Supervisor::new(3);

        assert!(
            supervisor
                .record_anomaly(AnomalyPattern::RepeatedCompilationErrors)
                .is_none()
        );
        assert!(
            supervisor
                .record_anomaly(AnomalyPattern::RepeatedCompilationErrors)
                .is_none()
        );

        let intervention = supervisor.record_anomaly(AnomalyPattern::RepeatedCompilationErrors);
        assert!(intervention.is_some());
        assert!(matches!(
            intervention.unwrap(),
            InterventionType::PromptInjection { .. }
        ));
    }

    #[test]
    fn counter_resets_after_intervention() {
        let mut supervisor = Supervisor::new(2);

        supervisor.record_anomaly(AnomalyPattern::ErrorStorm);
        let intervention = supervisor.record_anomaly(AnomalyPattern::ErrorStorm);
        assert!(intervention.is_some());

        // Counter should be reset
        assert_eq!(
            *supervisor
                .anomaly_counts()
                .get(&AnomalyPattern::ErrorStorm)
                .unwrap_or(&0),
            0
        );
    }

    #[test]
    fn independent_pattern_tracking() {
        let mut supervisor = Supervisor::new(3);

        supervisor.record_anomaly(AnomalyPattern::ErrorStorm);
        supervisor.record_anomaly(AnomalyPattern::RepeatedCompilationErrors);
        supervisor.record_anomaly(AnomalyPattern::ErrorStorm);

        assert_eq!(supervisor.anomaly_counts()[&AnomalyPattern::ErrorStorm], 2);
        assert_eq!(
            supervisor.anomaly_counts()[&AnomalyPattern::RepeatedCompilationErrors],
            1
        );
    }
}
