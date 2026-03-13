//! Six validation gates for the forge pipeline.

pub mod audit;
pub mod clippy;
pub mod compile;
pub mod forbidden;
pub mod test;
pub mod unsafe_check;

use project_e_core::error::ProjectEError;
use project_e_core::types::GateName;

/// Error type for gate validation failures.
#[derive(Debug, Clone)]
pub struct GateError {
    pub gate: GateName,
    pub message: String,
    pub details: Option<String>,
}

impl GateError {
    pub fn new(gate: GateName, message: impl Into<String>) -> Self {
        Self {
            gate,
            message: message.into(),
            details: None,
        }
    }

    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }
}

impl From<GateError> for ProjectEError {
    fn from(e: GateError) -> Self {
        ProjectEError::Forge {
            reason: format!("gate {:?} failed: {}", e.gate, e.message),
        }
    }
}

/// Result of running a single gate.
#[derive(Debug)]
pub enum GateResult {
    /// Gate passed.
    Pass,
    /// Gate failed with an error.
    Fail(GateError),
}

impl GateResult {
    pub fn is_pass(&self) -> bool {
        matches!(self, Self::Pass)
    }
}
