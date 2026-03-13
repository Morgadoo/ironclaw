//! Module lifecycle state machine enforcement.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use project_e_core::types::ModuleState;

/// A recorded state transition for audit trail.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateTransition {
    pub id: Uuid,
    pub module_id: Uuid,
    pub from_state: ModuleState,
    pub to_state: ModuleState,
    pub reason: String,
    pub transitioned_at: DateTime<Utc>,
}

impl StateTransition {
    pub fn new(
        module_id: Uuid,
        from_state: ModuleState,
        to_state: ModuleState,
        reason: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            module_id,
            from_state,
            to_state,
            reason,
            transitioned_at: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transition_record() {
        let t = StateTransition::new(
            Uuid::new_v4(),
            ModuleState::Testing,
            ModuleState::Active,
            "all tests passed".to_string(),
        );
        assert_eq!(t.from_state, ModuleState::Testing);
        assert_eq!(t.to_state, ModuleState::Active);
    }
}
