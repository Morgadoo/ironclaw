//! Evaluation engine — post-cycle assessment with homeostatic regulation.
//!
//! Adapts IronClaw's self-repair pattern into a full Evaluation Engine.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::health::HomeostaticIndicators;

/// Result of evaluating a complete cognitive cycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CycleEvaluation {
    pub cycle_id: Uuid,
    pub modules_published: Vec<Uuid>,
    pub modules_rejected: usize,
    pub forge_efficiency: f32,
    pub coverage_delta: f32,
    pub skill_effectiveness: Vec<SkillScore>,
    pub health_indicators: HomeostaticIndicators,
    pub homeostatic_actions: Vec<HomeostaticAction>,
    pub evaluated_at: DateTime<Utc>,
}

/// Per-skill effectiveness score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillScore {
    pub skill_name: String,
    pub success_rate: f32,
    pub activations: usize,
}

/// Actions taken by the homeostatic regulation system.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum HomeostaticAction {
    PrunedStaleData {
        count: usize,
    },
    ArchivedGapReports {
        count: usize,
    },
    DetectedRedundancy {
        module_a: Uuid,
        module_b: Uuid,
        similarity: f32,
    },
    AutoDeprecated {
        module_id: Uuid,
        failure_rate: f32,
    },
    RemovedDeprecated {
        module_id: Uuid,
        deprecated_days: u32,
    },
}

/// Pair of modules detected as redundant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedundancyPair {
    pub module_a: Uuid,
    pub module_b: Uuid,
    pub similarity: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycle_evaluation_serialization() {
        let eval = CycleEvaluation {
            cycle_id: Uuid::new_v4(),
            modules_published: vec![Uuid::new_v4()],
            modules_rejected: 2,
            forge_efficiency: 0.33,
            coverage_delta: 0.05,
            skill_effectiveness: vec![SkillScore {
                skill_name: "rust-module-generation".to_string(),
                success_rate: 0.75,
                activations: 4,
            }],
            health_indicators: HomeostaticIndicators {
                coverage: 0.7,
                stability: 0.8,
                efficiency: 0.5,
                utilization: 0.6,
                freshness: 0.9,
            },
            homeostatic_actions: vec![HomeostaticAction::PrunedStaleData { count: 50 }],
            evaluated_at: Utc::now(),
        };

        let json = serde_json::to_string(&eval).unwrap();
        let parsed: CycleEvaluation = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.cycle_id, eval.cycle_id);
        assert_eq!(parsed.forge_efficiency, 0.33);
    }
}
