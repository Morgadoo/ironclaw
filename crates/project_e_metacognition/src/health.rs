//! Five homeostatic health indicators.

use serde::{Deserialize, Serialize};

use project_e_core::config::HealthConfig;

/// Homeostatic health indicators computed from database metrics.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct HomeostaticIndicators {
    /// active_modules / identified_needs (from gap_reports).
    pub coverage: f32,
    /// modules_no_regression / total_active (test_results).
    pub stability: f32,
    /// successful_generations / total_attempts (generation_history).
    pub efficiency: f32,
    /// modules_with_events / total_active.
    pub utilization: f32,
    /// 1 - avg_data_age_days / max_acceptable_age.
    pub freshness: f32,
}

impl HomeostaticIndicators {
    /// Check if all indicators meet their configured thresholds.
    pub fn all_above_thresholds(&self, config: &HealthConfig) -> bool {
        self.coverage >= config.min_coverage
            && self.stability >= config.min_stability
            && self.efficiency >= config.min_efficiency
            && self.utilization >= config.min_utilization
            && self.freshness >= config.min_freshness
    }

    /// Returns a list of indicators that are below their thresholds.
    pub fn violations(&self, config: &HealthConfig) -> Vec<(&'static str, f32, f32)> {
        let mut violations = Vec::new();
        if self.coverage < config.min_coverage {
            violations.push(("coverage", self.coverage, config.min_coverage));
        }
        if self.stability < config.min_stability {
            violations.push(("stability", self.stability, config.min_stability));
        }
        if self.efficiency < config.min_efficiency {
            violations.push(("efficiency", self.efficiency, config.min_efficiency));
        }
        if self.utilization < config.min_utilization {
            violations.push(("utilization", self.utilization, config.min_utilization));
        }
        if self.freshness < config.min_freshness {
            violations.push(("freshness", self.freshness, config.min_freshness));
        }
        violations
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn healthy_indicators() -> HomeostaticIndicators {
        HomeostaticIndicators {
            coverage: 0.8,
            stability: 0.9,
            efficiency: 0.7,
            utilization: 0.6,
            freshness: 0.8,
        }
    }

    #[test]
    fn healthy_system_passes_thresholds() {
        let indicators = healthy_indicators();
        let config = HealthConfig::default();
        assert!(indicators.all_above_thresholds(&config));
        assert!(indicators.violations(&config).is_empty());
    }

    #[test]
    fn unhealthy_system_reports_violations() {
        let indicators = HomeostaticIndicators {
            coverage: 0.2,
            stability: 0.3,
            efficiency: 0.1,
            utilization: 0.1,
            freshness: 0.2,
        };
        let config = HealthConfig::default();
        assert!(!indicators.all_above_thresholds(&config));
        let violations = indicators.violations(&config);
        assert_eq!(violations.len(), 5);
    }
}
