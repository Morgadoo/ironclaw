//! Retry logic with best-candidate tracking across forge attempts.

use project_e_core::types::GateName;

use crate::gates::GateError;

/// Tracks state across retry attempts for a single module forge.
#[derive(Debug)]
pub struct RetryState {
    pub attempt: u8,
    pub max_attempts: u8,
    /// Most complete code seen so far (passed more gates).
    pub best_candidate: Option<String>,
    /// Highest gate reached by the best candidate.
    best_gate: Option<GateName>,
    /// Error history fed back to LLM on retry.
    pub error_history: Vec<GateError>,
    /// True when switched to escalated model.
    pub escalated: bool,
}

impl RetryState {
    pub fn new(max_attempts: u8) -> Self {
        Self {
            attempt: 0,
            max_attempts,
            best_candidate: None,
            best_gate: None,
            error_history: Vec::new(),
            escalated: false,
        }
    }

    /// Record an attempt. Updates best candidate if this code reached
    /// a later gate than the previous best.
    pub fn record_attempt(&mut self, code: &str, gate_reached: GateName, error: GateError) {
        self.attempt += 1;

        let current_progress = gate_reached.ordinal();
        let best_progress = self.best_gate.map(|g| g.ordinal()).unwrap_or(0);

        if current_progress >= best_progress {
            self.best_candidate = Some(code.to_string());
            self.best_gate = Some(gate_reached);
        }

        self.error_history.push(error);
    }

    /// Record a fully successful attempt.
    pub fn record_success(&mut self, code: &str) {
        self.attempt += 1;
        self.best_candidate = Some(code.to_string());
        self.best_gate = Some(GateName::Test);
    }

    /// Whether more retry attempts are available.
    pub fn can_retry(&self) -> bool {
        self.attempt < self.max_attempts
    }

    /// Whether the model should be escalated (after 2 failures).
    pub fn should_escalate(&self) -> bool {
        self.attempt >= 2 && !self.escalated
    }

    /// Mark as escalated.
    pub fn mark_escalated(&mut self) {
        self.escalated = true;
    }

    /// Return the best candidate for the rejection report.
    pub fn best_for_report(&self) -> Option<&str> {
        self.best_candidate.as_deref()
    }

    /// Return the highest gate reached.
    pub fn best_gate(&self) -> Option<GateName> {
        self.best_gate
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_state_tracks_best() {
        let mut state = RetryState::new(5);
        assert!(state.can_retry());
        assert!(state.best_candidate.is_none());

        state.record_attempt(
            "code v1",
            GateName::UnsafeCheck,
            GateError::new(GateName::ForbiddenPatterns, "found unwrap"),
        );
        assert_eq!(state.best_gate(), Some(GateName::UnsafeCheck));

        state.record_attempt(
            "code v2",
            GateName::Compile,
            GateError::new(GateName::Clippy, "warnings"),
        );
        assert_eq!(state.best_gate(), Some(GateName::Compile));
        assert_eq!(state.best_for_report(), Some("code v2"));

        // Worse attempt doesn't replace best
        state.record_attempt(
            "code v3",
            GateName::UnsafeCheck,
            GateError::new(GateName::ForbiddenPatterns, "unwrap again"),
        );
        assert_eq!(state.best_for_report(), Some("code v2"));
    }

    #[test]
    fn escalation_after_two_failures() {
        let mut state = RetryState::new(5);
        assert!(!state.should_escalate());

        state.record_attempt(
            "v1",
            GateName::Compile,
            GateError::new(GateName::Compile, "error"),
        );
        assert!(!state.should_escalate());

        state.record_attempt(
            "v2",
            GateName::Compile,
            GateError::new(GateName::Compile, "error"),
        );
        assert!(state.should_escalate());

        state.mark_escalated();
        assert!(!state.should_escalate());
    }

    #[test]
    fn can_retry_respects_max() {
        let mut state = RetryState::new(2);
        assert!(state.can_retry());

        state.record_attempt(
            "v1",
            GateName::Compile,
            GateError::new(GateName::Compile, "err"),
        );
        assert!(state.can_retry());

        state.record_attempt(
            "v2",
            GateName::Compile,
            GateError::new(GateName::Compile, "err"),
        );
        assert!(!state.can_retry());
    }
}
