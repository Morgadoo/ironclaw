//! CycleOrchestrator — executes the six-phase cognitive cycle using
//! IronClaw's job scheduler pattern.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use project_e_core::types::CognitivePhase;

/// Result of a single cognitive cycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CycleResult {
    pub cycle_id: Uuid,
    pub phases_completed: Vec<CognitivePhase>,
    pub duration_secs: u64,
    pub modules_published: usize,
    pub modules_rejected: usize,
    pub gaps_detected: usize,
    pub completed_at: chrono::DateTime<Utc>,
}

/// Status of an in-progress cycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CycleStatus {
    pub cycle_id: Uuid,
    pub current_phase: CognitivePhase,
    pub started_at: chrono::DateTime<Utc>,
    pub elapsed_secs: u64,
}

impl CycleResult {
    /// Create a new cycle result.
    pub fn new(cycle_id: Uuid, duration_secs: u64) -> Self {
        Self {
            cycle_id,
            phases_completed: vec![],
            duration_secs,
            modules_published: 0,
            modules_rejected: 0,
            gaps_detected: 0,
            completed_at: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycle_result_creation() {
        let result = CycleResult::new(Uuid::new_v4(), 120);
        assert_eq!(result.duration_secs, 120);
        assert!(result.phases_completed.is_empty());
    }
}
